// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use allocative::Allocative;
use starlark::values::{Value, type_repr::StarlarkTypeRepr, UnpackValue};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Allocative)]
pub enum AttrCfg {
    CurrentToolchain,
}

impl StarlarkTypeRepr for AttrCfg {
    type Canonical = String;

    fn starlark_type_repr() -> starlark::typing::Ty {
        String::starlark_type_repr()
    }
}

impl<'v> UnpackValue<'v> for AttrCfg {
    type Error = starlark::Error;

    fn unpack_value_impl(value: Value<'v>) -> Result<Option<Self>, Self::Error> {
        match value.unpack_str() {
            Some("target") => Ok(Some(AttrCfg::CurrentToolchain)),
            Some(s) => Err(crate::errors::Error::ConfigTransitionNotImplemented(s.to_owned()).into()),
            None => Ok(None),
        }
    }
}

