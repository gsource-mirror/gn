// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

pub mod actions;
pub mod args;
pub mod attr;
pub mod files;
pub mod gn;

use allocative::Allocative;
use starlark::environment::{Methods, MethodsBuilder, MethodsStatic};
use starlark::values::{
    Coerce, Freeze, FreezeResult, Freezer, ProvidesStaticType, StarlarkValue, Trace, Value,
    ValueLike,
};
use starlark_derive::{starlark_module, starlark_value, NoSerialize};
use std::fmt::{self, Display, Formatter};

use starlark::starlark_complex_value;
use crate::label::Label;

#[derive(Debug, Trace, Coerce, ProvidesStaticType, NoSerialize, Allocative)]
#[repr(C)]
pub struct CtxGen<V> {
    actions: V,
    attr: V,
    file: V,
    files: V,
    target: V,
    gn: V,
}

starlark_complex_value!(pub Ctx);

impl<'v, V: ValueLike<'v>> Display for CtxGen<V>
where
    Self: ProvidesStaticType<'v>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let target_ref = self.target.to_value().downcast_ref::<crate::target::TargetRef>().unwrap();
        write!(f, "ctx(label={})", target_ref.label())
    }
}

#[starlark_value(type = "ctx")]
impl<'v, V: ValueLike<'v>> StarlarkValue<'v> for CtxGen<V>
where
    Self: ProvidesStaticType<'v>,
{
    fn get_methods() -> Option<&'static Methods> {
        static RES: MethodsStatic = MethodsStatic::new("ctx", ctx_methods);
        Some(RES.methods())
    }
}

impl<'v> Freeze for Ctx<'v> {
    type Frozen = FrozenCtx;
    fn freeze(self, freezer: &Freezer) -> FreezeResult<Self::Frozen> {
        let actions = self.actions.freeze(freezer)?;
        let attr = self.attr.freeze(freezer)?;
        let file = self.file.freeze(freezer)?;
        let files = self.files.freeze(freezer)?;
        let target = self.target.freeze(freezer)?;
        let gn = self.gn.freeze(freezer)?;
        Ok(CtxGen {
            actions,
            attr,
            file,
            files,
            target,
            gn,
        })
    }
}

#[starlark_module]
pub fn ctx_methods(builder: &mut MethodsBuilder) {
    #[starlark(attribute)]
    fn actions<'v>(this: &Ctx<'v>) -> starlark::Result<Value<'v>> {
        Ok(this.actions.to_value())
    }

    #[starlark(attribute)]
    fn attr<'v>(this: &Ctx<'v>) -> starlark::Result<Value<'v>> {
        Ok(this.attr.to_value())
    }

    #[starlark(attribute)]
    fn file<'v>(this: &Ctx<'v>) -> starlark::Result<Value<'v>> {
        Ok(this.file.to_value())
    }

    #[starlark(attribute)]
    fn files<'v>(this: &Ctx<'v>) -> starlark::Result<Value<'v>> {
        Ok(this.files.to_value())
    }

    #[starlark(attribute)]
    fn label<'v>(this: &Ctx<'v>) -> starlark::Result<Label> {
        let target_ref = this.target.to_value().downcast_ref::<crate::target::TargetRef>().unwrap();
        Ok(target_ref.label().to_owned())
    }

    #[starlark(attribute)]
    fn target<'v>(this: &Ctx<'v>) -> starlark::Result<Value<'v>> {
        Ok(this.target.to_value())
    }

    #[starlark(attribute)]
    fn gn<'v>(this: &Ctx<'v>) -> starlark::Result<Value<'v>> {
        Ok(this.gn.to_value())
    }
}

impl<V> CtxGen<V> {
    pub fn new(actions: V, attr: V, file: V, files: V, target: V, gn: V) -> Self {
        Self {
            actions,
            attr,
            file,
            files,
            target,
            gn,
        }
    }
}
