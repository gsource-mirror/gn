// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::borrow::Cow;

use depset::Depset;
use starlark::{
    eval::Evaluator,
    values::{Value, list::ListRef, tuple::TupleRef},
};
use types::{escape_ninja_token, escape_ninja_token_owned};

use crate::{
    Error,
    args::{ArgValue, FrozenArgs},
    formatter::Formatter,
};

fn escape_value<'v>(value: Value<'v>) -> Cow<'v, str> {
    if let Some(s) = value.unpack_str() {
        escape_ninja_token(s)
    } else if let Some(file) = value.downcast_ref::<types::File>() {
        file.ninja_escaped_arg()
    } else {
        // to_str produces an *owned* string.
        Cow::Owned(escape_ninja_token_owned(value.to_str()))
    }
}

/// Helper to process and append a specific `FrozenArgs` object to the action's
/// command line.
pub fn expand_into<'v>(
    command: &mut Vec<String>,
    args_obj: &FrozenArgs<'v>,
    eval: &mut Evaluator<'v, '_, '_>,
) -> starlark::Result<()> {
    for arg in &args_obj.arguments {
        match arg {
            ArgValue::Scalar {
                arg_name,
                value,
                format,
            } => {
                if !value.is_none() {
                    handle_arg(arg_name.as_deref(), *value, format.as_ref(), command)?;
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
                let initial_len = command.len();
                if let Some(flag) = flag {
                    command.push(flag.clone());
                }

                let start_idx = command.len();
                handle_many_args(
                    *values,
                    *map_each,
                    format_each.as_ref(),
                    before_each.as_deref(),
                    command,
                    *uniquify,
                    eval,
                )?;

                if command.len() > start_idx || !omit_if_empty {
                    if let Some(term) = terminate_with {
                        command.push(term.clone());
                    }
                } else if flag.is_some() {
                    command.truncate(initial_len);
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
                    *values,
                    *map_each,
                    format_each.as_ref(),
                    None,
                    &mut dest,
                    *uniquify,
                    eval,
                )?;

                if !dest.is_empty() || !omit_if_empty {
                    if let Some(flag) = flag {
                        command.push(flag.clone());
                    }
                    let joined = dest.join(join_with);
                    command.push(if let Some(fj) = format_joined {
                        fj.format(&joined)
                    } else {
                        joined
                    });
                }
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

    let val_str = escape_value(value);
    dest.push(if let Some(fmt) = format {
        fmt.format(&val_str)
    } else {
        val_str.into_owned()
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
    } else if let Some(t) = TupleRef::from_value(value) {
        for v in t.iter() {
            f(v)?;
        }
        Ok(())
    } else {
        Err(Error::ArgumentsMustBeListTupleOrDepset.into())
    }
}

fn handle_many_args<'v>(
    value: Value<'v>,
    map_each: Option<Value<'v>>,
    format_each: Option<&Formatter>,
    before_each: Option<&str>,
    dest: &mut Vec<String>,
    uniquify: bool,
    eval: &mut Evaluator<'v, '_, '_>,
) -> starlark::Result<()> {
    let mut seen = starlark::collections::SmallSet::new();

    let mut process_item = |item: Value<'v>| -> starlark::Result<()> {
        if item.is_none() {
            return Err(Error::NoneNotAllowed.into());
        }
        let s = escape_value(item);
        if uniquify && !seen.insert(s.clone()) {
            return Ok(());
        }

        if let Some(before) = before_each {
            dest.push(before.to_owned());
        }
        dest.push(if let Some(format) = format_each {
            format.format(&s)
        } else {
            s.into_owned()
        });
        Ok(())
    };

    for_each(value, |v| {
        if let Some(map_each) = map_each {
            let mapped = eval.eval_function(map_each, &[v], &[])?;
            if let Some(l) = ListRef::from_value(mapped) {
                for item in l.iter() {
                    if item.unpack_str().is_none() {
                        return Err(Error::MapEachInvalidReturn.into());
                    }
                    process_item(item)?;
                }
            } else if let Some(t) = TupleRef::from_value(mapped) {
                for item in t.iter() {
                    if item.unpack_str().is_none() {
                        return Err(Error::MapEachInvalidReturn.into());
                    }
                    process_item(item)?;
                }
            } else if mapped.unpack_str().is_some() {
                process_item(mapped)?;
            } else if mapped.is_none() {
                // skip
            } else {
                return Err(Error::MapEachInvalidReturn.into());
            }
        } else {
            process_item(v)?;
        }
        Ok(())
    })?;

    Ok(())
}
