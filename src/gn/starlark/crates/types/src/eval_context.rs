// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use crate::{LabelRef, PackageRef, PathResolver, Session};

pub trait EvalContext:
    for<'v> starlark::values::ProvidesStaticType<'v, StaticType = Self>
    + allocative::Allocative
    + Send
    + Sync
    + 'static
{
    type Session: Session;

    fn current_package(&self) -> &PackageRef;
    fn path_resolver(&self) -> &PathResolver;
    fn session(&self) -> &Self::Session;
    fn current_toolchain(&self) -> LabelRef<'_>;
    fn require_macro(&self) -> starlark::Result<()>;
    fn require_bzl(&self) -> starlark::Result<()>;
    fn require_rule_impl(&self) -> starlark::Result<&crate::CtxState<<Self::Session as Session>::TargetRef>>;
    fn require_rule_impl_mut(&mut self) -> starlark::Result<&mut crate::CtxState<<Self::Session as Session>::TargetRef>>;
}

pub trait EvaluatorContextExt<'v, 'a, 'e> {
    fn context<C: EvalContext + 'static>(&self) -> &C;
    fn context_mut<C: EvalContext + 'static>(&mut self) -> &mut C;
}

impl<'v, 'a, 'e> EvaluatorContextExt<'v, 'a, 'e> for starlark::eval::Evaluator<'v, 'a, 'e> {
    #[inline]
    fn context<C: EvalContext + 'static>(&self) -> &C {
        let extra = self.extra.as_ref();
        // Safety: Extra *always* contains the evaluator context payload.
        unsafe { extra.unwrap_unchecked() }
            .downcast_ref::<C>()
            .unwrap()
    }

    #[inline]
    fn context_mut<C: EvalContext + 'static>(&mut self) -> &mut C {
        let extra = self.extra.as_mut();
        // Safety: Extra *always* contains the evaluator context payload.
        let val_ref = unsafe { extra.unwrap_unchecked() }
            .downcast_ref::<C>()
            .unwrap();
        unsafe { crate::util::add_mut(val_ref) }
    }
}
