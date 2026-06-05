// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::cell::RefCell;
use std::fmt;
use std::fmt::Display;

use allocative::Allocative;
use starlark::environment::GlobalsBuilder;
use starlark::environment::Methods;
use starlark::environment::MethodsBuilder;
use starlark::environment::MethodsStatic;
use starlark::eval::Evaluator;
use starlark::starlark_complex_value;
use starlark::typing::Ty;
use starlark::values::type_repr::StarlarkTypeRepr;
use starlark::values::Coerce;
use starlark::values::Freeze;
use starlark::values::FreezeResult;
use starlark::values::Freezer;
use starlark::values::FrozenValue;
use starlark::values::ProvidesStaticType;
use starlark::values::StarlarkValue;
use starlark::values::Trace;
use starlark::values::Tracer;
use starlark::values::UnpackValue;
use starlark::values::Value;
use starlark::values::ValueLike;
use starlark::values::ValueTyped;
use starlark_derive::starlark_module;
use starlark_derive::starlark_value;
use starlark_derive::NoSerialize;
use types::File;

use crate::errors::Error;

/// Helper to format argument values using a template containing `%s`.
#[derive(Debug, Clone, Allocative)]
pub struct Formatter {
    before: String,
    after: String,
}

impl Formatter {
    /// Parses a format string and returns a `Formatter` if valid (must contain exactly one `%s`).
    pub fn new(fmt: &str) -> starlark::Result<Self> {
        let mut split = fmt.split("%s");
        match (split.next(), split.next(), split.next()) {
            (Some(before), Some(after), None) => Ok(Self {
                before: before.to_owned(),
                after: after.to_owned(),
            }),
            _ => Err(Error::InvalidFormatString(fmt.to_owned()).into()),
        }
    }

    /// Formats the string by replacing `%s` with the input string.
    pub fn format(&self, s: &str) -> String {
        format!("{}{}{}", self.before, s, self.after)
    }
}

impl Freeze for Formatter {
    type Frozen = Self;
    fn freeze(self, _freezer: &Freezer) -> FreezeResult<Self::Frozen> {
        Ok(self)
    }
}

impl StarlarkTypeRepr for Formatter {
    type Canonical = String;

    fn starlark_type_repr() -> Ty {
        String::starlark_type_repr()
    }
}

impl<'v> UnpackValue<'v> for Formatter {
    type Error = starlark::Error;

    fn unpack_value_impl(value: Value<'v>) -> Result<Option<Self>, Self::Error> {
        let s = <&'v str>::unpack_value_err(value)?;
        Self::new(s).map(Some)
    }
}

/// Internal representation of individual arguments stored in `Args`.
#[derive(Debug, Clone, Trace, Coerce, ProvidesStaticType, NoSerialize, Allocative)]
#[repr(C)]
pub enum ArgValue<V> {
    Scalar {
        arg_name: Option<String>,
        value: Option<V>,
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
                value: value.map(|v| v.freeze(freezer)).transpose()?,
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

/// Generic representation of `Args` which can hold mutable or frozen Starlark values.
#[derive(Debug, ProvidesStaticType, NoSerialize, Allocative)]
#[repr(C)]
pub struct ArgsGen<V> {
    /// List of arguments added to the builder.
    pub arguments: RefCell<Vec<ArgValue<V>>>,
}

unsafe impl<'v, V: ValueLike<'v>> Trace<'v> for ArgsGen<V>
where
    Self: ProvidesStaticType<'v>,
{
    fn trace(&mut self, tracer: &Tracer<'v>) {
        if let Ok(mut args) = self.arguments.try_borrow_mut() {
            for arg in args.iter_mut() {
                arg.trace(tracer);
            }
        }
    }
}

unsafe impl<From, To> Coerce<ArgsGen<To>> for ArgsGen<From> where From: Coerce<To> {}

// The Starlark `Args` object used to construct command lines for actions.
starlark_complex_value!(pub Args);
/// Type alias for the frozen version of `ArgsGen`.
pub type ArgsGenFrozen = FrozenArgs;

impl<V> Default for ArgsGen<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V> ArgsGen<V> {
    /// Creates a new empty `ArgsGen` builder.
    pub fn new() -> Self {
        Self {
            arguments: RefCell::new(Vec::new()),
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
            args_test_methods(builder);
        });
        Some(RES.methods())
    }
}

impl Freeze for Args<'_> {
    type Frozen = FrozenArgs;
    fn freeze(self, freezer: &Freezer) -> FreezeResult<Self::Frozen> {
        let arguments = self.arguments.into_inner();
        let mut frozen_arguments = Vec::with_capacity(arguments.len());
        for arg in arguments {
            frozen_arguments.push(arg.freeze(freezer)?);
        }
        Ok(FrozenArgs {
            arguments: RefCell::new(frozen_arguments),
        })
    }
}

// Safety: ArgsGen interior mutability is strictly single-threaded during Starlark
// evaluation, and thread-safe read-only during action execution.
unsafe impl<V> Send for ArgsGen<V> {}
unsafe impl<V> Sync for ArgsGen<V> {}
fn arg_name_and_value<'v>(
    arg_name_or_value: Value<'v>,
    value: Option<Value<'v>>,
) -> Option<(Option<String>, Option<Value<'v>>)> {
    if let Some(arg_name) = arg_name_or_value.unpack_str() {
        Some((Some(arg_name.to_owned()), value))
    } else if value.is_some() {
        None
    } else {
        Some((None, Some(arg_name_or_value)))
    }
}

