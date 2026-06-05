// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use starlark::{
    environment::GlobalsBuilder,
    eval::Evaluator,
    values::{list::UnpackList, UnpackValue, Value},
};
use starlark_derive::starlark_module;

use crate::args::ArgsGen;

fn list(items: &[&str]) -> UnpackList<String> {
    UnpackList {
        items: items.iter().map(|s| (*s).to_owned()).collect(),
    }
}

#[starlark_module]
pub(crate) fn register_args_test_globals(builder: &mut GlobalsBuilder) {
    fn args<'v>(eval: &mut Evaluator<'v, '_, '_>) -> starlark::Result<Value<'v>> {
        Ok(eval.heap().alloc(ArgsGen::default()))
    }

    fn identity<'v>(x: Value<'v>) -> starlark::Result<Value<'v>> {
        Ok(x)
    }

    fn expand<'v>(
        args: Value<'v>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Vec<String>> {
        crate::unpack::ArgsSequence::unpack_value_err(args)?.expand(eval)
    }
}

fn new_assert() -> testutils::Assert {
    let mut a = testutils::Assert::default();
    a.modify_globals(|builder| {
        register_args_test_globals(builder);
        depset::depset_globals!(builder, testutils::FakeEvalContext);
    });
    a
}

#[test]
fn test_combine_string_and_args() {
    let mut a = new_assert();
    a.eq(
        r#"expand(["foo", args().add("bar"), "baz"])"#,
        list(&["foo", "bar", "baz"]),
    );
}

#[test]
fn test_args_add() {
    let mut a = new_assert();
    a.eq(r#"expand([args().add("--foo")])"#, list(&["--foo"]));

    a.eq(
        r#"expand([args().add(make_file("a/b.cc"))])"#,
        list(&["a/b.cc"]),
    );

    a.eq(
        r#"expand([args().add("--bar", "baz")])"#,
        list(&["--bar", "baz"]),
    );

    a.eq(r#"expand([args().add("--qux", None)])"#, list(&[]));

    a.eq(
        r#"expand([args().add("--val", 1, format="before %s after")])"#,
        list(&["--val", "before 1 after"]),
    );

    a.fail(
        r#"expand([args().add("--val", 1, format="no percent s")])"#,
        "Format string must contain exactly one '%s'",
    );

    a.fail(
        r#"expand([args().add("--val", 1, format="two %s %s")])"#,
        "Format string must contain exactly one '%s'",
    );
}

#[test]
fn test_args_add_all() {
    let mut a = new_assert();
    a.eq(r#"expand([args().add_all([1, 2])])"#, list(&["1", "2"]));

    a.eq(
        r#"expand([args().add_all("--flag", ["3", "4"], before_each = "-b")])"#,
        list(&["--flag", "-b", "3", "-b", "4"]),
    );

    a.eq(
        r#"expand([args().add_all(["x", "y"], before_each="-b", format_each=".%s.", terminate_with="--end")])"#,
        list(&["-b", ".x.", "-b", ".y.", "--end"]),
    );

    a.eq(
        r#"expand([args().add_all([make_file("x.cc"), make_file("y.cc")])])"#,
        list(&["x.cc", "y.cc"]),
    );

    a.fail(
        r#"expand([args().add_all(["a", None])])"#,
        "None is not allowed unless mapped by map_each",
    );
}

#[test]
fn test_args_add_joined() {
    let mut a = new_assert();
    a.eq(
        r#"expand([args().add_joined([1, 2], join_with = ',')])"#,
        list(&["1,2"]),
    );

    a.eq(
        r#"expand([args().add_joined("--flag", ["c", "d"], join_with=":")])"#,
        list(&["--flag", "c:d"]),
    );

    a.eq(
        r#"expand([args().add_joined(depset(["a", "b"]), join_with = ",")])"#,
        list(&["a,b"]),
    );

    a.eq(
        r#"
expand([
  args()
    .add_joined(
      [make_file("a/b.cc"), make_file("c/d.cc")],
      join_with = ":",
    )
])
"#,
        list(&["a/b.cc:c/d.cc"]),
    );

    a.eq(
        r#"expand([args().add_joined([1, 2], join_with=",", format_each=".%s.", format_joined="list=%s")])"#,
        list(&["list=.1.,.2."]),
    );
}

#[test]
fn test_args_omit_if_empty() {
    let mut a = new_assert();

    a.eq(
        r#"expand([args().add_all("--flag", [], terminate_with = "after")])"#,
        list(&[]),
    );

    a.eq(
        r#"expand([args().add_joined("--flag", [], join_with=",", omit_if_empty=False)])"#,
        list(&["--flag", ""]),
    );

    a.eq(
        r#"expand([args().add_all("--flag", [None], map_each=identity, allow_closure=False, omit_if_empty=True)])"#,
        list(&[]),
    );
}

#[test]
fn test_args_map_each() {
    let mut a = new_assert();

    a.eq(
        r#"expand([args().add_all(["abc", ["def", "ghi"], None], map_each=identity, allow_closure=False)])"#,
        list(&["abc", "def", "ghi"]),
    );

    a.fail(
        r#"expand([args().add_all([[1]], map_each=identity, allow_closure=False)])"#,
        "map_each must return a list[str], str, or None",
    );

    a.fail(
        r#"expand([args().add_all([1], map_each=identity, allow_closure=False)])"#,
        "map_each must return a list[str], str, or None",
    );

    a.fail(
        r#"expand([args().add_all([1], map_each=identity)])"#,
        "map_each was specified without allow_closure",
    );
    a.fail(
        r#"expand([args().add_all([1], map_each=fail, allow_closure=False)])"#,
        "fail: 1",
    );
}

#[test]
fn test_args_uniquify() {
    let mut a = new_assert();

    a.eq(
        r#"expand([args().add_all(["a", "b", "a", "c"])])"#,
        list(&["a", "b", "a", "c"]),
    );

    a.eq(
        r#"expand([args().add_all(["a", "b", "a", "c"], uniquify=True)])"#,
        list(&["a", "b", "c"]),
    );

    a.eq(
        r#"expand([args().add_joined(["a", "b", "a", "c"], join_with=",", uniquify=True)])"#,
        list(&["a,b,c"]),
    );
}

#[test]
fn test_args_chaining() {
    let mut a = new_assert();
    a.eq(
        r#"
expand([
  args()
    .add("--foo")
    .add_all(["bar", "baz"])
    .add_joined(["x", "y"], join_with=":")
])
"#,
        list(&["--foo", "bar", "baz", "x:y"]),
    );
}
