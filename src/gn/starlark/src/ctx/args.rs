// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use allocative::Allocative;
use starlark::environment::{Methods, MethodsBuilder, MethodsStatic};
#[cfg(test)]
use starlark::environment::GlobalsBuilder;
use starlark::eval::Evaluator;
use starlark::starlark_complex_value;
use starlark::values::{
    Coerce, Freeze, FreezeResult, Freezer, ProvidesStaticType, StarlarkValue, Trace, Tracer, Value,
    ValueLike, FrozenValue, ValueTyped,
};
use starlark_derive::{starlark_module, starlark_value, NoSerialize};
use std::fmt::{self, Display};
use std::cell::RefCell;
use crate::Error;



#[derive(Debug, Clone, Allocative)]
pub struct Formatter {
    before: String,
    after: String,
}

impl Formatter {
    pub fn new(fmt: &str) -> starlark::Result<Self> {
        let mut split = fmt.split("%s");
        match (split.next(), split.next(), split.next()) {
            (Some(before), Some(after), None) => {
                Ok(Self {
                    before: before.to_owned(),
                    after: after.to_owned(),
                })
            }
            _ => Err(Error::InvalidFormatString(fmt.to_owned()).into()),
        }
    }

    pub fn format(&self, s: &str) -> String {
        format!("{}{}{}", self.before, s, self.after)
    }
}

impl Freeze for Formatter {
    type Frozen = Formatter;
    fn freeze(self, _freezer: &Freezer) -> FreezeResult<Self::Frozen> {
        Ok(self)
    }
}

impl starlark::values::type_repr::StarlarkTypeRepr for Formatter {
    type Canonical = String;

    fn starlark_type_repr() -> starlark::typing::Ty {
        String::starlark_type_repr()
    }
}

impl<'v> starlark::values::UnpackValue<'v> for Formatter {
    type Error = starlark::Error;

    fn unpack_value_impl(value: Value<'v>) -> Result<Option<Self>, Self::Error> {
        let s = <&'v str>::unpack_value_err(value)?;
        Formatter::new(s).map(Some)
    }
}

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

impl<'v> Freeze for ArgValue<Value<'v>> {
    type Frozen = ArgValue<FrozenValue>;
    fn freeze(self, freezer: &Freezer) -> FreezeResult<Self::Frozen> {
        match self {
            ArgValue::Scalar { arg_name, value, format } => Ok(ArgValue::Scalar {
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

#[derive(Debug, ProvidesStaticType, NoSerialize, Allocative)]
#[repr(C)]
pub struct ArgsGen<V> {
    pub(crate) arguments: RefCell<Vec<ArgValue<V>>>,
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

unsafe impl<From, To> starlark::coerce::Coerce<ArgsGen<To>> for ArgsGen<From>
where
    From: starlark::coerce::Coerce<To>,
{}

starlark_complex_value!(pub Args);

impl<V> ArgsGen<V> {
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
            #[cfg(test)]
            args_test_methods(builder);
        });
        Some(RES.methods())
    }
}

impl<'v> Freeze for Args<'v> {
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
    err_msg: &str,
) -> starlark::Result<(Option<String>, Option<Value<'v>>)> {
    if let Some(arg_name) = arg_name_or_value.unpack_str() {
        Ok((Some(arg_name.to_owned()), value))
    } else if value.is_some() {
        Err(Error::GenericError(err_msg.to_owned()).into())
    } else {
        Ok((None, Some(arg_name_or_value)))
    }
}

#[starlark_module]
pub fn args_methods(builder: &mut MethodsBuilder) {
    fn add<'v>(
        this: ValueTyped<'v, Args<'v>>,
        arg_name_or_value: Value<'v>,
        value: Option<Value<'v>>,
        #[starlark(require = named)] format: Option<&str>,
    ) -> starlark::Result<ValueTyped<'v, Args<'v>>> {
        let mut args = this.arguments.borrow_mut();

        let (arg_name, val) = arg_name_and_value(
            arg_name_or_value,
            value,
            "Expected first argument of add to be a string flag when value is specified",
        )?;

        if !val.map_or(false, |v| v.is_none()) {
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

        let (flag, values_to_add) = arg_name_and_value(
            arg_name_or_values,
            values,
            "Expected first argument of add_all to be a string flag when values is specified",
        )?;

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

        let (flag, values_to_add) = arg_name_and_value(
            arg_name_or_values,
            values,
            "Expected first argument of add_joined to be a string flag when values is specified",
        )?;

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

#[cfg(test)]
#[starlark_module]
pub fn args_test_methods(builder: &mut MethodsBuilder) {
    fn expand<'v>(
        this: Value<'v>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        let args_obj = this.downcast_ref::<ArgsGen<Value<'v>>>().ok_or_else(|| {
            Error::GenericError("Expected Args object".to_owned())
        })?;
        let (args, inputs) = args_obj.expand(eval)?;
        Ok(eval.heap().alloc((args, inputs)))
    }
}

impl<'v, V: ValueLike<'v>> ArgsGen<V>
where
    Self: ProvidesStaticType<'v>,
{
    pub fn expand(&self, eval: &mut Evaluator<'v, '_, '_>) -> starlark::Result<(Vec<String>, Vec<crate::file::File>)> {
        let mut action = crate::action::Action::default();
        action.add_arg(self, eval)?;
        Ok((action.command, action.inputs.into_iter().collect()))
    }
}

#[cfg(test)]
#[starlark_module]
pub fn register_args_test_globals(builder: &mut GlobalsBuilder) {
    fn new_args<'v>(eval: &mut Evaluator<'v, '_, '_>) -> starlark::Result<Value<'v>> {
        Ok(eval.heap().alloc(ArgsGen::new()))
    }

    fn new_file<'v>(
        path: &str,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        let leaked_path: &'static std::path::Path = Box::leak(std::path::PathBuf::from(path).into_boxed_path());
        Ok(eval.heap().alloc(crate::file::File(leaked_path)))
    }
}

#[cfg(test)]
mod tests {
    use crate::util::run_starlark_test;

