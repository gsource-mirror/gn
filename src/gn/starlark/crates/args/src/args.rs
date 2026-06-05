// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::{cell::UnsafeCell, fmt, fmt::Display};

use allocative::Allocative;
use starlark::{
    environment::{Methods, MethodsBuilder, MethodsStatic},
    eval::Evaluator,
    starlark_complex_value,
    values::{
        Coerce, Freeze, FreezeResult, Freezer, FrozenValue, ProvidesStaticType, StarlarkValue,
        Trace, Tracer, Value, ValueLike,
    },
};
use starlark_derive::{starlark_module, starlark_value, NoSerialize};

use crate::{formatter::Formatter, Error};

/// Internal representation of individual arguments stored in `Args`.
#[derive(Debug, Clone, Trace, Coerce, ProvidesStaticType, NoSerialize, Allocative)]
#[repr(C)]
pub enum ArgValue<V> {
    Scalar {
        arg_name: Option<String>,
        value: V,
        format: Option<Formatter>,
    },
    All {
        flag: Option<String>,
        values: V,
        map_each: Option<V>,
        format_each: Option<Formatter>,
        before_each: Option<String>,
        terminate_with: Option<String>,
        omit_if_empty: bool,
        uniquify: bool,
    },
    Joined {
        flag: Option<String>,
        values: V,
        join_with: String,
        map_each: Option<V>,
        format_each: Option<Formatter>,
        format_joined: Option<Formatter>,
        omit_if_empty: bool,
        uniquify: bool,
    },
}

impl ArgValue<FrozenValue> {
    pub fn to_value<'v>(&self) -> ArgValue<Value<'v>> {
        match self {
            ArgValue::Scalar {
                arg_name,
                value,
                format,
            } => ArgValue::Scalar {
                arg_name: arg_name.clone(),
                value: value.to_value(),
                format: format.clone(),
            },
            ArgValue::All {
                flag,
                values,
                map_each,
                format_each,
                before_each,
                terminate_with,
                omit_if_empty,
                uniquify,
            } => ArgValue::All {
                flag: flag.clone(),
                values: values.to_value(),
                map_each: map_each.map(|m| m.to_value()),
                format_each: format_each.clone(),
                before_each: before_each.clone(),
                terminate_with: terminate_with.clone(),
                omit_if_empty: *omit_if_empty,
                uniquify: *uniquify,
            },
            ArgValue::Joined {
                flag,
                values,
                join_with,
                map_each,
                format_each,
                format_joined,
                omit_if_empty,
                uniquify,
            } => ArgValue::Joined {
                flag: flag.clone(),
                values: values.to_value(),
                join_with: join_with.clone(),
                map_each: map_each.map(|m| m.to_value()),
                format_each: format_each.clone(),
                format_joined: format_joined.clone(),
                omit_if_empty: *omit_if_empty,
                uniquify: *uniquify,
            },
        }
    }
}

impl Freeze for ArgValue<Value<'_>> {
    type Frozen = ArgValue<FrozenValue>;

    fn freeze(self, freezer: &Freezer) -> FreezeResult<Self::Frozen> {
        match self {
            ArgValue::Scalar {
                arg_name,
                value,
                format,
            } => Ok(ArgValue::Scalar {
                arg_name,
                value: value.freeze(freezer)?,
                format: format.freeze(freezer)?,
            }),
            ArgValue::All {
                flag,
                values,
                map_each,
                format_each,
                before_each,
                terminate_with,
                omit_if_empty,
                uniquify,
            } => Ok(ArgValue::All {
                flag,
                values: values.freeze(freezer)?,
                map_each: map_each.map(|m| m.freeze(freezer)).transpose()?,
                format_each: format_each.freeze(freezer)?,
                before_each,
                terminate_with,
                omit_if_empty,
                uniquify,
            }),
            ArgValue::Joined {
                flag,
                values,
                join_with,
                map_each,
                format_each,
                format_joined,
                omit_if_empty,
                uniquify,
            } => Ok(ArgValue::Joined {
                flag,
                values: values.freeze(freezer)?,
                join_with,
                map_each: map_each.map(|m| m.freeze(freezer)).transpose()?,
                format_each: format_each.freeze(freezer)?,
                format_joined: format_joined.freeze(freezer)?,
                omit_if_empty,
                uniquify,
            }),
        }
    }
}

