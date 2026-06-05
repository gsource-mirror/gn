// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use starlark::collections::SmallMap;
use starlark::environment::GlobalsBuilder;
use starlark::eval::{ParametersSpec, ParametersSpecParam};
use starlark::values::{Heap, Value, ValueLike};
use starlark_derive::starlark_module;

use crate::attr::AttrSchema;
use crate::label::{Label, Package};
use crate::Error;
use super::rule::RuleCallableGen;

fn common_rule<'v>(
    implementation: Value<'v>,
    attrs: Option<SmallMap<&'v str, &'v AttrSchema>>,
    heap: Heap<'v>,
    is_extension: bool,
) -> starlark::Result<Value<'v>> {
    let attrs = attrs.unwrap_or_default();
    if attrs.contains_key("name") {
        return Err(Error::NameAttrForbidden.into());
    }
    let mut named_only = vec![];
    named_only.push(("name", ParametersSpecParam::Optional));
    for (name, attr) in attrs.iter() {
        let spec_val = match &attr.default {
            None => ParametersSpecParam::Required,
            Some(_) => ParametersSpecParam::Optional,
        };
        named_only.push((name, spec_val));
    }

    let signature = ParametersSpec::new_parts(
        "rule",
        [("target", ParametersSpecParam::Optional)],
        std::iter::empty(),
        false,
        named_only,
        false,
    );

    Ok(heap.alloc(RuleCallableGen {
        implementation,
        attrs: attrs.iter().map(|(k, &v)| (k.to_string(), v.clone())).collect(),
        is_extension,
        signature,
    }))
}

#[starlark_module]
pub fn register_rule(builder: &mut GlobalsBuilder) {
    fn rule<'v>(
        implementation: Value<'v>,
        attrs: Option<SmallMap<&'v str, &'v AttrSchema>>,
        heap: Heap<'v>,
    ) -> starlark::Result<Value<'v>> {
        common_rule(implementation, attrs, heap, false)
    }

    fn rule_extension<'v>(
        implementation: Value<'v>,
        attrs: Option<SmallMap<&'v str, &'v AttrSchema>>,
        heap: Heap<'v>,
    ) -> starlark::Result<Value<'v>> {
        common_rule(implementation, attrs, heap, true)
    }
}


#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub unsafe extern "C" fn get_custom_ninja(
    target: &crate::ffi::Target,
    session: &crate::ffi::bindings::rust::StarlarkSession,
) -> &'static str {
    use crate::ffi::AsRust;
    let rust_session = session.as_rust();

    let starlark_target = crate::ffi::GetTargetStarlarkTarget(target as *const crate::ffi::Target);
    if starlark_target.is_null() {
        return "";
    }

    let target_label = unsafe { &*crate::ffi::GetTargetLabel(target) };
    let label = crate::LabelRef::from(target_label);

    let toolchain = crate::LabelRef {
        package: target_label.toolchain_dir().into(),
        name: unsafe { crate::util::from_utf8_unchecked(target_label.toolchain_name()) },
    };

    let target_ref = rust_session.get_target_by_label(label, toolchain);
    if let Some(s) = target_ref.get_custom_ninja_ref() {
        let static_ref: &'static str = unsafe { crate::util::extend_lifetime(s.as_str()) };
        static_ref
    } else {
        ""
    }
}

pub fn collect_files_from_attr<'v>(
    val: Value<'v>,
    rust_session: &crate::session::StarlarkSession,
    current_toolchain: &Label,
    _target: *mut crate::ffi::Target,
    _paths: std::sync::Arc<std::sync::Mutex<Vec<std::path::PathBuf>>>,
    caller_pkg_str: &str,
    heap: Heap<'v>,
) -> Result<Vec<Value<'v>>, starlark::Error> {
    let mut files = Vec::new();

    let process_item = |item: Value<'v>, files: &mut Vec<Value<'v>>| -> Result<(), starlark::Error> {
        if let Some(s) = item.unpack_str() {
            if s.starts_with("//") || s.starts_with(':') {
                let caller_pkg = Package(caller_pkg_str.to_string());
                let lbl = Label::parse(s, caller_pkg.as_ref()).map_err(|e| starlark::Error::new_other(e))?;
                let resolved_ref = rust_session.get_target_by_label(lbl.as_ref(), current_toolchain.as_ref());
                let resolved = heap.alloc(resolved_ref);
                collect_files_from_resolved_target(resolved, files, heap)?;
            } else {
                let file_path = if s.starts_with("//") {
                     std::path::PathBuf::from(s)
                } else {
                     let mut path = std::path::PathBuf::from(caller_pkg_str);
                     path.push(s);
                     path
                };
                let static_path: &'static std::path::Path = Box::leak(file_path.into_boxed_path());
                let f = crate::file::File(static_path);
                files.push(heap.alloc(f));
            }
        } else if let Some(lbl) = item.downcast_ref::<Label>() {
            let resolved_ref = rust_session.get_target_by_label(lbl.as_ref(), current_toolchain.as_ref());
            let resolved = heap.alloc(resolved_ref);
            collect_files_from_resolved_target(resolved, files, heap)?;
        } else if item.downcast_ref::<crate::file::File>().is_some() {
            files.push(item);
        } else {
            collect_files_from_resolved_target(item, files, heap)?;
        }
        Ok(())
    };

    if let Some(list) = starlark::values::list::ListRef::from_value(val) {
        for item in list.iter() {
            let item: Value<'v> = unsafe { std::mem::transmute(item) };
            process_item(item, &mut files)?;
        }
    } else {
        process_item(val, &mut files)?;
    }

    Ok(files)
}

pub fn collect_files_from_resolved_target<'v>(
    resolved: Value<'v>,
    files: &mut Vec<Value<'v>>,
    heap: Heap<'v>,
) -> Result<(), starlark::Error> {
    if !resolved.is_none() {
        if let Ok(Some(default_info)) = resolved.get_attr("DefaultInfo", heap) {
            if !default_info.is_none() {
                if let Ok(Some(files_depset)) = default_info.get_attr("files", heap) {
                    if !files_depset.is_none() {
                        if let Some(depset) = files_depset.downcast_ref::<crate::depset::Depset>() {
                            depset.for_each_fallible(|v| {
                                files.push(v);
                                Ok(())
                            })?;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
