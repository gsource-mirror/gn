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

pub trait Ostream {
    fn write(&mut self, s: &str);

    fn write_token(&mut self, s: &str, needs_space: &mut bool) {
        if *needs_space {
            self.write(" ");
        }
        self.write(s);
        *needs_space = true;
    }
}

impl Ostream for String {
    fn write(&mut self, s: &str) {
        self.push_str(s);
    }
}

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

/// Helper to process and write a specific `FrozenArgs` object to the stream.
pub fn write<'v>(
    stream: &mut impl Ostream,
    args_obj: &FrozenArgs<'v>,
    eval: &mut Evaluator<'v, '_, '_>,
    needs_space: &mut bool,
) -> starlark::Result<()> {
    for arg in &args_obj.arguments {
        match arg {
            ArgValue::Scalar {
                arg_name,
                value,
                format,
            } => {
                if !value.is_none() {
                    handle_arg(
                        arg_name.as_deref(),
                        *value,
                        format.as_ref(),
                        stream,
                        needs_space,
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
                handle_many_args(
                    *values,
                    *map_each,
                    stream,
                    needs_space,
                    |stream, needs_space| {
                        if let Some(f) = flag.as_deref() {
                            stream.write_token(f, needs_space);
                        }
                    },
                    |stream, needs_space, v| {
                        if let Some(before) = before_each.as_deref() {
                            stream.write_token(before, needs_space);
                        }
                        if let Some(f) = format_each.as_ref() {
                            stream.write_token(&f.before, needs_space);
                            stream.write(v);
                            stream.write(&f.after);
                        } else {
                            stream.write_token(v, needs_space);
                        }
                    },
                    |stream, needs_space| {
                        if let Some(t) = terminate_with.as_deref() {
                            stream.write_token(t, needs_space);
                        }
                    },
                    *omit_if_empty,
                    *uniquify,
                    eval,
                )?;
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
                let mut needs_delim = false;
                handle_many_args(
                    *values,
                    *map_each,
                    stream,
                    needs_space,
                    |stream, needs_space| {
                        if let Some(f) = flag.as_deref() {
                            stream.write_token(f, needs_space);
                        }
                        if *needs_space {
                            stream.write(" ");
                        }
                        *needs_space = true;
                        if let Some(f) = format_joined.as_ref() {
                            stream.write(&f.before);
                        }
                    },
                    |stream, _needs_space, v| {
                        if needs_delim {
                            stream.write(join_with);
                        }
                        if let Some(f) = format_each.as_ref() {
                            stream.write(&f.before);
                            stream.write(v);
                            stream.write(&f.after);
                        } else {
                            stream.write(v);
                        }
                        needs_delim = true;
                    },
                    |stream, _needs_space| {
                        if let Some(f) = format_joined.as_ref() {
                            stream.write(&f.after);
                        }
                    },
                    *omit_if_empty,
                    *uniquify,
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
    stream: &mut impl Ostream,
    needs_space: &mut bool,
) -> starlark::Result<()> {
    if let Some(flag) = arg_name {
        stream.write_token(flag, needs_space);
    }

    let val_str = escape_value(value);
    if let Some(fmt) = format {
        stream.write_token(&fmt.before, needs_space);
        stream.write(&val_str);
        stream.write(&fmt.after);
    } else {
        stream.write_token(&val_str, needs_space);
    }
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

fn handle_many_args<'v, S: Ostream>(
    value: Value<'v>,
    map_each: Option<Value<'v>>,
    stream: &mut S,
    needs_space: &mut bool,
    before: impl FnOnce(&mut S, &mut bool),
    mut for_each_item: impl FnMut(&mut S, &mut bool, &str),
    after: impl FnOnce(&mut S, &mut bool),
    omit_if_empty: bool,
    uniquify: bool,
    eval: &mut Evaluator<'v, '_, '_>,
) -> starlark::Result<()> {
    let mut before = Some(before);
    let mut seen = starlark::collections::SmallSet::new();
    let mut wrote_any = false;

    let mut process_item = |item: Value<'v>| -> starlark::Result<()> {
        if item.is_none() {
            return Err(Error::NoneNotAllowed.into());
        }
        let s = escape_value(item);
        if uniquify && !seen.insert(s.clone()) {
            return Ok(());
        }

        if let Some(f) = before.take() {
            f(stream, needs_space);
        }
        for_each_item(stream, needs_space, &s);
        wrote_any = true;
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

    if let Some(f) = before.take()
        && !omit_if_empty
    {
        f(stream, needs_space);
    }
    if wrote_any || !omit_if_empty {
        after(stream, needs_space);
    }

    Ok(())
}
