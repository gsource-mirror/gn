/*
 * Copyright 2026 The GN Authors. All rights reserved.
 * Use of this source code is governed by a BSD-style license that can be
 * found in the LICENSE file.
 */

use starlark::environment::{Globals, GlobalsBuilder};
use crate::Error;
use starlark_derive::starlark_module;
use std::sync::OnceLock;

use starlark::values::FrozenHeapName;




use crate::provider::TypeId;

pub struct BuiltinModule {
    pub default_info: starlark::values::FrozenValue,
    pub gn_substitution_info: starlark::values::FrozenValue,
    pub default_info_type: TypeId,
    pub empty_default_info: starlark::values::FrozenValue,
}

static GLOBALS_AND_BUILTINS: OnceLock<(Globals, BuiltinModule)> = OnceLock::new();

fn get_globals_and_builtins() -> &'static (Globals, BuiltinModule) {
    GLOBALS_AND_BUILTINS.get_or_init(|| {
        let mut builder = GlobalsBuilder::extended_by(&[
            starlark::environment::LibraryExtension::Debug,
            starlark::environment::LibraryExtension::Print, // Adds print()
            starlark::environment::LibraryExtension::Pprint, // Adds pprint()
            starlark::environment::LibraryExtension::RecordType,
            starlark::environment::LibraryExtension::SetType,
            starlark::environment::LibraryExtension::StructType,
        ]);
        crate::depset::register_depset(&mut builder);
        crate::attr::register_attr(&mut builder);
        crate::rule::register_rule(&mut builder);
        crate::provider::register_provider(&mut builder);
        #[cfg(test)]
        crate::ctx::args::register_args_test_globals(&mut builder);
        register_gn_rules(&mut builder);
        
        let builtins = load_builtins(&mut builder);
        
        (builder.build(), builtins)
    })
}

pub fn make_globals() -> &'static Globals {
    &get_globals_and_builtins().0
}

pub fn make_builtins() -> &'static BuiltinModule {
    &get_globals_and_builtins().1
}

fn load_builtins(builder: &mut GlobalsBuilder) -> BuiltinModule {
    let builtins_src = include_str!("builtins.bzl");
    let ast = starlark::syntax::AstModule::parse(
        "builtins.bzl",
        builtins_src.to_owned(),
        &starlark::syntax::Dialect::Extended,
    )
    .unwrap();

    let mut temp_builder = GlobalsBuilder::extended_by(&[
        starlark::environment::LibraryExtension::RecordType,
    ]);
    crate::depset::register_depset(&mut temp_builder);
    crate::provider::register_provider(&mut temp_builder);
    let temp_globals = temp_builder.build();

    let module = starlark::environment::Module::with_temp_heap(|module| {
        {
            let mut eval = starlark::eval::Evaluator::new(&module);
            eval.eval_module(ast, &temp_globals)?;
        }
        module
            .freeze_named(FrozenHeapName::User(Box::new(
                "builtins.bzl".to_owned(),
            )))
            .map_err(|e| starlark::Error::new_other(e))
    })
    .unwrap();


    let mut get = |name: &str| {
        let val = module.get(name).unwrap();
        let frozen = unsafe { val.owned_frozen_value(builder.frozen_heap()) };
        builder.set(name, frozen);
        frozen
    };

    let default_info = get("DefaultInfo");
    let gn_substitution_info = get("GnSubstitutionInfo");
    let empty_default_info = get("empty_default_info");
    let default_info_type = crate::provider::UnpackProvider::unpack(empty_default_info.to_value()).unwrap();

    BuiltinModule {
        default_info,
        gn_substitution_info,
        default_info_type,
        empty_default_info,
    }
}

fn handle_gn_target_type<'v>(
    rule_name: &str,
    target_name: &str,
    kwargs: starlark::values::dict::DictRef<'v>,
    eval: &mut starlark::eval::Evaluator<'v, '_, '_>,
) -> starlark::Result<starlark::values::Value<'v>> {
    let extra: &crate::session::EvalContext = (&*eval).into();
    if !matches!(extra.kind, crate::session::EvalKind::BuildFile(_)) {
        return Err(Error::OnlyAllowedInMacros(rule_name.to_owned()).into());
    }

    let out = crate::ffi::Value::new();
    autocxx::prelude::moveit!(let mut out_pin = out);

    unsafe {
        crate::ffi::InitializeTargetScope(
            out_pin.as_mut(),
            extra.scope,
        );
    }

    for (k, v) in kwargs.iter() {
        let key_str = k.unpack_str().unwrap();
        let value_ptr = crate::ffi::SetScopeValueAt(out_pin.as_mut(), key_str);
        let value_pin = unsafe { std::pin::Pin::new_unchecked(&mut *value_ptr) };
        let heap_ref = starlark::values::FrozenHeapRef::default();
        crate::ffi::to_cxx_value(v, value_pin, &heap_ref, extra)?;
    }

    let err_ctor = crate::ffi::Err::new();
    autocxx::prelude::moveit!(let mut err = err_ctor);
    let target_ptr = unsafe {
        crate::ffi::CreateTarget(
            rule_name.into(),
            target_name.into(),
            extra.origin,
            out_pin.as_mut(),
            err.as_mut().get_mut() as *mut crate::ffi::Err,
        )
    };

    if target_ptr.is_null() {
        let err_msg = crate::ffi::GetErrorMessage(err.as_ref().get_ref());
        let err_str = err_msg.to_str().unwrap_or_default();
        return Err(Error::TargetCreationError(err_str.to_owned()).into());
    }

    Ok(starlark::values::Value::new_none())
}

