// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use depset::{Depset, FrozenDepset};
use starlark::{
    eval::Evaluator,
    values::{list::ListRef, tuple::TupleRef, ProvidesStaticType, Value, ValueLike},
};

use crate::{
    args::{ArgValue, ArgsGen},
    errors::Error,
    formatter::Formatter,
};

/// Helper to process and append a specific `ArgsGen` object to the action's command line.
pub fn expand_into<'v, V: ValueLike<'v> + Copy>(
    command: &mut Vec<String>,
    args_obj: &ArgsGen<V>,
    eval: &mut Evaluator<'v, '_, '_>,
) -> starlark::Result<()>
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
                let value = (*value).to_value();
                if !value.is_none() {
                    handle_arg(
                        arg_name.as_deref(),
                        value,
                        format.as_ref(),
                        command,
                    )?;
                }
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
            } => {
                if *uniquify {
                    let mut temp_dest = Vec::new();
                    handle_many_args(
                        (*values).to_value(),
                        map_each.as_ref(),
                        format_each.as_ref(),
                        before_each.as_ref().map(|x| x.as_str()),
                        &mut temp_dest,
                        flag,
                        terminate_with,
                        *omit_if_empty,
                        |dest| {
                            unique(std::mem::take(dest), command);
                        },
                        eval,
                    )?;
                } else {
                    handle_many_args(
                        (*values).to_value(),
                        map_each.as_ref(),
                        format_each.as_ref(),
                        before_each.as_ref().map(|x| x.as_str()),
                        command,
                        flag,
                        terminate_with,
                        *omit_if_empty,
                        |_| {},
                        eval,
                    )?;
                }
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
            } => {
                let mut dest = Vec::new();
                handle_many_args(
                    (*values).to_value(),
                    map_each.as_ref(),
                    format_each.as_ref(),
                    None,
                    &mut dest,
                    &None,
                    &None,
                    *omit_if_empty,
                    |dest| {
                        if let Some(arg_name) = flag {
                            command.push(arg_name.clone());
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
                        command.push(formatted);
                    },
                    eval,
                )?;
            },
        }
    }
    Ok(())
}

fn handle_arg(
    arg_name: Option<&str>,
    value: Value<'_>,
    format: Option<&Formatter>,
    dest: &mut Vec<String>,
) -> starlark::Result<()> {
    if let Some(flag) = arg_name {
        dest.push(flag.to_owned());
    }

    dest.push(if let Some(fmt) = format {
        fmt.format(&value.to_str())
    } else {
        value.to_str()
    });
    Ok(())
}

fn for_each<'v, F>(value: Value<'v>, mut f: F) -> starlark::Result<()>
where
    F: FnMut(Value<'v>) -> starlark::Result<()>,
{
    if let Some(l) = ListRef::from_value(value) {
        for v in l.iter() {
            f(v)?;
        }
        Ok(())
    } else if let Some(depset) = value.downcast_ref::<Depset>() {
        for v in depset.iter() {
            f(v)?;
        }
        Ok(())
    } else if let Some(depset) = value.downcast_ref::<FrozenDepset>() {
        for v in depset.iter() {
            f(v.to_value())?;
        }
        Ok(())
    } else if let Some(t) = TupleRef::from_value(value) {
        for v in t.iter() {
            f(v)?;
        }
        Ok(())
    } else {
        Err(Error::ArgumentsMustBeListOrDepset.into())
    }
}

fn handle_many_args<'v, V: ValueLike<'v>, F>(
    value: Value<'v>,
    map_each: Option<&V>,
    format_each: Option<&Formatter>,
    before_each: Option<&str>,
    dest: &mut Vec<String>,
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
    let initial_len = dest.len();
    for_each(value, |v| {
        if let Some(map_each) = map_each {
            let mapped = eval.eval_function((*map_each).to_value(), &[v], &[])?;
            if let Some(l) = ListRef::from_value(mapped) {
                for item in l.iter() {
                    if item.unpack_str().is_none() {
                        return Err(Error::MapEachInvalidReturn.into());
                    }
                    handle_arg(before_each, item, format_each, dest)?;
                }
            } else if mapped.unpack_str().is_some() {
                handle_arg(before_each, mapped, format_each, dest)?;
            } else if mapped.is_none() {
                // skip
            } else {
                return Err(Error::MapEachInvalidReturn.into());
            }
        } else {
            if v.is_none() {
                return Err(Error::NoneNotAllowed.into());
            }
            handle_arg(before_each, v, format_each, dest)?;
        }
        Ok(())
    })?;

    let has_items = dest.len() > initial_len;
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
