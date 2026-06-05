// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

pub(crate) use providers::BuiltinModule;
use starlark::environment::Globals;
use starlark::environment::GlobalsBuilder;
use starlark::environment::LibraryExtension;
use starlark::eval::Evaluator;
use starlark::values::dict::DictRef;
use starlark::values::FrozenHeapRef;
use starlark::values::Value;
use starlark_derive::starlark_module;

use crate::ffi;
use crate::session::evaluator::{EvalContext, EvalKind};
use attr::EvalContext as _;
use types::EvaluatorContextExt;

mod attr_globals {
    use attr::EvalContext as _;
    attr::declare_attr_module!(crate::EvalContext);
}
use attr_globals::AttrModule;

fn base_builder() -> GlobalsBuilder {
    #[allow(unused_mut)]
    let mut builder = GlobalsBuilder::extended_by(&[
        LibraryExtension::Debug,
        LibraryExtension::Print,  // Adds print()
        LibraryExtension::Pprint, // Adds pprint()
        LibraryExtension::RecordType,
        LibraryExtension::SetType,
        LibraryExtension::StructType,
    ]);
    #[cfg(test)]
    args::register_args_test_globals(&mut builder);
    builder
}

pub(crate) fn populate_bzl_globals(builder: &mut GlobalsBuilder, builtins: &BuiltinModule) {
    builder.set("DefaultInfo", builtins.default_info);
    builder.set("GnSubstitutionInfo", builtins.gn_substitution_info);
    builder.set("GnInputsInfo", builtins.gn_inputs_info);
    builder.set("empty_default_info", builtins.empty_default_info);
    crate::rule::register_rule(builder);
    builder.set("attr", AttrModule);
    providers::register_provider(builder);
    register_depset(builder);
    register_gn_rules(builder);
}

fn bzl_globals_builder(builtins: &BuiltinModule) -> GlobalsBuilder {
    let mut builder = base_builder();
    populate_bzl_globals(&mut builder, builtins);
    builder
}

pub(crate) fn make_bzl_globals(builtins: &BuiltinModule) -> Globals {
    bzl_globals_builder(builtins).build()
}

pub(crate) fn populate_test_globals(builder: &mut GlobalsBuilder, builtins: &BuiltinModule) {
    populate_bzl_globals(builder, builtins);
    args::register_args_test_globals(builder);
    crate::testing::test_globals(builder);
}

fn handle_gn_target_type<'v>(
    rule_name: &str,
    target_name: &str,
    kwargs: DictRef<'v>,
    eval: &mut Evaluator<'v, '_, '_>,
) -> starlark::Result<Value<'v>> {
    let extra = eval.context::<EvalContext>();
    if !matches!(extra.kind, EvalKind::Macro(_)) {
        return Err(crate::Error::OnlyAllowedIn(rule_name.to_owned()).into());
    }

    let out = ffi::GnValue::new();
    struct ValueGuard(*mut ffi::GnValue);
    impl Drop for ValueGuard {
        fn drop(&mut self) {
            ffi::GnValue::free(self.0);
        }
    }
    let _out_guard = ValueGuard(out);
    let out_ref = unsafe { &mut *out };

    out_ref.initialize_target_scope(unsafe { extra.scope.as_mut() });

    for (k, v) in kwargs.iter() {
        let key_str = k.unpack_str().unwrap();
        if let Some(value_ref) = out_ref.set_scope_value_at(key_str) {
            let heap_ref = FrozenHeapRef::default();
            crate::to_cxx_value(v, value_ref, &heap_ref, extra)?;
        }
    }

    let err = ffi::Err::new();
    struct ErrGuard(*mut ffi::Err);
    impl Drop for ErrGuard {
        fn drop(&mut self) {
            ffi::Err::free(self.0);
        }
    }
    let _err_guard = ErrGuard(err);
    let err_ref = unsafe { &mut *err };

    let origin_ref = if extra.origin.is_null() {
        None
    } else {
        Some(unsafe { &*extra.origin })
    };

    let target_ref = ffi::Target::create(rule_name, target_name, origin_ref, out_ref, err_ref);

    if target_ref.is_none() {
        let err_str = err_ref.message();
        return Err(types::Error::TargetCreationError(err_str).into());
    }
    let target_ref = target_ref.unwrap();

    if crate::testing::IS_TESTING.load(std::sync::atomic::Ordering::Relaxed) {
        // In tests, we immediately register the target in the session to ensure that starlark knows about it.
        // TODO: Call OnResolved() like we do in GN tests?
        let eval_context = eval.context::<EvalContext>();
        eval_context
            .session()
            .register_target(crate::Target::new_cxx(
                target_ref,
                eval_context.session(),
            ));
    }

    Ok(Value::new_none())
}

