// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

use allocative::Allocative;
use starlark::environment::Methods;
use starlark::environment::MethodsBuilder;
use starlark::environment::MethodsStatic;
use starlark::eval::Evaluator;
use starlark::starlark_simple_value;
use starlark::values::ProvidesStaticType;
use starlark::values::StarlarkValue;
use starlark::values::Value;
use starlark_derive::starlark_module;
use starlark_derive::starlark_value;
use starlark_derive::NoSerialize;

use crate::File;
use crate::EvalContext;
use types::EvaluatorContextExt;
use types::EvalContext as _;

/// ctx.gn provides access to GN internals of a target.
/// It is only set for rule extension targets.
#[derive(Debug, ProvidesStaticType, NoSerialize, Allocative)]
#[repr(C)]
pub struct Gn;

starlark_simple_value!(Gn);

crate::cannot_freeze!(Gn);

impl Display for Gn {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "ctx.gn")
    }
}

#[starlark_value(type = "ctx.gn")]
impl<'v> StarlarkValue<'v> for Gn {
    fn get_methods() -> Option<&'static Methods> {
        static RES: MethodsStatic = MethodsStatic::new("ctx.gn", gn_methods);
        Some(RES.methods())
    }
}

/// Registers the Starlark methods for the `ctx.gn` object.
#[starlark_module]
pub fn gn_methods(builder: &mut MethodsBuilder) {
    fn get_output_files<'v>(
        this: &Gn,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        let _ = this;
        let ctx = eval.context::<EvalContext>();
        Ok(eval.heap().alloc(
            ctx.require_rule_impl()?
                .target
                .outputs()
                .iter()
                .map(|f: &File| eval.heap().alloc(f.clone()))
                .collect::<Vec<_>>(),
        ))
    }

    fn deps<'v>(this: &Gn, eval: &mut Evaluator<'v, '_, '_>) -> starlark::Result<Value<'v>> {
        let _ = this;
        let ctx = eval.context::<EvalContext>();
        Ok(eval.heap().alloc(
            unsafe { &*ctx.require_rule_impl()?.target.ptr() }
                .deps()
                .iter()
                .map(|t| eval.heap().alloc::<crate::TargetRef>(t.as_rust()))
                .collect::<Vec<_>>(),
        ))
    }

    fn public_deps<'v>(this: &Gn, eval: &mut Evaluator<'v, '_, '_>) -> starlark::Result<Value<'v>> {
        let _ = this;
        let ctx = eval.context::<EvalContext>();
        Ok(eval.heap().alloc(
            unsafe { &*ctx.require_rule_impl()?.target.ptr() }
                .public_deps()
                .iter()
                .map(|t| eval.heap().alloc::<crate::TargetRef>(t.as_rust()))
                .collect::<Vec<_>>(),
        ))
    }

    fn sources<'v>(this: &Gn, eval: &mut Evaluator<'v, '_, '_>) -> starlark::Result<Value<'v>> {
        let _ = this;
        let ctx = eval.context::<EvalContext>();
        Ok(eval.heap().alloc(
            unsafe { &*ctx.require_rule_impl()?.target.ptr() }
                .private_sources()
                .iter()
                .map(|s| eval.heap().alloc::<File>(File::from_rust(s.to_string())))
                .collect::<Vec<_>>(),
        ))
    }

    fn public<'v>(this: &Gn, eval: &mut Evaluator<'v, '_, '_>) -> starlark::Result<Value<'v>> {
        let _ = this;
        let ctx = eval.context::<EvalContext>();
        Ok(eval.heap().alloc(
            unsafe { &*ctx.require_rule_impl()?.target.ptr() }
                .public_sources()
                .iter()
                .map(|s| eval.heap().alloc::<File>(File::from_rust(s.to_string())))
                .collect::<Vec<_>>(),
        ))
    }
}

impl Gn {
    /// Creates a new `Gn` context accessor.
    pub fn new() -> Self {
        Self
    }
}
