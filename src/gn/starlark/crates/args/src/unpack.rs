// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use either::Either;
use starlark::{
    eval::Evaluator,
    typing::Ty,
    values::{type_repr::StarlarkTypeRepr, UnpackValue, Value, ValueTyped},
};

use crate::{args::Args, formatter::Formatter};

impl<'v> UnpackValue<'v> for Formatter {
    type Error = starlark::Error;

    fn unpack_value_impl(value: Value<'v>) -> Result<Option<Self>, Self::Error> {
        let s = <&'v str>::unpack_value_err(value)?;
        Self::new(s).map(Some)
    }
}

pub struct ArgsSequence<'v>(pub Vec<Either<&'v str, ValueTyped<'v, Args<'v>>>>);

impl<'v> StarlarkTypeRepr for ArgsSequence<'v> {
    type Canonical = starlark::values::Value<'v>;

    fn starlark_type_repr() -> Ty {
        Ty::list(Ty::any())
    }
}

impl<'v> UnpackValue<'v> for ArgsSequence<'v> {
    type Error = starlark::Error;

    fn unpack_value_impl(value: Value<'v>) -> Result<Option<Self>, Self::Error> {
        let list = <starlark::values::list::UnpackList<Either<&'v str, ValueTyped<'v, Args<'v>>>>>::unpack_value(value)?;
        Ok(list.map(|l| ArgsSequence(l.items)))
    }
}

impl<'v> ArgsSequence<'v> {
    pub fn expand(&self, eval: &mut Evaluator<'v, '_, '_>) -> starlark::Result<Vec<String>> {
        let mut command = Vec::new();
        for item in &self.0 {
            match item {
                Either::Left(s) => command.push((*s).to_owned()),
                Either::Right(args) => {
                    crate::expand::expand_into(&mut command, args, eval)?;
                },
            }
        }
        Ok(command)
    }
}
