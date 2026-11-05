// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::convert::Infallible;

use allocative::Allocative;
use starlark::{
    any::ProvidesStaticType,
    eval::Evaluator,
    typing::Ty,
    values::{
        Freeze, FreezeResult, Freezer, StringValue, UnpackValue, Value, ValueTyped,
        list::{ListRef, UnpackList},
        type_repr::StarlarkTypeRepr,
    },
};

use crate::{
    ArgVisitor,
    args::{Args, FrozenArgs},
    expand::expand_args,
};

/// Starlark value for Args | str.
/// It is possible to write the following code. When doing so, bazel
/// does *not* add foo to the command-line.
/// ```
/// args = ctx.actions.args()
/// ctx.actions.run(..., arguments = [args])
/// args.add("--foo")
/// ```
/// To match this behaviour, if an args is provided, we take a copy of args rather than a reference.
/// As an optimization, however, since args is append-only, (reference, size) is equivalent to a
/// copy.
#[derive(Allocative, Clone, Debug, PartialEq, Eq, ProvidesStaticType)]
pub enum ArgsLike<'v> {
    /// A string.
    String(StringValue<'v>),
    /// A slice of an args object (`args[..upto]`).
    Args(ValueTyped<'v, Args<'v>>, usize),
    /// A slice of a frozen args object (`args[..upto]`).
    FrozenArgs(ValueTyped<'v, FrozenArgs<'v>>, usize),
}

// Safety: ArgsLike with 'static lifetime only references frozen/immutable data.
unsafe impl Sync for ArgsLike<'static> {}

impl<'v> Freeze<'v> for ArgsLike<'v> {
    type Frozen<'fv> = ArgsLike<'fv>;

    fn freeze<'fv>(self, freezer: &Freezer<'v, 'fv>) -> FreezeResult<Self::Frozen<'fv>> {
        Ok(match self {
            Self::String(s) => ArgsLike::String(s.freeze(freezer)?),
            Self::Args(args, upto) => ArgsLike::FrozenArgs(args.freeze(freezer)?, upto),
            Self::FrozenArgs(args, upto) => ArgsLike::FrozenArgs(args.freeze(freezer)?, upto),
        })
    }
}

impl<'v> StarlarkTypeRepr for ArgsLike<'v> {
    type Canonical = Value<'v>;

    fn starlark_type_repr() -> Ty {
        Ty::union2(Args::starlark_type_repr(), Ty::string())
    }
}

impl<'v> UnpackValue<'v> for ArgsLike<'v> {
    type Error = Infallible;

    fn unpack_value_impl(value: Value<'v>) -> Result<Option<Self>, Self::Error> {
        if let Some(s) = StringValue::new(value) {
            Ok(Some(Self::String(s)))
        } else if let Some(args) = ValueTyped::<Args<'v>>::new(value) {
            Ok(Some(Self::Args(args, args.arguments.borrow().len())))
        } else if let Some(args) = ValueTyped::<FrozenArgs<'v>>::new(value) {
            Ok(Some(Self::FrozenArgs(args, args.arguments.len())))
        } else {
            Ok(None)
        }
    }
}

/// Starlark value for list[Args | str].
#[derive(Allocative, Debug, ProvidesStaticType)]
pub struct ArgsList<'v>(Vec<ArgsLike<'v>>);

impl<'v> StarlarkTypeRepr for ArgsList<'v> {
    type Canonical = <UnpackList<ArgsLike<'v>> as StarlarkTypeRepr>::Canonical;

    fn starlark_type_repr() -> Ty {
        UnpackList::<ArgsLike<'v>>::starlark_type_repr()
    }
}

impl<'v> UnpackValue<'v> for ArgsList<'v> {
    type Error = Infallible;

    fn unpack_value_impl(value: Value<'v>) -> Result<Option<Self>, Self::Error> {
        let Some(list) = <&ListRef>::unpack_value_opt(value) else {
            return Ok(None);
        };
        Ok(list
            .iter()
            .map(ArgsLike::unpack_value_opt)
            .collect::<Option<_>>()
            .map(Self))
    }
}

impl<'v> ArgsList<'v> {
    /// Expands the arguments list into the given visitor.
    pub fn expand(
        &self,
        eval: &mut Evaluator<'v, '_, '_>,
        visitor: &mut impl ArgVisitor,
    ) -> starlark::Result<()> {
        for arg in &self.0 {
            match arg {
                ArgsLike::String(s) => visitor.add_arg(s.as_str()),
                ArgsLike::Args(..) => {
                    unreachable!("expand never be called on a non-frozen ArgsList")
                },
                ArgsLike::FrozenArgs(args, upto) => {
                    expand_args(&args.arguments[..*upto], eval, visitor)?;
                },
            }
        }
        Ok(())
    }
}