#[starlark_module]
pub fn register_gn_rules(builder: &mut GlobalsBuilder) {
    fn action<'v>(
        #[starlark(require = named)] name: &str,
        #[starlark(kwargs)] kwargs: starlark::values::dict::DictRef<'v>,
        eval: &mut starlark::eval::Evaluator<'v, '_, '_>,
    ) -> starlark::Result<starlark::values::Value<'v>> {
        handle_gn_target_type("action", name, kwargs, eval)
    }

    fn action_foreach<'v>(
        #[starlark(require = named)] name: &str,
        #[starlark(kwargs)] kwargs: starlark::values::dict::DictRef<'v>,
        eval: &mut starlark::eval::Evaluator<'v, '_, '_>,
    ) -> starlark::Result<starlark::values::Value<'v>> {
        handle_gn_target_type("action_foreach", name, kwargs, eval)
    }

    fn bundle_data<'v>(
        #[starlark(require = named)] name: &str,
        #[starlark(kwargs)] kwargs: starlark::values::dict::DictRef<'v>,
        eval: &mut starlark::eval::Evaluator<'v, '_, '_>,
    ) -> starlark::Result<starlark::values::Value<'v>> {
        handle_gn_target_type("bundle_data", name, kwargs, eval)
    }

    fn create_bundle<'v>(
        #[starlark(require = named)] name: &str,
        #[starlark(kwargs)] kwargs: starlark::values::dict::DictRef<'v>,
        eval: &mut starlark::eval::Evaluator<'v, '_, '_>,
    ) -> starlark::Result<starlark::values::Value<'v>> {
        handle_gn_target_type("create_bundle", name, kwargs, eval)
    }

    fn copy<'v>(
        #[starlark(require = named)] name: &str,
        #[starlark(kwargs)] kwargs: starlark::values::dict::DictRef<'v>,
        eval: &mut starlark::eval::Evaluator<'v, '_, '_>,
    ) -> starlark::Result<starlark::values::Value<'v>> {
        handle_gn_target_type("copy", name, kwargs, eval)
    }

    fn executable<'v>(
        #[starlark(require = named)] name: &str,
        #[starlark(kwargs)] kwargs: starlark::values::dict::DictRef<'v>,
        eval: &mut starlark::eval::Evaluator<'v, '_, '_>,
    ) -> starlark::Result<starlark::values::Value<'v>> {
        handle_gn_target_type("executable", name, kwargs, eval)
    }

    fn group<'v>(
        #[starlark(require = named)] name: &str,
        #[starlark(kwargs)] kwargs: starlark::values::dict::DictRef<'v>,
        eval: &mut starlark::eval::Evaluator<'v, '_, '_>,
    ) -> starlark::Result<starlark::values::Value<'v>> {
        handle_gn_target_type("group", name, kwargs, eval)
    }

    fn loadable_module<'v>(
        #[starlark(require = named)] name: &str,
        #[starlark(kwargs)] kwargs: starlark::values::dict::DictRef<'v>,
        eval: &mut starlark::eval::Evaluator<'v, '_, '_>,
    ) -> starlark::Result<starlark::values::Value<'v>> {
        handle_gn_target_type("loadable_module", name, kwargs, eval)
    }

    fn shared_library<'v>(
        #[starlark(require = named)] name: &str,
        #[starlark(kwargs)] kwargs: starlark::values::dict::DictRef<'v>,
        eval: &mut starlark::eval::Evaluator<'v, '_, '_>,
    ) -> starlark::Result<starlark::values::Value<'v>> {
        handle_gn_target_type("shared_library", name, kwargs, eval)
    }

    fn source_set<'v>(
        #[starlark(require = named)] name: &str,
        #[starlark(kwargs)] kwargs: starlark::values::dict::DictRef<'v>,
        eval: &mut starlark::eval::Evaluator<'v, '_, '_>,
    ) -> starlark::Result<starlark::values::Value<'v>> {
        handle_gn_target_type("source_set", name, kwargs, eval)
    }

    fn static_library<'v>(
        #[starlark(require = named)] name: &str,
        #[starlark(kwargs)] kwargs: starlark::values::dict::DictRef<'v>,
        eval: &mut starlark::eval::Evaluator<'v, '_, '_>,
    ) -> starlark::Result<starlark::values::Value<'v>> {
        handle_gn_target_type("static_library", name, kwargs, eval)
    }

    fn generated_file<'v>(
        #[starlark(require = named)] name: &str,
        #[starlark(kwargs)] kwargs: starlark::values::dict::DictRef<'v>,
        eval: &mut starlark::eval::Evaluator<'v, '_, '_>,
    ) -> starlark::Result<starlark::values::Value<'v>> {
        handle_gn_target_type("generated_file", name, kwargs, eval)
    }

    fn rust_library<'v>(
        #[starlark(require = named)] name: &str,
        #[starlark(kwargs)] kwargs: starlark::values::dict::DictRef<'v>,
        eval: &mut starlark::eval::Evaluator<'v, '_, '_>,
    ) -> starlark::Result<starlark::values::Value<'v>> {
        handle_gn_target_type("rust_library", name, kwargs, eval)
    }

    fn rust_proc_macro<'v>(
        #[starlark(require = named)] name: &str,
        #[starlark(kwargs)] kwargs: starlark::values::dict::DictRef<'v>,
        eval: &mut starlark::eval::Evaluator<'v, '_, '_>,
    ) -> starlark::Result<starlark::values::Value<'v>> {
        handle_gn_target_type("rust_proc_macro", name, kwargs, eval)
    }
}
