// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use allocative::Allocative;
use starlark::values::ProvidesStaticType;
use types::{LabelRef, PackageRef, PathResolver};

use crate::{errors::Error, Scope};

enum EvalContextKind {
    BzlFile,
    Macro {
        scope: &'static Scope,
        err: std::cell::RefCell<std::pin::Pin<&'static mut crate::bridge::Err>>,
    },
}

#[derive(Allocative, ProvidesStaticType)]
pub struct EvalContext {
    #[allocative(skip)]
    session: &'static crate::session::Session,
    #[allocative(skip)]
    package: &'static PackageRef,
    #[allocative(skip)]
    kind: EvalContextKind,
}

impl EvalContext {
    pub fn new_bzl_file(
        session: &'static crate::session::Session,
        package: &'static PackageRef,
    ) -> Self {
        Self {
            session,
            package,
            kind: EvalContextKind::BzlFile,
        }
    }

    pub fn new_macro(
        session: &'static crate::session::Session,
        package: &'static PackageRef,
        scope: &'static Scope,
        err: std::pin::Pin<&'static mut crate::bridge::Err>,
    ) -> Self {
        Self {
            session,
            package,
            kind: EvalContextKind::Macro {
                scope,
                err: std::cell::RefCell::new(err),
            },
        }
    }
}

impl types::EvalContext for EvalContext {
    type Scope = crate::Scope;
    type Session = crate::session::Session;

    fn current_package(&self) -> &types::PackageRef {
        self.package
    }

    fn path_resolver(&self) -> &PathResolver {
        &self.session.path_resolver
    }

    fn session(&self) -> &Self::Session {
        self.session
    }

    fn current_toolchain(&self) -> LabelRef<'_> {
        todo!()
    }

    fn require_macro(&self) -> starlark::Result<&Self::Scope> {
        match &self.kind {
            EvalContextKind::Macro { scope, .. } => Ok(*scope),
            _ => Err(Error::RequiresMacro.into()),
        }
    }

    fn require_bzl(&self) -> starlark::Result<()> {
        matches!(self.kind, EvalContextKind::BzlFile)
            .then_some(())
            .ok_or_else(|| Error::RequiresBzlFile.into())
    }

    fn require_rule_impl(&self) -> starlark::Result<&mut types::CtxState<crate::TargetRef>> {
        todo!()
    }
}

impl attr::traits::EvalContextAttrExt for EvalContext {
    fn create_target(
        &self,
        target_type: Option<types::OutputType>,
        target_name: &str,
        scope: std::pin::Pin<&mut Scope>,
    ) -> starlark::Result<
        std::pin::Pin<
            &'static mut <<Self::Session as types::Session>::TargetRef as types::TargetRef>::Cxx,
        >,
    > {
        let output_type = target_type.map_or("", |t| t.into());
        let mut err_guard = match &self.kind {
            EvalContextKind::Macro { err, .. } => err.borrow_mut(),
            _ => return Err(Error::RequiresMacro.into()),
        };
        let target_ptr =
            crate::bridge::create_target(scope, target_name, output_type, err_guard.as_mut());
        err_guard.as_ref().into_result()?;
        let target_ref: &'static mut crate::bridge::Target =
            // Safety: Target allocated by GN in C++ outlives the session and is non-null when err is not set.
            unsafe { target_ptr.as_mut() }.expect("Target pointer is null but no error was set");
        // Safety: All C++ opaque types are marked Unpin, but target can safely be
        // pinned.
        Ok(unsafe { std::pin::Pin::new_unchecked(target_ref) })
    }

    fn register_target(
        &self,
        cxx_target: std::pin::Pin<
            &'static mut <<Self::Session as types::Session>::TargetRef as types::TargetRef>::Cxx,
        >,
        _rule: starlark::values::FrozenValue,
        _attrs: Vec<attr::Attr>,
    ) -> starlark::Result<<Self::Session as types::Session>::TargetRef> {
        let ffi_target: &'static crate::bridge::Target =
            std::pin::Pin::into_ref(cxx_target).get_ref();
        Ok(self
            .session
            .register_target(crate::target::Target { ffi: ffi_target }))
    }
}
