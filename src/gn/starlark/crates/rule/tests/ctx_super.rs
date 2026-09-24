// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use rule::run;
use starlark::values::list::ListRef;
use testutils::Assert;
use types::{Label, PackageRef, Session as _};

#[test]
fn test_ctx_super() {
    let mut assert = Assert::new_rule_assert();
    assert.load_module("//rules:pure.scl");
    assert.pass(
        r#"
load("//rules:pure.scl", "child_rule", "parent_rule")

parent_rule(
    name = "child",
    parent_only = "child_parent_val",
)

child_rule(
    name = "target",
    parent_only = "parent_val",
    child_only = "child_val",
)
"#,
    );

    let context = assert.context();
    let label = Label::new(PackageRef::root().to_owned(), "target".to_owned());
    let target = context
        .session
        .get_target(label.as_ref(), context.session.default_toolchain.as_ref());

    let session = assert.session();
    let res = run(&target, move |t: &testutils::FakeTargetRef| {
        testutils::FakeEvalContext::rule_impl(session.clone(), t.clone())
    })
    .unwrap();

    let list = ListRef::from_value(res.as_ref().value()).unwrap();
    let items: Vec<starlark::values::Value<'_>> = list.iter().collect();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].to_repr(), r#"ParentInfo(parent = "parent_val")"#);
    assert_eq!(items[1].to_repr(), r#"ChildInfo(child = "child_val")"#);
}
#[test]
fn test_run_builtin_target() {
    let mut assert = Assert::new_rule_assert();
    assert.pass(
        r#"
load("//builtins:rules.scl", "static_library")

static_library(
    name = "native_target",
)
"#,
    );

    let context = assert.context();
    let label = Label::new(PackageRef::root().to_owned(), "native_target".to_owned());
    let target = context
        .session
        .get_target(label.as_ref(), context.session.default_toolchain.as_ref());

    let session = assert.session();
    let res = run(&target, move |t: &testutils::FakeTargetRef| {
        testutils::FakeEvalContext::rule_impl(session.clone(), t.clone())
    })
    .unwrap();

    let list = ListRef::from_value(res.as_ref().value()).unwrap();
    assert_eq!(list.len(), 0);
}

#[test]
fn test_ctx_super_builtin() {
    let mut assert = Assert::new_rule_assert();
    assert.module(
        "//rules:custom.scl",
        r#"
load("//builtins:rules.scl", "static_library")

def _child_impl(ctx):
    return ctx.super()

child_rule = rule(
    implementation = _child_impl,
    parent = static_library,
)
"#,
    );
    assert.pass(
        r#"
load("//rules:custom.scl", "child_rule")

child_rule(
    name = "target",
)
"#,
    );

    let context = assert.context();
    let label = Label::new(PackageRef::root().to_owned(), "target".to_owned());
    let target = context
        .session
        .get_target(label.as_ref(), context.session.default_toolchain.as_ref());

    let session = assert.session();
    let res = run(&target, move |t: &testutils::FakeTargetRef| {
        testutils::FakeEvalContext::rule_impl(session.clone(), t.clone())
    })
    .unwrap();

    let list = ListRef::from_value(res.as_ref().value()).unwrap();
    assert_eq!(list.len(), 0);
}

#[test]
fn test_load_builtin_providers() {
    let mut assert = Assert::new_rule_assert();
    assert.module(
        "//rules:providers_rule.scl",
        r#"
load("//builtins:providers.scl", "GnInputsInfo", "GnSubstitutionsInfo")

def _rule_impl(ctx):
    return [
        DefaultInfo(),
        GnInputsInfo(files = []),
        GnSubstitutionsInfo(substitutions = struct(flag = ["--opt"])),
    ]

my_rule = rule(
    implementation = _rule_impl,
)
"#,
    );
    assert.pass(
        r#"
load("//rules:providers_rule.scl", "my_rule")

my_rule(
    name = "target_with_builtin_providers",
)
"#,
    );

    let context = assert.context();
    let label = Label::new(
        PackageRef::root().to_owned(),
        "target_with_builtin_providers".to_owned(),
    );
    let target = context
        .session
        .get_target(label.as_ref(), context.session.default_toolchain.as_ref());

    let session = assert.session();
    let res = run(&target, move |t: &testutils::FakeTargetRef| {
        testutils::FakeEvalContext::rule_impl(session.clone(), t.clone())
    })
    .unwrap();

    let list = ListRef::from_value(res.as_ref().value()).unwrap();
    let items: Vec<starlark::values::Value<'_>> = list.iter().collect();
    assert_eq!(items.len(), 3);
}
