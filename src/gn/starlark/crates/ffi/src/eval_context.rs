// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use allocative::Allocative;
use starlark::values::ProvidesStaticType;
use types::{LabelRef, PackageRef, PathResolver};

#[derive(Allocative)]
enum EvalContextKind {
    BzlFile,
}

#[derive(Allocative, ProvidesStaticType)]
pub struct EvalContext {
    #[allocative(skip)]
    session: *const crate::session::Session,
    #[allocative(skip)]
    package: *const PackageRef,
    kind: EvalContextKind,
}

impl EvalContext {
    pub fn new_bzl_file(
        session: *const crate::session::Session,
        package: *const PackageRef,
    ) -> Self {
        Self {
            session,
            package,
            kind: EvalContextKind::BzlFile,
        }
    }
}

impl types::EvalContext for EvalContext {
    type Scope = crate::scope::OwnedScope;
    type Session = crate::session::Session;

    fn current_package(&self) -> &types::PackageRef {
        // Safety: The session guarantees package reference lives as long as EvalContext
        // is evaluated.
        unsafe { &*self.package }
    }

    fn path_resolver(&self) -> &PathResolver {
        // Safety: The session reference is guaranteed to be valid during evaluation.
        unsafe { &(*self.session).path_resolver }
    }

    fn session(&self) -> &Self::Session {
        // Safety: The session reference is guaranteed to be valid during evaluation.
        unsafe { &*self.session }
    }

    fn current_toolchain(&self) -> LabelRef<'_> {
        todo!()
    }

    fn require_macro(&self) -> starlark::Result<&Self::Scope> {
        todo!()
    }

    fn require_bzl(&self) -> starlark::Result<()> {
        todo!()
    }

    fn require_rule_impl(
        &self,
    ) -> starlark::Result<&mut types::CtxState<crate::target::TargetRef>> {
        todo!()
    }
}

impl attr::traits::EvalContextAttrExt for EvalContext {
    fn create_target(
        &self,
        _target_type: Option<types::OutputType>,
        _target_name: &str,
        _scope: &Self::Scope,
        _rule: starlark::values::FrozenValue,
        _attrs: Vec<attr::Attr>,
    ) -> starlark::Result<<Self::Session as types::Session>::TargetRef> {
        todo!()
    }
}
