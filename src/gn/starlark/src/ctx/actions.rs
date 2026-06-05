// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use allocative::Allocative;
use starlark::environment::{Methods, MethodsBuilder, MethodsStatic};
use starlark::eval::Evaluator;
use starlark::values::{
    Coerce, Freeze, FreezeResult, Freezer, ProvidesStaticType, StarlarkValue, Trace, Value,
};
use starlark_derive::{starlark_module, starlark_value, NoSerialize};
use std::fmt::{self, Display, Formatter};

use starlark::starlark_complex_value;




use starlark::collections::SmallMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Trace, Coerce, ProvidesStaticType, NoSerialize, Allocative)]
#[repr(C)]
pub struct ActionsGen<V> {
    pub(crate) declared_outputs: std::cell::RefCell<SmallMap<String, PathBuf>>,
    pub(crate) output_dir: String,
    phantom: std::marker::PhantomData<V>,
}

unsafe impl<V> Send for ActionsGen<V> {}
unsafe impl<V> Sync for ActionsGen<V> {}

starlark_complex_value!(pub Actions);

impl<'v, V: starlark::values::ValueLike<'v>> Display for ActionsGen<V>
where
    Self: ProvidesStaticType<'v>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "actions")
    }
}

#[starlark_value(type = "actions")]
impl<'v, V: starlark::values::ValueLike<'v>> StarlarkValue<'v> for ActionsGen<V>
where
    Self: ProvidesStaticType<'v>,
{
    fn get_methods() -> Option<&'static Methods> {
        static RES: MethodsStatic = MethodsStatic::new("actions", actions_methods);
        Some(RES.methods())
    }
}

impl<'v> Freeze for Actions<'v> {
    type Frozen = FrozenActions;
    fn freeze(self, _freezer: &Freezer) -> FreezeResult<Self::Frozen> {
        Ok(ActionsGen {
            declared_outputs: std::cell::RefCell::new(self.declared_outputs.into_inner()),
            output_dir: self.output_dir,
            phantom: std::marker::PhantomData,
        })
    }
}

#[starlark_module]
pub fn actions_methods(builder: &mut MethodsBuilder) {
    fn declare_file<'v>(
        this: &Actions<'v>,
        filename: &str,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        let path = std::path::Path::new(&this.output_dir).join(filename);
        let mut declared_outputs = this.declared_outputs.borrow_mut();
        if declared_outputs.contains_key(filename) {
            return Err(starlark::Error::new_other(crate::errors::Error::FileAlreadyDeclared(
                filename.to_owned()
            )));
        }
        declared_outputs.insert(filename.to_owned(), path.clone());
        let static_path: &'static Path = Box::leak(path.into_boxed_path());
        Ok(eval.heap().alloc(crate::file::File(static_path)))
    }

    fn run<'v>(
        this: &Actions<'v>,
        outputs: Value<'v>,
        inputs: Value<'v>,
        executable: Value<'v>,
        arguments: Option<Value<'v>>,
        env: Option<Value<'v>>,
    ) -> starlark::Result<Value<'v>> {
        let _ = (this, outputs, inputs, executable, arguments, env);
        todo!()
    }

    fn run_shell<'v>(
        this: &Actions<'v>,
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
        this: &Actions<'v>,
        output: Value<'v>,
        content: Value<'v>,
        #[starlark(default = false)] is_executable: bool,
    ) -> starlark::Result<Value<'v>> {
        let _ = (this, output, content, is_executable);
        todo!()
    }

    fn args<'v>(
        this: &Actions<'v>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        let _ = this;
        Ok(eval.heap().alloc(super::args::Args::new()))
    }
}

impl<V> ActionsGen<V> {
    pub fn new(output_dir: String) -> Self {
        Self {
            declared_outputs: std::cell::RefCell::new(SmallMap::new()),
            output_dir,
            phantom: std::marker::PhantomData,
        }
    }
}
