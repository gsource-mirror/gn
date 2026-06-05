// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use allocative::Allocative;
use starlark::collections::SmallMap;
use starlark::values::{
    Coerce, Freeze, FreezeResult, Freezer, Heap, ProvidesStaticType, StarlarkValue, Trace, Value,
    ValueLike,
};
use starlark_derive::{starlark_value, NoSerialize};
use std::fmt::{self, Display, Formatter};

use starlark::starlark_complex_value;

#[derive(Debug, Trace, Coerce, ProvidesStaticType, NoSerialize, Allocative)]
#[repr(C)]
pub struct CtxAttrGen<V> {
    attrs: SmallMap<String, V>,
}

starlark_complex_value!(pub CtxAttr);

impl<'v, V: ValueLike<'v>> Display for CtxAttrGen<V>
where
    Self: ProvidesStaticType<'v>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "ctx.attr")
    }
}

#[starlark_value(type = "ctx_attr")]
impl<'v, V: ValueLike<'v>> StarlarkValue<'v> for CtxAttrGen<V>
where
    Self: ProvidesStaticType<'v>,
{
    fn get_attr(&self, attribute: &str, _heap: Heap<'v>) -> Option<Value<'v>> {
        self.attrs.get(attribute).map(|v| v.to_value())
    }

    fn dir_attr(&self) -> Vec<String> {
        self.attrs.keys().cloned().collect()
    }
}

impl<'v> Freeze for CtxAttr<'v> {
    type Frozen = FrozenCtxAttr;
    fn freeze(self, freezer: &Freezer) -> FreezeResult<Self::Frozen> {
        let attrs = self.attrs.freeze(freezer)?;
        Ok(CtxAttrGen { attrs })
    }
}

impl<V> CtxAttrGen<V> {
    pub fn new(attrs: SmallMap<String, V>) -> Self {
        Self { attrs }
    }
}