/// Registers the Starlark methods of the `Args` class (`add`, `add_all`, `add_joined`).
#[starlark_module]
pub fn args_methods(builder: &mut MethodsBuilder) {
    fn add<'v>(
        this: ValueTyped<'v, Args<'v>>,
        arg_name_or_value: Value<'v>,
        value: Option<Value<'v>>,
        #[starlark(require = named)] format: Option<&str>,
    ) -> starlark::Result<ValueTyped<'v, Args<'v>>> {
        let mut args = this.arguments.borrow_mut();

        let (arg_name, val) = arg_name_and_value(arg_name_or_value, value)
            .ok_or_else(|| starlark::Error::from(Error::ExpectedAddStringFlag))?;

        if !val.is_some_and(|v| v.is_none()) {
            args.push(ArgValue::Scalar {
                arg_name,
                value: val,
                format: format.map(Formatter::new).transpose()?,
            });
        }
        Ok(this)
    }

    fn add_all<'v>(
        this: ValueTyped<'v, Args<'v>>,
        arg_name_or_values: Value<'v>,
        values: Option<Value<'v>>,
        #[starlark(require = named)] map_each: Option<Value<'v>>,
        #[starlark(require = named)] format_each: Option<Formatter>,
        #[starlark(require = named)] before_each: Option<&str>,
        #[starlark(require = named)] terminate_with: Option<&str>,
        #[starlark(require = named, default = true)] omit_if_empty: bool,
        #[starlark(require = named, default = false)] uniquify: bool,
        #[starlark(require = named, default = true)] expand_directories: bool,
    ) -> starlark::Result<ValueTyped<'v, Args<'v>>> {
        let _ = expand_directories;

        let (flag, values_to_add) = arg_name_and_value(arg_name_or_values, values)
            .ok_or_else(|| starlark::Error::from(Error::ExpectedAddAllStringFlag))?;

        if let Some(vals) = values_to_add {
            if !vals.is_none() {
                this.arguments.borrow_mut().push(ArgValue::All {
                    flag,
                    values: vals,
                    map_each,
                    format_each,
                    before_each: before_each.map(String::from),
                    terminate_with: terminate_with.map(String::from),
                    omit_if_empty,
                    uniquify,
                });
            }
        }
        Ok(this)
    }

    fn add_joined<'v>(
        this: ValueTyped<'v, Args<'v>>,
        arg_name_or_values: Value<'v>,
        values: Option<Value<'v>>,
        #[starlark(require = named)] join_with: &str,
        #[starlark(require = named)] map_each: Option<Value<'v>>,
        #[starlark(require = named)] format_each: Option<&str>,
        #[starlark(require = named)] format_joined: Option<&str>,
        #[starlark(require = named, default = true)] omit_if_empty: bool,
        #[starlark(require = named, default = false)] uniquify: bool,
        #[starlark(require = named, default = true)] expand_directories: bool,
    ) -> starlark::Result<ValueTyped<'v, Args<'v>>> {
        let _ = expand_directories;

        let (flag, values_to_add) = arg_name_and_value(arg_name_or_values, values)
            .ok_or_else(|| starlark::Error::from(Error::ExpectedAddJoinedStringFlag))?;

        if let Some(vals) = values_to_add {
            if !vals.is_none() {
                this.arguments.borrow_mut().push(ArgValue::Joined {
                    flag,
                    values: vals,
                    join_with: join_with.to_owned(),
                    map_each,
                    format_each: format_each.map(Formatter::new).transpose()?,
                    format_joined: format_joined.map(Formatter::new).transpose()?,
                    omit_if_empty,
                    uniquify,
                });
            }
        }
        Ok(this)
    }
}

/// Registers additional testing methods for the `Args` class.
#[starlark_module]
pub fn args_test_methods(builder: &mut MethodsBuilder) {
    fn expand<'v>(
        this: Value<'v>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        let args_obj = this
            .downcast_ref::<ArgsGen<Value<'v>>>()
            .ok_or_else(|| starlark::Error::from(Error::ExpectedArgsObject))?;
        let (args, inputs) = args_obj.expand(eval)?;
        Ok(eval.heap().alloc((args, inputs)))
    }
}

impl<'v, V: ValueLike<'v>> ArgsGen<V>
where
    Self: ProvidesStaticType<'v>,
{
    /// Expands the stored arguments list into command-line arguments and input files.
    pub fn expand(
        &self,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<(Vec<String>, Vec<File>)> {
        let mut action = crate::action::Action::default();
        action.add_arg(self, eval)?;
        Ok((action.command, action.inputs.into_iter().collect()))
    }
}

/// Registers global test helpers to instantiate mock `Args` and `File` values.
#[starlark_module]
pub fn register_args_test_globals(builder: &mut GlobalsBuilder) {
    fn new_args<'v>(eval: &mut Evaluator<'v, '_, '_>) -> starlark::Result<Value<'v>> {
        Ok(eval.heap().alloc(ArgsGen::new()))
    }

    fn make_file<'v>(
        path: String,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        Ok(eval.heap().alloc(File::from_rust(path)))
    }
}
