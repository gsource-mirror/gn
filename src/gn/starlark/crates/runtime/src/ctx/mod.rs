// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

pub mod actions;
pub mod gn;

use std::fmt::{self, Display, Formatter};

use allocative::Allocative;
use starlark::environment::{Methods, MethodsBuilder, MethodsStatic};
use starlark::starlark_complex_value;
use starlark::values::Heap;
use starlark::values::{ProvidesStaticType, StarlarkValue, Trace, Tracer, Value, ValueLike};
use starlark_derive::{starlark_module, starlark_value, NoSerialize};

use self::actions::Actions;
use self::gn::Gn;
pub type CtxState = types::CtxState<crate::TargetRef>;

/// Generic representation of rule context `ctx` which can hold mutable or frozen Starlark values.
#[derive(Debug, ProvidesStaticType, NoSerialize, Allocative)]
#[repr(C)]
pub struct CtxGen<V> {
    actions: V,
    attr: V,
    file: V,
    files: V,
    gn: V,
}

unsafe impl<'v, V: Trace<'v>> Trace<'v> for CtxGen<V> {
    fn trace(&mut self, tracer: &Tracer<'v>) {
        self.actions.trace(tracer);
        self.attr.trace(tracer);
        self.file.trace(tracer);
        self.files.trace(tracer);
        self.gn.trace(tracer);
    }
}

unsafe impl<FromV: starlark::coerce::Coerce<ToV>, ToV> starlark::coerce::Coerce<CtxGen<ToV>>
    for CtxGen<FromV>
{
}

// The Starlark `ctx` object passed to rule implementation functions.
starlark_complex_value!(pub Ctx);

impl<'v, V: ValueLike<'v>> Display for CtxGen<V>
where
    Self: ProvidesStaticType<'v>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "ctx")
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

crate::cannot_freeze!(Ctx<'v>, FrozenCtx);

/// Registers the Starlark attributes and methods of the `ctx` object.
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
    fn gn<'v>(this: &Ctx<'v>) -> starlark::Result<Value<'v>> {
        Ok(this.gn.to_value())
    }
}

impl<'v> CtxGen<Value<'v>> {
    /// Creates a new `Ctx` object from resolved target attributes.
    pub fn new(attr: Value<'v>, file: Value<'v>, files: Value<'v>, heap: &Heap<'v>) -> Self {
        Self {
            actions: heap.alloc(Actions::new()),
            attr,
            file,
            files,
            gn: heap.alloc(Gn::new()),
        }
    }
}
