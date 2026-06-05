// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use starlark::collections::SmallMap;
use starlark::environment::GlobalsBuilder;
use starlark::eval::Evaluator;
use starlark::values::none::NoneOr;
use starlark::values::Value;
use starlark_derive::starlark_module;

use crate::attr::schema::AttrSchema;
use crate::attr::Attr;
use crate::attr::LabelOrFile;
use crate::Assert;
use crate::EvalContext;
use crate::Label;
use crate::Package;
use attr::EvalContext as _;
use types::EvaluatorContextExt as _;

#[starlark_module]
fn test_globals(builder: &mut GlobalsBuilder) {
    fn validate<'v>(
        schema: &AttrSchema,
        value: Option<Value<'v>>,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Attr> {
        let val_opt = value.map(|v| {
            if v.is_none() {
                NoneOr::None
            } else {
                NoneOr::Other(v)
            }
        });
        let eval_context = eval.context::<EvalContext>();
        Attr::create(
            "$name",
            schema,
            val_opt,
            eval_context.current_package(),
            eval_context.path_resolver(),
        )
    }
}

fn attr_assert() -> Box<Assert> {
    let mut a = Assert::new();
    a.globals_add(test_globals);
    a
}

#[test]
fn test_validate_attr_bool() {
    let a = attr_assert();
    a.eq("validate(attr.bool(), False)", &Attr::Bool(false));
    a.eq("validate(attr.bool(default=True))", &Attr::Bool(true));
    a.eq(
        "validate(attr.bool(mandatory=True), True)",
        &Attr::Bool(true),
    );
    a.fail(
        "validate(attr.bool(mandatory=True))",
        "Attribute `$name` is mandatory",
    );
    a.fail(
        "validate(attr.bool(), None)",
        "expected `bool`, actual `NoneType",
    );
}

#[test]
fn test_validate_attr_int() {
    let a = attr_assert();
    a.eq("validate(attr.int())", &Attr::Int(0));
    a.eq("validate(attr.int(default=42))", &Attr::Int(42));
    a.eq(
        "validate(attr.int(values=[1, 2], default=2))",
        &Attr::Int(2),
    );
    a.fail(
        "validate(attr.int(values=[1, 2], default=1), 3)",
        "Value 3 is not in allowed set",
    );
    // Perhaps unintuitively, bazel allows this.
    a.eq("validate(attr.int(values=[1, 2]))", &Attr::Int(0));
    a.fail(
        "validate(attr.int(mandatory=True))",
        "Attribute `$name` is mandatory",
    );
}

#[test]
fn test_validate_attr_int_list() {
    let a = attr_assert();
    a.eq("validate(attr.int_list())", &Attr::IntList(vec![]));
    a.fail(
        "validate(attr.int_list(allow_empty=False, mandatory=True), [])",
        "Want non-empty list, got []",
    );
    a.eq(
        "validate(attr.int_list(), [1, 2, 3])",
        &Attr::IntList(vec![1, 2, 3]),
    );
    a.fail(
        "validate(attr.int_list(), [1, 'two', 3])",
        "expected `list[int]`",
    );
}

#[test]
fn test_validate_attr_string() {
    let a = attr_assert();
    a.eq("validate(attr.string())", &Attr::String("".to_owned()));
    a.eq(
        "validate(attr.string(default='$name'))",
        &Attr::String("$name".to_owned()),
    );
    a.eq(
        "validate(attr.string(values=['a', 'b']))",
        &Attr::String("".to_owned()),
    );
    a.fail(
        "validate(attr.string(values=['a', 'b'], default='a'), 'c')",
        "Value \"c\" is not in allowed set",
    );
}

#[test]
fn test_validate_attr_string_dict() {
    let a = attr_assert();
    a.eq(
        "validate(attr.string_dict())",
        &Attr::StringDict(SmallMap::new()),
    );
    a.fail(
        "validate(attr.string_dict(allow_empty=False, mandatory=True), {})",
        "Want non-empty dict, got {}",
    );
    a.fail(
        "validate(attr.string_dict(), {'a': 123})",
        "expected `dict[str, str]`",
    );
}

#[test]
fn test_validate_attr_string_list() {
    let a = attr_assert();
    a.eq("validate(attr.string_list())", &Attr::StringList(vec![]));
    a.fail(
        "validate(attr.string_list(allow_empty=False, mandatory=True), [])",
        "Want non-empty list, got []",
    );
    a.fail(
        "validate(attr.string_list(), ['a', 123])",
        "expected `list[str]`",
    );
}

#[test]
fn test_validate_attr_string_list_dict() {
    let a = attr_assert();
    a.eq(
        "validate(attr.string_list_dict())",
        &Attr::StringListDict(SmallMap::new()),
    );
    a.fail(
        "validate(attr.string_list_dict(allow_empty=False, mandatory=True), {})",
        "Want non-empty dict, got {}",
    );
    a.fail(
        "validate(attr.string_list_dict(), {'a': ['b', 123]})",
        "expected `dict[str, list[str]]`",
    );
}

#[test]
fn test_validate_attr_label() {
    let a = attr_assert();
    a.eq(
        "validate(attr.label(), '//pkg:bar')",
        &Attr::Label(LabelOrFile::Label(Label::new(
            Package::from("//pkg".to_owned()),
            "bar".to_owned(),
        ))),
    );
    a.eq(
        "validate(attr.label(default=':bar'))",
        &Attr::Label(LabelOrFile::Label(Label::new(
            Package::from("//".to_owned()),
            "bar".to_owned(),
        ))),
    );
}

#[test]
fn test_validate_attr_label_list() {
    let a = attr_assert();
    a.eq(
        "validate(attr.label_list(), ['//pkg:bar', ':baz'])",
        &Attr::LabelList(vec![
            LabelOrFile::Label(Label::new(
                Package::from("//pkg".to_owned()),
                "bar".to_owned(),
            )),
            LabelOrFile::Label(Label::new(Package::from("//".to_owned()), "baz".to_owned())),
        ]),
    );
}
