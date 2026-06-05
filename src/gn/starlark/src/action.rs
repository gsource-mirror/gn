// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use starlark::eval::Evaluator;
use starlark::values::{Value, ValueLike, FrozenValue, ProvidesStaticType};
use starlark::values::list::ListRef;
use starlark::collections::SmallSet;
use crate::file::File;
use crate::ctx::args::{ArgsGen, ArgValue, Formatter};
use crate::Error;



#[derive(Default)]
pub struct Action {
    pub command: Vec<String>,
    pub inputs: SmallSet<File>,
    pub outputs: Vec<File>,
}

impl Action {
    pub fn add_args<'v>(&mut self, args: Value<'v>, eval: &mut Evaluator<'v, '_, '_>) -> starlark::Result<()> {
        for arg in args.iterate(eval.heap()).map_err(|_| Error::IterableRequired)? {
            if let Some(s) = arg.unpack_str() {
                self.command.push(s.to_owned());
            } else if let Some(args_obj) = arg.downcast_ref::<crate::ctx::args::ArgsGen<Value<'v>>>() {
                self.add_arg(args_obj, eval)?;
            } else if let Some(args_obj) = arg.downcast_ref::<crate::ctx::args::ArgsGen<FrozenValue>>() {
                self.add_arg(args_obj, eval)?;
            } else {
                return Err(Error::InvalidArgumentType(arg.get_type().to_owned()).into());
            }
        }
        Ok(())
    }

    pub fn add_arg<'v, V: ValueLike<'v>>(&mut self, args_obj: &ArgsGen<V>, eval: &mut Evaluator<'v, '_, '_>) -> starlark::Result<()>
    where
        ArgsGen<V>: ProvidesStaticType<'v>,
    {
        let unique = |src: Vec<String>, dest: &mut Vec<String>| {
            let mut seen = std::collections::HashSet::new();
            for s in src {
                if seen.insert(s.clone()) {
                    dest.push(s);
                }
            }
        };

        let args = args_obj.arguments.borrow();
        for arg in args.iter() {
            match arg {
                ArgValue::Scalar {
                    arg_name,
                    value,
                    format,
                } => {
                    if let Some(val) = value {
                        let value = val.to_value();
                        if !value.is_none() {
                            Self::handle_arg(
                                arg_name.as_deref(),
                                value,
                                format.as_ref(),
                                &mut self.inputs,
                                &mut self.command,
                            )?;
                        } else if let Some(flag) = arg_name {
                            self.command.push(flag.clone());
                        }
                    } else if let Some(flag) = arg_name {
                        let formatted = if let Some(fmt) = format {
                            fmt.format(flag)
                        } else {
                            flag.clone()
                        };
                        self.command.push(formatted);
                    }
                }
                ArgValue::All {
                    flag,
                    values,
                    map_each,
                    format_each,
                    before_each,
                    terminate_with,
                    omit_if_empty,
                    uniquify,
                } => {
                    if *uniquify {
                        let mut temp_dest = Vec::new();
                        Self::handle_many_args(
                            values.to_value(),
                            map_each.as_ref().map(|x| x.to_value()),
                            format_each.as_ref(),
                            before_each.as_ref().map(|x| x.as_str()),
                            &mut temp_dest,
                            &mut self.inputs,
                            flag,
                            terminate_with,
                            *omit_if_empty,
                            |dest| {
                                unique(std::mem::take(dest), &mut self.command);
                            },
                            eval,
                        )?;
                    } else {
                        Self::handle_many_args(
                            values.to_value(),
                            map_each.as_ref().map(|x| x.to_value()),
                            format_each.as_ref(),
                            before_each.as_ref().map(|x| x.as_str()),
                            &mut self.command,
                            &mut self.inputs,
                            flag,
                            terminate_with,
                            *omit_if_empty,
                            |_| {},
                            eval,
                        )?;
                    }
                }
                ArgValue::Joined {
                    flag,
                    values,
                    join_with,
                    map_each,
                    format_each,
                    format_joined,
                    omit_if_empty,
                    uniquify,
                } => {
                    let mut dest = Vec::new();
                    Self::handle_many_args(
                        values.to_value(),
                        map_each.as_ref().map(|x| x.to_value()),
                        format_each.as_ref(),
                        None,
                        &mut dest,
                        &mut self.inputs,
                        &None,
                        &None,
                        *omit_if_empty,
                        |dest| {
                            if let Some(arg_name) = flag {
                                self.command.push(arg_name.clone());
                            }
                            let joined = if *uniquify {
                                let mut unique_dest = Vec::new();
                                unique(std::mem::take(dest), &mut unique_dest);
                                unique_dest.join(join_with)
                            } else {
                                dest.join(join_with)
                            };
                            let formatted = if let Some(fj) = format_joined {
                                fj.format(&joined)
                            } else {
                                joined
                            };
                            self.command.push(formatted);
                        },
                        eval,
                    )?;
                }
            }
        }
        Ok(())
    }

    fn handle_arg<'v>(
        arg_name: Option<&str>,
        value: Value<'v>,
        format: Option<&Formatter>,
        inputs: &mut SmallSet<File>,
        dest: &mut Vec<String>,
    ) -> starlark::Result<()> {
        if let Some(flag) = arg_name {
            dest.push(flag.to_owned());
        }
        let s = if let Some(s) = value.unpack_str() {
            s.to_owned()
        } else if let Some(file) = value.downcast_ref::<File>() {
            inputs.insert(file.clone());
            file.as_path().to_string_lossy().into_owned()
        } else if let Some(i) = value.unpack_i32() {
            i.to_string()
        } else {
            return Err(Error::NotFormattable.into());
        };

        let formatted = if let Some(fmt) = format {
            fmt.format(&s)
        } else {
            s
        };
        dest.push(formatted);
        Ok(())
    }

    fn for_each<'v, F>(value: Value<'v>, mut f: F) -> starlark::Result<bool>
    where
        F: FnMut(Value<'v>) -> starlark::Result<()>,
    {
        if let Some(l) = ListRef::from_value(value) {
            for v in l.iter() {
                f(v)?;
            }
            Ok(!l.is_empty())
        } else if let Some(depset) = value.downcast_ref::<crate::depset::Depset>() {
            depset.for_each_fallible(f)?;
            Ok(!depset.is_empty())
        } else {
            Err(Error::ArgumentsMustBeListOrDepset.into())
        }
    }

    fn handle_many_args<'v, F>(
        value: Value<'v>,
        map_each: Option<Value<'v>>,
        format_each: Option<&Formatter>,
        before_each: Option<&str>,
        dest: &mut Vec<String>,
        inputs: &mut SmallSet<File>,
        arg_name: &Option<String>,
        terminate_with: &Option<String>,
        omit_if_empty: bool,
        after: F,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<()>
    where
        F: FnOnce(&mut Vec<String>),
    {
        if let Some(arg_name) = arg_name {
            dest.push(arg_name.clone());
        }

        let before_each = before_each;
        let has_items = Self::for_each(
            value,
            |v| {
                if let Some(map_each) = map_each {
                    let mapped = eval.eval_function(map_each, &[v], &[])?;
                    if let Some(l) = ListRef::from_value(mapped) {
                        for item in l.iter() {
                            Self::handle_arg(before_each, item, format_each, inputs, dest)?;
                        }
                    } else if mapped.unpack_str().is_some() {
                        Self::handle_arg(before_each, mapped, format_each, inputs, dest)?;
                    } else if mapped.is_none() {
                        // skip
                    } else {
                        return Err(Error::MapEachInvalidReturn.into());
                    }
                } else {
                    Self::handle_arg(before_each, v, format_each, inputs, dest)?;
                }
                Ok(())
            },
        )?;

        let want = has_items || !omit_if_empty;
        if want {
            after(dest);
        }

        if let Some(terminate_with) = terminate_with {
            if want {
                dest.push(terminate_with.clone());
            }
        }
        if !want && arg_name.is_some() {
            dest.pop();
        }

        Ok(())
    }
}