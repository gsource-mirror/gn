// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use starlark::environment::Module;
use starlark::eval::Evaluator;
use starlark::values::{OwnedFrozenValue, ProvidesStaticType, Trace, Tracer};

use types::EvalContext as TypesEvalContext;
use attr::EvalContextExt as AttrEvalContextExt;
use crate::ctx::CtxState;
use crate::ffi;
use crate::Package;

/// Represents the phase or context of Starlark code evaluation.
#[derive(Debug, Clone, allocative::Allocative)]
pub enum EvalKind {
    /// When calling load("foo.bzl", ...) and actually executing the bzl file.
    /// The package is the directory the bzl file lives in.
    BzlFile(Package),
    /// When calling load("foo.bzl", "foo"), converting foo from a starlark to C++ variable.
    VariableConversion,
    /// When calling starlark macros.
    /// The package is the directory of the caller.
    Macro(Package),
    /// When executing the implementation of a rule.
    RuleEval(CtxState),
}

unsafe impl<'v> Trace<'v> for EvalKind {
    fn trace(&mut self, _tracer: &Tracer<'v>) {}
}

/// Thread-local compilation state passed via the evaluator's `extra` field.
/// Coordinates FFI communication with the C++ GN Scope and ParseNode context.
#[derive(allocative::Allocative)]
pub struct EvalContext {
    /// Pointer to the underlying C++ `Scope` object.
    #[allocative(skip)]
    pub scope: *mut ffi::Scope,
    /// Pointer to the C++ `ParseNode` originating this evaluation.
    #[allocative(skip)]
    pub origin: *const ffi::ParseNode,
    /// The compilation dialect/evaluation phase.
    pub kind: EvalKind,
}

unsafe impl<'v> Trace<'v> for EvalContext {
    fn trace(&mut self, tracer: &Tracer<'v>) {
        self.kind.trace(tracer);
    }
}

unsafe impl Send for EvalContext {}
unsafe impl Sync for EvalContext {}

impl EvalContext {
    /// Creates a new `EvalContext`.
    pub fn new(scope: *mut ffi::Scope, origin: *const ffi::ParseNode, kind: EvalKind) -> Self {
        Self {
            scope,
            origin,
            kind,
        }
    }
}

unsafe impl<'v> ProvidesStaticType<'v> for EvalContext {
    type StaticType = Self;
}

impl EvalContext {
    /// Retrieves the C++ `Settings` object associated with the current toolchain.
    pub fn settings(&self) -> &ffi::Settings {
        unsafe { (&*self.scope).settings() }
    }
}


impl TypesEvalContext for EvalContext {
    type Session = crate::StarlarkSession;

    fn current_package(&self) -> &types::PackageRef {
        match &self.kind {
            EvalKind::BzlFile(package) => package,
            EvalKind::Macro(package) => package,
            EvalKind::RuleEval(state) => state.target.package(),
            EvalKind::VariableConversion => unreachable!(),
        }
    }

    fn path_resolver(&self) -> &types::PathResolver {
        let session = unsafe { (&*self.scope).starlark_session() };
        session.path_resolver()
    }

    fn session(&self) -> &Self::Session {
        unsafe { (&*self.scope).starlark_session() }
    }

    fn current_toolchain(&self) -> types::LabelRef<'_> {
        self.settings().toolchain_label()
    }

    fn require_macro(&self) -> starlark::Result<()> {
        if matches!(self.kind, EvalKind::Macro(_)) {
            Ok(())
        } else {
            Err(starlark::Error::new_other(crate::Error::OnlyAllowedIn("macros".to_owned())))
        }
    }

    fn require_bzl(&self) -> starlark::Result<()> {
        if matches!(self.kind, EvalKind::BzlFile(_)) {
            Ok(())
        } else {
            Err(starlark::Error::new_other(crate::Error::OnlyAllowedIn("bzl files".to_owned())))
        }
    }

    fn require_rule_impl(&self) -> starlark::Result<&types::CtxState<crate::TargetRef>> {
        if let EvalKind::RuleEval(state) = &self.kind {
            Ok(state)
        } else {
            Err(starlark::Error::new_other(crate::Error::OnlyAllowedIn("rule implementation".to_owned())))
        }
    }

