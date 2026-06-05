// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use attr::{Attr, EvalContext as AttrEvalContext, EvalContextAttrExt, Session as AttrSession};
use starlark::{
    values::{FrozenValue, ProvidesStaticType},
    Result,
};
use types::{CtxState, Label, LabelRef, Package, PackageRef, PathResolver, Session};

use crate::{FakeSession, FakeTarget, FakeTargetRef};

/// A simple implementation of the evaluation context used in Starlark unit
/// tests.
#[derive(allocative::Allocative)]
pub struct FakeEvalContext {
    /// The current package being processed.
    pub package: Package,
    /// The current toolchain.
    pub current_toolchain: Label,
    /// The fake starlark session.
    #[allocative(skip)]
    pub session: FakeSession,
    /// The fake path resolver.
    #[allocative(skip)]
    pub path_resolver: PathResolver,
    /// The fake rule state.
    #[allocative(skip)]
    pub rule_state: CtxState<FakeTargetRef>,
}

unsafe impl<'v> ProvidesStaticType<'v> for FakeEvalContext {
    type StaticType = Self;
}

impl Default for FakeEvalContext {
    fn default() -> Self {
        Self::new("//")
    }
}

impl FakeEvalContext {
    /// Creates a new `FakeEvalContext` for a given package name.
    pub fn new(package: &str) -> Self {
        let session = FakeSession::new();
        let package_ref = PackageRef::new_for_testing(package).to_owned();
        Self {
            package: package_ref,
            current_toolchain: session.default_toolchain.clone(),
            session,
            path_resolver: PathResolver::new_for_testing(),
            rule_state: CtxState::new(FakeTargetRef::default()),
        }
    }
}

impl AttrEvalContext for FakeEvalContext {
    type Session = FakeSession;

    fn session(&self) -> &Self::Session {
        &self.session
    }

    fn current_package(&self) -> &PackageRef {
        &self.package
    }

    fn path_resolver(&self) -> &PathResolver {
        &self.path_resolver
    }

    fn current_toolchain(&self) -> LabelRef<'_> {
        self.current_toolchain.as_ref()
    }

    fn require_macro(&self) -> Result<()> {
        Ok(())
    }

    fn require_bzl(&self) -> Result<()> {
        Ok(())
    }

    fn require_rule_impl(&self) -> Result<&CtxState<<Self::Session as Session>::TargetRef>> {
        Ok(&self.rule_state)
    }

    fn require_rule_impl_mut(
        &mut self,
    ) -> Result<&mut CtxState<<Self::Session as Session>::TargetRef>> {
        Ok(&mut self.rule_state)
    }
}

impl EvalContextAttrExt for FakeEvalContext {
    fn create_starlark_target(
        &self,
        target_name: &str,
        _rule: FrozenValue,
        attrs: Vec<Attr>,
    ) -> Result<<Self::Session as AttrSession>::TargetRef> {
        let label = Label::new(self.package.clone(), target_name.to_owned());
        let target = FakeTargetRef::new(FakeTarget {
            outputs: vec![],
            attrs,
            ..Default::default()
        });
        self.session.insert_target(label, target.clone());
        Ok(target)
    }
}
