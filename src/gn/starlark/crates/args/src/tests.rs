// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use starlark::environment::GlobalsBuilder;
use starlark::environment::LibraryExtension;
use starlark::eval::Evaluator;
use starlark::typing::Ty;
use starlark::values::list::UnpackList;
use starlark::values::type_repr::StarlarkTypeRepr;
use starlark::values::UnpackValue;
use starlark::values::Value;
use starlark_derive::starlark_module;
use types::File;
use types::EvaluatorContextExt;

use super::*;

#[derive(PartialEq, Debug)]
struct UnpackedArgs<'v> {
    args: Vec<&'v str>,
    files: Vec<&'v std::path::Path>,
}

impl StarlarkTypeRepr for UnpackedArgs<'_> {
    type Canonical = Self;

    fn starlark_type_repr() -> Ty {
        Ty::any()
    }
}

impl<'v> UnpackValue<'v> for UnpackedArgs<'v> {
    type Error = starlark::Error;

    fn unpack_value_impl(value: Value<'v>) -> Result<Option<Self>, starlark::Error> {
        let (args, files_list) = <(UnpackList<&str>, UnpackList<&File>)>::unpack_value_err(value)?;
        Ok(Some(UnpackedArgs {
            args: args.items,
            files: files_list.items.into_iter().map(|f| f.as_path()).collect(),
        }))
    }
}

#[starlark_module]
fn mock_depset_global(builder: &mut GlobalsBuilder) {
    fn depset<'v>(
        direct: Option<UnpackList<Value<'v>>>,
        transitive: Option<UnpackList<depset::UnpackDepset<'v>>>,
        #[starlark(default = depset::Order::Unspecified)] order: depset::Order,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        depset::depset_constructor::<testutils::FakeEvalContext>(direct, transitive, order, &eval.heap(), eval.context_mut())
    }
}

fn new_assert() -> testutils::Assert {
    let mut a = testutils::Assert::new();
    let mut builder = GlobalsBuilder::extended_by(&[LibraryExtension::StructType]);
    register_args_test_globals(&mut builder);
    mock_depset_global(&mut builder);
    a.globals(builder.build());
    a
}

#[test]
fn test_args_add_basic() {
    let a = new_assert();
    a.eq(
        r#"
f = make_file("a/b.cc")
(new_args()
  .add("--foo")
  .add("--bar", "baz")
  .add("--qux", None)
  .add("--val", 1, format="n=%s")
  .add(f)
  .expand())
"#,
        UnpackedArgs {
            args: vec!["--foo", "--bar", "baz", "--val", "n=1", "a/b.cc"],
            files: vec![std::path::Path::new("a/b.cc")],
        },
    );
}

#[test]
fn test_args_add_all() {
    let a = new_assert();
    a.eq(
        "new_args().add_all([1, 2]).expand()",
        UnpackedArgs {
            args: vec!["1", "2"],
            files: vec![],
        },
    );

    a.eq(
            r#"new_args().add_all("--flag", ["3", "4"], before_each = "-b", format_each = "f=%s", omit_if_empty = True).expand()"#,
            UnpackedArgs { args: vec!["--flag", "-b", "f=3", "-b", "f=4"], files: vec![] }
        );

    a.eq(
        r#"new_args().add_all("--omit", [], terminate_with="--term", omit_if_empty=True).expand()"#,
        UnpackedArgs {
            args: vec![],
            files: vec![],
        },
    );

    a.eq(
        r#"new_args().add_all("--no-omit", [], omit_if_empty=False).expand()"#,
        UnpackedArgs {
            args: vec!["--no-omit"],
            files: vec![],
        },
    );

    a.eq(
        r#"new_args().add_all([], terminate_with="--term", omit_if_empty=False).expand()"#,
        UnpackedArgs {
            args: vec!["--term"],
            files: vec![],
        },
    );

    a.eq(
        r#"new_args().add_all(["x", "y"], before_each="-b", terminate_with="--end").expand()"#,
        UnpackedArgs {
            args: vec!["-b", "x", "-b", "y", "--end"],
            files: vec![],
        },
    );

    a.eq(
        r#"new_args().add_all(["a", "b", "a", "c"], uniquify=True).expand()"#,
        UnpackedArgs {
            args: vec!["a", "b", "c"],
            files: vec![],
        },
    );

    a.eq(
        r#"
def module_formatter(s):
  return s.module_name + "=" + s.pcm

new_args().add_all([struct(module_name = "foo", pcm="foo.pcm")], map_each=module_formatter).expand()
"#,
        UnpackedArgs {
            args: vec!["foo=foo.pcm"],
            files: vec![],
        },
    );

    a.eq(
        r#"
x = make_file("x.cc")
y = make_file("y.cc")
new_args().add_all([x, y]).expand()
"#,
        UnpackedArgs {
            args: vec!["x.cc", "y.cc"],
            files: vec![std::path::Path::new("x.cc"), std::path::Path::new("y.cc")],
        },
    );
}

#[test]
fn test_args_add_joined() {
    let a = new_assert();
    a.eq(
        r#"new_args().add_joined(["a", "b"], join_with=",").expand()"#,
        UnpackedArgs {
            args: vec!["a,b"],
            files: vec![],
        },
    );

    a.eq(
        r#"new_args().add_joined("--flag", ["c", "d"], join_with=":").expand()"#,
        UnpackedArgs {
            args: vec!["--flag", "c:d"],
            files: vec![],
        },
    );

    a.eq(
        r#"new_args().add_joined("--omit", [], join_with=",", omit_if_empty=True).expand()"#,
        UnpackedArgs {
            args: vec![],
            files: vec![],
        },
    );

    a.eq(
        r#"new_args().add_joined("--no-omit", [], join_with=",", omit_if_empty=False).expand()"#,
        UnpackedArgs {
            args: vec!["--no-omit", ""],
            files: vec![],
        },
    );

    a.eq(
        r#"new_args().add_joined(["a", "b", "a", "c"], join_with=",", uniquify=True).expand()"#,
        UnpackedArgs {
            args: vec!["a,b,c"],
            files: vec![],
        },
    );

    a.eq(
        r#"
def prefix_flag(x):
  return "prefix-" + str(x)

new_args().add_joined(["a", "b"], join_with=",", map_each=prefix_flag).expand()
"#,
        UnpackedArgs {
            args: vec!["prefix-a,prefix-b"],
            files: vec![],
        },
    );

    a.eq(
        r#"new_args().add_joined(depset(["a", "b"]), join_with = ",").expand()"#,
        UnpackedArgs {
            args: vec!["a,b"],
            files: vec![],
        },
    );

    a.eq(
        r#"
b = make_file("a/b.cc")
d = make_file("c/d.cc")
new_args().add_joined([b, d], join_with = ":").expand()
"#,
        UnpackedArgs {
            args: vec!["a/b.cc:c/d.cc"],
            files: vec![
                std::path::Path::new("a/b.cc"),
                std::path::Path::new("c/d.cc"),
            ],
        },
    );
}