/// Generic representation of `Args` which can hold mutable or frozen Starlark
/// values.
#[derive(Debug, ProvidesStaticType, NoSerialize, Allocative)]
#[repr(C)]
pub struct ArgsGen<V> {
    /// List of arguments added to the builder.
    #[allocative(skip)]
    pub arguments: UnsafeCell<Vec<ArgValue<V>>>,
}

unsafe impl<'v, V: ValueLike<'v>> Trace<'v> for ArgsGen<V>
where
    Self: ProvidesStaticType<'v>,
{
    fn trace(&mut self, tracer: &Tracer<'v>) {
        // Safety: Starlark garbage collection tracing is single-threaded and runs when
        // no other active borrows to the arguments vector exist.
        for arg in unsafe { self.borrow_mut() }.iter_mut() {
            arg.trace(tracer);
        }
    }
}

unsafe impl<From, To> Coerce<ArgsGen<To>> for ArgsGen<From> where From: Coerce<To> {}

// The Starlark `Args` object used to construct command lines for actions.
starlark_complex_value!(pub Args);
/// Type alias for the frozen version of `ArgsGen`.
pub type ArgsGenFrozen = FrozenArgs;

impl<V> ArgsGen<V> {
    // Safety: Caller must ensure that Starlark evaluation is single-threaded,
    // and no other borrows to this vector are active.
    #[allow(clippy::mut_from_ref)]
    pub(crate) unsafe fn borrow_mut(&self) -> &mut Vec<ArgValue<V>> {
        // Safety: Caller ensures this is safe.
        unsafe { &mut *self.arguments.get() }
    }
}

impl ArgsGen<FrozenValue> {
    /// Immutably borrows the arguments vector.
    /// This is safe because frozen values are immutable and thread-safe to
    /// share concurrently.
    pub(crate) fn borrow(&self) -> &Vec<ArgValue<FrozenValue>> {
        // Safety: Since the value is frozen, no mutable borrows can occur and the
        // vector is immutable.
        unsafe { &*self.arguments.get() }
    }
}

impl<V> Default for ArgsGen<V> {
    fn default() -> Self {
        Self {
            arguments: UnsafeCell::new(Vec::new()),
        }
    }
}

impl<'v, V: ValueLike<'v>> Display for ArgsGen<V>
where
    Self: ProvidesStaticType<'v>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "args")
    }
}

#[starlark_value(type = "Args")]
impl<'v, V: ValueLike<'v>> StarlarkValue<'v> for ArgsGen<V>
where
    Self: ProvidesStaticType<'v>,
{
    fn get_methods() -> Option<&'static Methods> {
        static RES: MethodsStatic = MethodsStatic::new("Args", |builder| {
            args_methods(builder);
        });
        Some(RES.methods())
    }
}

impl Freeze for Args<'_> {
    type Frozen = FrozenArgs;

    fn freeze(self, freezer: &Freezer) -> FreezeResult<Self::Frozen> {
        let frozen_arguments = self
            .arguments
            .into_inner()
            .into_iter()
            .map(|arg| arg.freeze(freezer))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(FrozenArgs {
            arguments: UnsafeCell::new(frozen_arguments),
        })
    }
}

// Safety: ArgsGen interior mutability is strictly single-threaded during
// Starlark evaluation. Once frozen, it is read-only and safe to share.
unsafe impl<V: Send> Send for ArgsGen<V> {}
unsafe impl<V: Sync> Sync for ArgsGen<V> {}

// Inline it to ensure that the error doesn't actually need to be passed as a
// function parameter.
#[inline]
fn arg_name_and_value<'v>(
    arg_name_or_value: Value<'v>,
    value: Option<Value<'v>>,
    err: Error,
) -> starlark::Result<(Option<String>, Value<'v>)> {
    if let Some(val) = value {
        if let Some(arg_name) = arg_name_or_value.unpack_str() {
            Ok((Some(arg_name.to_owned()), val))
        } else {
            Err(starlark::Error::from(err))
        }
    } else {
        Ok((None, arg_name_or_value))
    }
}