#[starlark_module]
pub fn register_gn_rules(builder: &mut GlobalsBuilder) {
    fn action<'v>(
        #[starlark(require = named)] name: &str,
        #[starlark(kwargs)] kwargs: DictRef<'v>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        handle_gn_target_type("action", name, kwargs, eval)
    }

    fn action_foreach<'v>(
        #[starlark(require = named)] name: &str,
        #[starlark(kwargs)] kwargs: DictRef<'v>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        handle_gn_target_type("action_foreach", name, kwargs, eval)
    }

    fn bundle_data<'v>(
        #[starlark(require = named)] name: &str,
        #[starlark(kwargs)] kwargs: DictRef<'v>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        handle_gn_target_type("bundle_data", name, kwargs, eval)
    }

    fn create_bundle<'v>(
        #[starlark(require = named)] name: &str,
        #[starlark(kwargs)] kwargs: DictRef<'v>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        handle_gn_target_type("create_bundle", name, kwargs, eval)
    }

    fn copy<'v>(
        #[starlark(require = named)] name: &str,
        #[starlark(kwargs)] kwargs: DictRef<'v>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        handle_gn_target_type("copy", name, kwargs, eval)
    }

    fn executable<'v>(
        #[starlark(require = named)] name: &str,
        #[starlark(kwargs)] kwargs: DictRef<'v>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        handle_gn_target_type("executable", name, kwargs, eval)
    }

    fn group<'v>(
        #[starlark(require = named)] name: &str,
        #[starlark(kwargs)] kwargs: DictRef<'v>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        handle_gn_target_type("group", name, kwargs, eval)
    }

    fn loadable_module<'v>(
        #[starlark(require = named)] name: &str,
        #[starlark(kwargs)] kwargs: DictRef<'v>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        handle_gn_target_type("loadable_module", name, kwargs, eval)
    }

    fn shared_library<'v>(
        #[starlark(require = named)] name: &str,
        #[starlark(kwargs)] kwargs: DictRef<'v>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        handle_gn_target_type("shared_library", name, kwargs, eval)
    }

    fn source_set<'v>(
        #[starlark(require = named)] name: &str,
        #[starlark(kwargs)] kwargs: DictRef<'v>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        handle_gn_target_type("source_set", name, kwargs, eval)
    }

    fn static_library<'v>(
        #[starlark(require = named)] name: &str,
        #[starlark(kwargs)] kwargs: DictRef<'v>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        handle_gn_target_type("static_library", name, kwargs, eval)
    }

    fn generated_file<'v>(
        #[starlark(require = named)] name: &str,
        #[starlark(kwargs)] kwargs: DictRef<'v>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        handle_gn_target_type("generated_file", name, kwargs, eval)
    }

    fn rust_library<'v>(
        #[starlark(require = named)] name: &str,
        #[starlark(kwargs)] kwargs: DictRef<'v>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        handle_gn_target_type("rust_library", name, kwargs, eval)
    }

    fn rust_proc_macro<'v>(
        #[starlark(require = named)] name: &str,
        #[starlark(kwargs)] kwargs: DictRef<'v>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        handle_gn_target_type("rust_proc_macro", name, kwargs, eval)
    }
}

#[starlark_module]
pub fn register_depset(builder: &mut GlobalsBuilder) {
    fn depset<'v>(
        direct: Option<starlark::values::list::UnpackList<Value<'v>>>,
        transitive: Option<starlark::values::list::UnpackList<depset::UnpackDepset<'v>>>,
        #[starlark(default = depset::Order::Unspecified)] order: depset::Order,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        depset::depset_constructor::<EvalContext>(
            direct,
            transitive,
            order,
            &eval.heap(),
            eval.context_mut::<EvalContext>(),
        )
    }
}