    #[test]
    fn test_args_add_basic() {
        run_starlark_test(
            r#"
a = new_args()
a.add("--foo")
a.add("--bar", "baz")
a.add("--qux", None) # adds nothing
a.add("--val", 1, format="n=%s")
f = new_file("a/b.cc")
a.add(f)
got_args, got_files = a.expand()
assert_eq(got_args, ["--foo", "--bar", "baz", "--val", "n=1", "a/b.cc"])
assert_eq([f.path for f in got_files], ["a/b.cc"])
"#,
        );
    }

    #[test]
    fn test_args_add_all() {
        run_starlark_test(
            r#"
def test_case(*args, want, want_files=[], **kwargs):
  a = new_args()
  a.add_all(*args, **kwargs)
  got_args, got_files = a.expand()
  assert_eq(got_args, want)
  assert_eq([f.path for f in got_files], [f.path for f in want_files])

test_case(
    [1, 2],
    want=["1", "2"]
)
test_case(
    "--flag",
    ["3", "4"],
    before_each = "-b",
    format_each = "f=%s",
    omit_if_empty = True,
    want=["--flag", "-b", "f=3", "-b", "f=4"]
)

test_case(
    "--omit",
    [],
    terminate_with="--term",
    omit_if_empty=True,
    want=[]
)

test_case(
    "--no-omit",
    [],
    omit_if_empty=False,
    want=["--no-omit"]
)

test_case(
    [],
    terminate_with="--term",
    omit_if_empty=False,
    want=["--term"]
)

test_case(
    ["x", "y"],
    before_each="-b",
    terminate_with="--end",
    want=["-b", "x", "-b", "y", "--end"]
)

test_case(
    ["a", "b", "a", "c"],
    uniquify=True,
    want=["a", "b", "c"]
)

def module_formatter(s):
  return s.module_name + "=" + s.pcm

test_case(
    [struct(module_name = "foo", pcm="foo.pcm")],
    map_each=module_formatter,
    want=["foo=foo.pcm"]
)

x = new_file("x.cc")
y = new_file("y.cc")
test_case(
    [x, y],
    want=["x.cc", "y.cc"],
    want_files=[x, y]
)
"#,
        );
    }


    #[test]
    fn test_args_add_joined() {
        run_starlark_test(
            r#"
def test_case(*args, want, want_files=[], **kwargs):
  a = new_args()
  a.add_joined(*args, **kwargs)
  got_args, got_files = a.expand()
  assert_eq(got_args, want)
  assert_eq([f.path for f in got_files], [f.path for f in want_files])

test_case(
    ["a", "b"],
    join_with=",",
    want=["a,b"]
)
test_case(
    "--flag",
    ["c", "d"],
    join_with=":",
    want=["--flag", "c:d"]
)
test_case(
    "--omit",
    [],
    join_with=",",
    omit_if_empty=True,
    want=[]
)
test_case(
    "--no-omit",
    [],
    join_with=",",
    omit_if_empty=False,
    want=["--no-omit", ""]
)

test_case(
    ["a", "b", "a", "c"],
    join_with=",",
    uniquify=True,
    want=["a,b,c"]
)

def prefix_flag(x):
  return "prefix-" + str(x)

test_case(
    ["a", "b"],
    join_with=",",
    map_each=prefix_flag,
    want=["prefix-a,prefix-b"]
)

test_case(
    depset(["a", "b"]),
    join_with = ",",
    want = ["a,b"]
)

b = new_file("a/b.cc")
d = new_file("c/d.cc")
test_case(
    [b, d],
    join_with = ":",
    want = ["a/b.cc:c/d.cc"],
    want_files = [b, d]
)
"#,
        );
    }

}