    fn require_rule_impl_mut(&mut self) -> starlark::Result<&mut types::CtxState<crate::TargetRef>> {
        if let EvalKind::RuleEval(state) = &self.kind {
            unsafe { Ok(types::util::add_mut(state)) }
        } else {
            Err(starlark::Error::new_other(crate::Error::OnlyAllowedIn("rule implementation".to_owned())))
        }
    }
}

impl AttrEvalContextExt for EvalContext {
    fn create_starlark_target(
        &self,
        target_name: &str,
        rule: starlark::values::FrozenValue,
        attrs: Vec<attr::Attr>,
    ) -> Result<<Self::Session as attr::Session>::TargetRef, types::Error> {
        let out = ffi::GnValue::new();
        struct ValueGuard(*mut ffi::GnValue);
        impl Drop for ValueGuard {
            fn drop(&mut self) {
                ffi::GnValue::free(self.0);
            }
        }
        let _out_guard = ValueGuard(out);
        let out_ref = unsafe { &mut *out };
        out_ref.initialize_target_scope(unsafe { self.scope.as_mut() });

        let err = ffi::Err::new();
        struct ErrGuard(*mut ffi::Err);
        impl Drop for ErrGuard {
            fn drop(&mut self) {
                ffi::Err::free(self.0);
            }
        }
        let _err_guard = ErrGuard(err);
        let err_ref = unsafe { &mut *err };

        let origin_ref = if self.origin.is_null() {
            None
        } else {
            Some(unsafe { &*self.origin })
        };

        let rule_typed = starlark::values::FrozenValueTyped::<crate::rule::FrozenRule>::new(rule)
            .ok_or_else(|| {
                types::Error::TargetCreationError("Rule must be a rule object".to_owned())
            })?;

        let created_ref = ffi::Target::create("", target_name, origin_ref, out_ref, err_ref);
        if created_ref.is_none() {
            let err_str = err_ref.message();
            return Err(types::Error::TargetCreationError(err_str));
        }

        let rust_target = crate::Target::new_starlark(created_ref.unwrap(), rule_typed, attrs);
        let session = self.session();
        Ok(session.register_target(rust_target))
    }
}

impl attr::TargetRefExt for crate::TargetRef {
    fn set_attrs(&self, attrs: Vec<attr::Attr>) {
        let target_static: &'static crate::Target = (*self).into();
        let target_ptr = target_static as *const crate::Target as *mut crate::Target;
        unsafe {
            (*target_ptr).attrs = attrs;
        }
    }
}

/// Helper to call a frozen Starlark function with positional and keyword arguments, propagating errors.
pub fn invoke_starlark_function(
    func: &OwnedFrozenValue,
    args: &[&OwnedFrozenValue],
    kwargs: &[(&str, &OwnedFrozenValue)],
    context: &EvalContext,
) -> Result<OwnedFrozenValue, starlark::Error> {
    Module::with_temp_heap(|module| {
        let res = {
            let mut eval = Evaluator::new(&module);
            eval.extra = Some(context);

            let positional_args: Vec<starlark::values::Value> = args
                .iter()
                .map(|item| {
                    module.heap().add_reference(item.owner());
                    // Safety: item owner is registered in the module heap, keeping it alive during evaluation.
                    unsafe { item.unchecked_frozen_value().to_value() }
                })
                .collect();

            let named_args: Vec<(&str, starlark::values::Value)> = kwargs
                .iter()
                .map(|(name, item)| {
                    module.heap().add_reference(item.owner());
                    // Safety: item owner is registered in the module heap, keeping it alive during evaluation.
                    let val = unsafe { item.unchecked_frozen_value().to_value() };
                    (*name, val)
                })
                .collect();

            // Safety: func owner is registered in the module heap, keeping it alive during evaluation.
            let func_val = unsafe { func.unchecked_frozen_value().to_value() };
            module.heap().add_reference(func.owner());

            eval.eval_function(func_val, &positional_args, &named_args)?
        };

        module.set("", res);

        Ok(module.freeze()?.get("").unwrap())
    })
}
