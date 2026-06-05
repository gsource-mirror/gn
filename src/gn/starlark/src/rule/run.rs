// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use starlark::environment::Module;
use starlark::eval::Evaluator;
use starlark::values::{Value, ValueLike, UnpackValue};
use starlark::values::list::UnpackList;
use crate::provider::UnpackProvider;

fn escape_for_ninja(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        if c == '$' || c == ' ' || c == ':' {
            result.push('$');
        }
        result.push(c);
    }
    result
}


#[no_mangle]
pub unsafe extern "C" fn run_target_rule_implementation(
    starlark_target: *mut crate::ffi::bindings::rust::RustTarget,
    scope: *mut crate::ffi::Scope,
    session: &crate::ffi::bindings::rust::StarlarkSession,
    err: std::pin::Pin<&mut crate::ffi::Err>,
) -> bool {
    let res = (|| -> Result<bool, String> {
        use crate::ffi::AsRust;
        let rust_session = session.as_rust();
        let rust_target = starlark_target.as_rust().unwrap();
        let target = rust_target.ptr();

        let rule_frozen = rust_target.rule().unwrap();
        let attrs_map = rust_target.get_unevaluated_attrs();

        let frozen_rule = &*rule_frozen;

        let target_label = &*crate::ffi::GetTargetLabel(unsafe { &*target });
        let toolchain_dir = target_label.toolchain_dir();
        let toolchain_name_cxx = target_label.toolchain_name();
        let toolchain_name_str = toolchain_name_cxx.to_str().unwrap();

        let current_toolchain = rust_target.toolchain();

        let run_res = Module::with_temp_heap(|module| -> Result<bool, String> {
            let extra = crate::session::EvalContext::new(
                scope,
                std::ptr::null(),
                crate::session::EvalKind::RuleEval,
            );
            let mut eval = Evaluator::new(&module);
            eval.extra = Some(&extra);
            let heap = module.heap();

            let mut resolved_attrs = starlark::collections::SmallMap::new();
            let mut resolved_file_attrs = starlark::collections::SmallMap::new();
            let mut resolved_files_attrs = starlark::collections::SmallMap::new();

            for (&name, attr) in attrs_map {
                let name: &str = name;
                let schema = frozen_rule.attrs.get(name).ok_or_else(|| {
                    format!("Attribute '{}' not found in rule schema", name)
                })?;

                let attr_val = attr.to_value(schema, rust_session, &current_toolchain, target, &heap)
                    .map_err(|e| format!("{}", e))?;

                resolved_attrs.insert(name.to_string(), attr_val.attr);
                if let Some(f) = attr_val.file {
                    resolved_file_attrs.insert(name.to_string(), f);
                }
                if let Some(fs) = attr_val.files {
                    resolved_files_attrs.insert(name.to_string(), fs);
                }
            }

            let attr = heap.alloc(crate::ctx::attr::CtxAttrGen::new(resolved_attrs));

            let ctx_file = crate::ctx::files::CtxFileGen {
                resolved_files: resolved_file_attrs,
            };
            let file = heap.alloc(ctx_file);

            let ctx_files = crate::ctx::files::CtxFilesGen {
                target: target as usize,
                resolved_files: resolved_files_attrs,
            };
            let files = heap.alloc(ctx_files);

            use crate::ffi::ToRust;
            let target_output_dir = unsafe { crate::ffi::GetTargetOutputDir(&*target) }
                .as_ref()
                .unwrap()
                .to_rust();
            let actions_gen = crate::ctx::actions::ActionsGen::new(target_output_dir);
            let actions = heap.alloc(actions_gen);

            let label = rust_target.label();
            let toolchain = rust_target.toolchain();
            let target_ref = rust_session.get_target_by_label(label, toolchain);
            let target_val = heap.alloc(target_ref);

            let gn_gen = crate::ctx::gn::GnGen::new(target_ref);
            let gn = heap.alloc(gn_gen);
            let ctx = heap.alloc(crate::ctx::CtxGen::new(actions, attr, file, files, target_val, gn));

            let impl_val = frozen_rule.implementation.to_value();
            let providers_val = eval.eval_function(impl_val, &[ctx], &[])
                .map_err(|e| format!("{}", e))?;

            let providers = if providers_val.is_none() {
                UnpackList { items: Vec::new() }
            } else {
                UnpackList::<UnpackProvider>::unpack_value(providers_val)
                    .map_err(|e| format!("{}", e))?
                    .ok_or_else(|| "Expected list of providers".to_owned())?
            };




            // Process GnSubstitutionInfo provider
            let builtins = crate::globals::make_builtins();
            let gn_sub_info_type = builtins.gn_substitution_info.to_value();
            let gn_sub_info_type_id = if let Some(rt) = gn_sub_info_type.downcast_ref::<starlark::values::record::RecordType>() {
                Some(rt.type_id())
            } else if let Some(rt) = gn_sub_info_type.downcast_ref::<starlark::values::record::FrozenRecordType>() {
                Some(rt.type_id())
            } else {
                None
            };

            if let Some(type_id) = gn_sub_info_type_id {
                if let Some(provider) = providers.items.iter().find(|p| p.0 == type_id) {
                    let provider_inst = provider.1;
                    if let Some(subs) = provider_inst.get_attr("substitutions", heap).map_err(|e| format!("{}", e))? {
                        let struct_ref = starlark::values::structs::StructRef::from_value(subs).ok_or_else(|| {
                            starlark::Error::from(crate::errors::Error::GenericError(format!(
                                "substitutions must be a struct, got type: {}",
                                subs.get_type()
                            )))
                        }).map_err(|e| format!("{}", e))?;

                        let mut custom_ninja_out = String::new();
                        let is_action = unsafe { crate::ffi::IsActionTarget(&*target) };
                        let indent_str = if is_action { "  " } else { "" };

                        for (name, val) in struct_ref.iter() {
                            let mut args_list = Vec::new();
                            if let Some(list) = starlark::values::list::ListRef::from_value(val) {
                                for item in list.iter() {
                                    args_list.push(item);
                                }
                            } else {
                                args_list.push(val);
                            }

                            let mut expanded_values = Vec::new();
                            for arg_val in args_list {
                                if let Some(args_obj) = arg_val.downcast_ref::<crate::ctx::args::ArgsGen<starlark::values::Value>>() {
                                    if let Ok((expanded_strings, _)) = args_obj.expand(&mut eval) {
                                        expanded_values.extend(expanded_strings);
                                    }
                                } else if let Some(args_obj) = arg_val.downcast_ref::<crate::ctx::args::ArgsGen<starlark::values::FrozenValue>>() {
                                    if let Ok((expanded_strings, _)) = args_obj.expand(&mut eval) {
                                        expanded_values.extend(expanded_strings);
                                    }
                                } else if let Some(s) = arg_val.unpack_str() {
                                    expanded_values.push(s.to_owned());
                                }
                            }

                            custom_ninja_out.push_str(indent_str);
                            custom_ninja_out.push_str(name.as_str());
                            custom_ninja_out.push_str(" =");
                            for val in expanded_values {
                                custom_ninja_out.push(' ');
                                custom_ninja_out.push_str(&escape_for_ninja(&val));
                            }
                            custom_ninja_out.push('\n');
                        }
                        rust_target.set_custom_ninja(custom_ninja_out);
                    }
                }
            }

            #[allow(invalid_reference_casting)]
            unsafe {
                let rust_target_mut = &mut *(rust_target as *const crate::target::Target as *mut crate::target::Target);
                rust_target_mut.set_providers(providers_val).map_err(|e| format!("{:?}", e))?;

                let actions_ref = actions.downcast_ref::<crate::ctx::actions::ActionsGen<Value<'_>>>().unwrap();
                let mut declared_ref = actions_ref.declared_outputs.borrow_mut();
                let mut temp = starlark::collections::SmallMap::new();
                std::mem::swap(&mut *declared_ref, &mut temp);
                for (_, path) in temp {
                    rust_target_mut.add_declared_output(path);
                }
            }
            Ok(true)
        });

        run_res
    })();

    crate::ffi::handle_result(err, std::ptr::null(), res)
}