fn get_mutable_args<'v>(this: Value<'v>) -> starlark::Result<&'v mut Vec<ArgValue<Value<'v>>>> {
    if let Some(args) = this.downcast_ref::<ArgsGen<Value<'v>>>() {
        // Safety: Starlark evaluation is strictly single-threaded, and this method is
        // only called on mutable (non-frozen) Args objects during evaluation.
        Ok(unsafe { args.borrow_mut() })
    } else if this.downcast_ref::<ArgsGen<FrozenValue>>().is_some() {
        Err(starlark::Error::new_other(Error::CannotMutateFrozenArgs))
    } else {
        Err(Error::ExpectedArgsObject.into())
    }
}

/// Registers the Starlark methods of the `Args` class (`add`, `add_all`,
/// `add_joined`).
#[starlark_module]
pub fn args_methods(builder: &mut MethodsBuilder) {
    fn add<'v>(
        this: Value<'v>,
        arg_name_or_value: Value<'v>,
        value: Option<Value<'v>>,
        #[starlark(require = named)] format: Option<Formatter>,
    ) -> starlark::Result<Value<'v>> {
        let args = get_mutable_args(this)?;

        let (arg_name, val) =
            arg_name_and_value(arg_name_or_value, value, Error::ExpectedAddStringFlag)?;

        args.push(ArgValue::Scalar {
            arg_name,
            value: val,
            format,
        });
        Ok(this)
    }

    fn add_all<'v>(
        this: Value<'v>,
        arg_name_or_values: Value<'v>,
        values: Option<Value<'v>>,
        #[starlark(require = named)] map_each: Option<Value<'v>>,
        #[starlark(require = named)] format_each: Option<Formatter>,
        #[starlark(require = named)] before_each: Option<&str>,
        #[starlark(require = named)] terminate_with: Option<&str>,
        #[starlark(require = named, default = true)] omit_if_empty: bool,
        #[starlark(require = named, default = false)] uniquify: bool,
        #[starlark(require = named)] allow_closure: Option<bool>,
    ) -> starlark::Result<Value<'v>> {
        // See a comment on the error message for more details on why this is needed.
        if map_each.is_some() && allow_closure.is_none() {
            return Err(Error::MapEachRequiresAllowClosure.into());
        }

        let args = get_mutable_args(this)?;

        let (flag, values) =
            arg_name_and_value(arg_name_or_values, values, Error::ExpectedAddAllStringFlag)?;

        args.push(ArgValue::All {
            flag,
            values,
            map_each,
            format_each,
            before_each: before_each.map(String::from),
            terminate_with: terminate_with.map(String::from),
            omit_if_empty,
            uniquify,
        });
        Ok(this)
    }

    fn add_joined<'v>(
        this: Value<'v>,
        arg_name_or_values: Value<'v>,
        values: Option<Value<'v>>,
        #[starlark(require = named)] join_with: &str,
        #[starlark(require = named)] map_each: Option<Value<'v>>,
        #[starlark(require = named)] format_each: Option<Formatter>,
        #[starlark(require = named)] format_joined: Option<Formatter>,
        #[starlark(require = named, default = true)] omit_if_empty: bool,
        #[starlark(require = named, default = false)] uniquify: bool,
        #[starlark(require = named)] allow_closure: Option<bool>,
    ) -> starlark::Result<Value<'v>> {
        // See a comment on the error message for more details on why this is needed.
        if map_each.is_some() && allow_closure.is_none() {
            return Err(Error::MapEachRequiresAllowClosure.into());
        }

        let args = get_mutable_args(this)?;

        let (flag, values) = arg_name_and_value(
            arg_name_or_values,
            values,
            Error::ExpectedAddJoinedStringFlag,
        )?;

        args.push(ArgValue::Joined {
            flag,
            values,
            join_with: join_with.to_owned(),
            map_each,
            format_each,
            format_joined,
            omit_if_empty,
            uniquify,
        });
        Ok(this)
    }
}

impl<'v> ArgsGen<FrozenValue> {
    /// Expands the stored arguments list into command-line arguments and input
    /// files.
    pub fn expand(&self, eval: &mut Evaluator<'v, '_, '_>) -> starlark::Result<Vec<String>> {
        let mut command = Vec::new();
        crate::expand::expand_into(&mut command, self, eval)?;
        Ok(command)
    }
}
