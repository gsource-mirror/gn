// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

use allocative::Allocative;
use args::{Args, ArgsGen};
use depset::UnpackFileDepset;
use either::Either;
use starlark::environment::Methods;
use starlark::environment::MethodsBuilder;
use starlark::environment::MethodsStatic;
use starlark::eval::Evaluator;
use starlark::starlark_simple_value;
use starlark::values::list::UnpackList;
use starlark::values::ProvidesStaticType;
use starlark::values::StarlarkValue;
use starlark::values::Value;
use starlark::values::ValueTyped;
use starlark_derive::starlark_module;
use starlark_derive::starlark_value;
use starlark_derive::NoSerialize;

use crate::EvalContext;
use crate::File;
use types::EvalContext as _;
use types::EvaluatorContextExt as _;

/// The `ctx.actions` object which provides APIs to declare output files and register build actions.
#[derive(Debug, ProvidesStaticType, NoSerialize, Allocative)]
pub struct Actions;

starlark_simple_value!(Actions);

impl Display for Actions {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "ctx.actions")
    }
}

#[starlark_value(type = "ctx.actions")]
impl<'v> StarlarkValue<'v> for Actions {
    fn get_methods() -> Option<&'static Methods> {
        static RES: MethodsStatic = MethodsStatic::new("ctx.actions", actions_methods);
        Some(RES.methods())
    }
}

crate::cannot_freeze!(Actions);

/// Registers the Starlark methods for the `ctx.actions` object.
#[starlark_module]
pub fn actions_methods(builder: &mut MethodsBuilder) {
    fn declare_file<'v>(
        this: &Actions,
        filename: &str,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        let _ = this;
        let file = {
            eval.context_mut::<EvalContext>().require_rule_impl_mut()?.declare_file(filename)
        };
        Ok(eval.heap().alloc(file))
    }

    fn run<'v>(
        this: &Actions,
        outputs: UnpackList<ValueTyped<File>>,
        inputs: Either<UnpackList<&File>, UnpackFileDepset>,
        executable: &File,
        arguments: Option<UnpackList<Either<&ArgsGen<Value<'v>>, &'v str>>>,
        env: Option<starlark::values::dict::UnpackDictEntries<&'v str, &'v str>>,
    ) -> starlark::Result<Value<'v>> {
        drop((this, outputs, inputs, executable, arguments, env));
        todo!()
    }

    fn run_shell<'v>(
        this: &Actions,
        outputs: Value<'v>,
        inputs: Value<'v>,
        command: Value<'v>,
        arguments: Option<Value<'v>>,
        env: Option<Value<'v>>,
    ) -> starlark::Result<Value<'v>> {
        let _ = (this, outputs, inputs, command, arguments, env);
        todo!()
    }

    fn write<'v>(
        this: &Actions,
        output: Value<'v>,
        content: Value<'v>,
        #[starlark(default = false)] is_executable: bool,
    ) -> starlark::Result<Value<'v>> {
        let _ = (this, output, content, is_executable);
        todo!()
    }

    fn args<'v>(this: &Actions, eval: &mut Evaluator<'v, '_, '_>) -> starlark::Result<Value<'v>> {
        let _ = this;
        Ok(eval.heap().alloc(Args::new()))
    }
}

impl Actions {
    /// Creates a new `Actions` accessor instance.
    pub fn new() -> Self {
        Self
    }
}
