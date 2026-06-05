// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::path::PathBuf;

use starlark::values::ProvidesStaticType;
use types::Label;

use crate::FakeSession;
use attr::EvalContextExt;

/// A mock implementation of the evaluation context used in Starlark unit tests.
#[derive(allocative::Allocative)]
pub struct FakeEvalContext {
    /// The current package being processed.
    pub package: types::Package,
    /// The current toolchain.
    pub current_toolchain: Label,
    /// The mock starlark session.
    #[allocative(skip)]
    pub session: FakeSession,
    /// The mock path resolver.
    #[allocative(skip)]
    pub path_resolver: types::PathResolver,
    /// The mock rule state.
    #[allocative(skip)]
    pub rule_state: std::cell::UnsafeCell<types::CtxState<crate::FakeTargetRef>>,
}

unsafe impl<'v> ProvidesStaticType<'v> for FakeEvalContext {
    type StaticType = Self;
}

unsafe impl Send for FakeEvalContext {}
unsafe impl Sync for FakeEvalContext {}

impl Default for FakeEvalContext {
    fn default() -> Self {
        Self::new("//".into())
    }
}

impl FakeEvalContext {
    /// Creates a new `FakeEvalContext` for a given package name.
    pub fn new(package: &str) -> Self {
        let session = FakeSession::new();
        let target = crate::FakeTargetRef(std::rc::Rc::new(crate::FakeTarget { outputs: vec![] }));
        Self {
            package: types::Package::from(package.to_owned()),
            current_toolchain: session.default_toolchain.clone(),
            session,
            path_resolver: types::PathResolver::new(PathBuf::from("/"), "".to_string()),
            rule_state: std::cell::UnsafeCell::new(types::CtxState::new(target)),
        }
    }

}

impl attr::EvalContext for FakeEvalContext {
    type Session = FakeSession;

    fn session(&self) -> &Self::Session {
        &self.session
    }

    fn current_package(&self) -> &types::PackageRef {
        &self.package
    }

    fn path_resolver(&self) -> &types::PathResolver {
        &self.path_resolver
    }

    fn current_toolchain(&self) -> types::LabelRef<'_> {
        self.current_toolchain.as_ref()
    }

    fn require_macro(&self) -> starlark::Result<()> {
        Ok(())
    }

    fn require_bzl(&self) -> starlark::Result<()> {
        Ok(())
    }

    fn require_rule_impl(&self) -> starlark::Result<&types::CtxState<<Self::Session as types::Session>::TargetRef>> {
        // Safety: Evaluator is single-threaded, and concurrent reads do not overlap writes.
        unsafe { Ok(&*self.rule_state.get()) }
    }

    fn require_rule_impl_mut(&mut self) -> starlark::Result<&mut types::CtxState<<Self::Session as types::Session>::TargetRef>> {
        // Safety: Evaluator is single-threaded, and writes do not overlap active borrows.
        unsafe { Ok(&mut *self.rule_state.get()) }
    }
}

impl EvalContextExt for FakeEvalContext {
    fn create_starlark_target(
        &self,
        target_name: &str,
        _rule: starlark::values::FrozenValue,
        _attrs: Vec<attr::Attr>,
    ) -> Result<<Self::Session as attr::Session>::TargetRef, types::Error> {
        let label = types::Label::new(self.package.clone(), target_name.to_owned());
        let target = crate::FakeTargetRef(std::rc::Rc::new(crate::FakeTarget { outputs: vec![] }));
        self.session.insert_target(label, target.clone());
        Ok(target)
    }
}

impl attr::TargetRefExt for crate::FakeTargetRef {
    fn set_attrs(&self, _attrs: Vec<attr::Attr>) {}
}

