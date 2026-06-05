// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use crate::label::{LabelRef, Package, PackageRef};
use crate::session::StarlarkSession;
use crate::ffi::AsRust;
use starlark::values::{ProvidesStaticType, Trace};
use starlark::eval::Evaluator;

/// Represents the phase or context of Starlark code evaluation.
#[derive(Debug, Trace, Clone, PartialEq, Eq)]
pub enum EvalKind {
    /// When calling load("foo.bzl", ...) and actually executing the bzl file.
    /// The package is the directory the bzl file lives in.
    BzlFile(Package),
    /// When calling load("foo.bzl", "foo"), converting foo from a starlark to C++ variable.
    VariableConversion,
    /// When calling the function.
    /// The package is the directory of the caller.
    BuildFile(Package),
    /// When executing the implementation of a rule.
    RuleEval,
}

/// Thread-local compilation state passed via the evaluator's `extra` field.
/// Coordinates FFI communication with the C++ GN Scope and ParseNode context.
pub struct EvalContext {
    /// Pointer to the underlying C++ `Scope` object.
    pub scope: *mut crate::ffi::Scope,
    /// Pointer to the C++ `ParseNode` originating this evaluation.
    pub origin: *const crate::ffi::ParseNode,
    /// The compilation dialect/evaluation phase.
    pub kind: EvalKind,
    pub(crate) heaps: std::cell::RefCell<Vec<starlark::values::FrozenHeapRef>>,
}

unsafe impl<'v> Trace<'v> for EvalContext {
    fn trace(&mut self, tracer: &starlark::values::Tracer<'v>) {
        self.kind.trace(tracer);
    }
}

impl EvalContext {
    /// Creates a new `EvalContext`.
    pub fn new(scope: *mut crate::ffi::Scope, origin: *const crate::ffi::ParseNode, kind: EvalKind) -> Self {
        Self {
            scope,
            origin,
            kind,
            heaps: std::cell::RefCell::new(Vec::new()),
        }
    }

    pub fn keep_alive(&self, heap: starlark::values::FrozenHeapRef) {
        self.heaps.borrow_mut().push(heap);
    }
}

unsafe impl<'v> ProvidesStaticType<'v> for EvalContext {
    type StaticType = Self;
}

impl EvalContext {
    pub fn try_from_eval<'v, 'a, 'e, 'r>(eval: &'r Evaluator<'v, 'a, 'e>) -> Option<&'r EvalContext> {
        eval.extra.as_ref()?.downcast_ref::<EvalContext>()
    }

    /// Retrieves the C++ `Settings` object associated with the current toolchain.
    pub fn settings(&self) -> &crate::ffi::Settings {
        unsafe { &*crate::ffi::GetSettingsFromScope(self.scope) }
    }

    /// Gets the current package/directory being evaluated.
    pub fn current_package(&self) -> &PackageRef {
        match &self.kind {
            EvalKind::BzlFile(package) => package,
            EvalKind::BuildFile(package) => package,
            EvalKind::VariableConversion | EvalKind::RuleEval => unreachable!("Irrelevant"),
        }
    }

    /// Gets the C++ `Label` pointer representing the current toolchain.
    pub fn current_toolchain<'a>(&'a self) -> LabelRef<'a> {
        crate::ffi::GetToolchainLabelFromSettings(self.settings()).into()
    }

    /// Gets the associated Rust-side `StarlarkSession` manager.
    pub fn session<'a>(&self) -> &'a StarlarkSession {
        let session_ptr = unsafe { crate::ffi::GetStarlarkSessionFromScope(self.scope) };
        let session_ref = unsafe { &*session_ptr };
        session_ref.as_rust()
    }
}

impl<'v, 'a, 'e, 'r> From<&'r Evaluator<'v, 'a, 'e>> for &'r EvalContext {
    fn from(eval: &'r Evaluator<'v, 'a, 'e>) -> Self {
        eval.extra.as_ref().unwrap().downcast_ref::<EvalContext>().unwrap()
    }
}

impl<'v, 'a, 'e, 'r> From<&'r mut Evaluator<'v, 'a, 'e>> for &'r EvalContext {
    fn from(eval: &'r mut Evaluator<'v, 'a, 'e>) -> Self {
        eval.extra.as_ref().unwrap().downcast_ref::<EvalContext>().unwrap()
    }
}
