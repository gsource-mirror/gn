// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#![cfg(test)]

use starlark::environment::GlobalsBuilder;
use starlark::eval::Evaluator;
use starlark::values::list::UnpackList;
use starlark::values::{Value, ValueLike as _};
use starlark_derive::starlark_module;

use crate::FrozenDepset;
use types::EvaluatorContextExt;

#[starlark_module]
fn test_globals(builder: &mut GlobalsBuilder) {
    fn make_file<'v>(
        eval: &mut Evaluator<'v, '_, '_>,
        path: String,
    ) -> starlark::Result<Value<'v>> {
        Ok(eval.heap().alloc(types::File::from_rust(path)))
    }

    fn depset<'v>(
        direct: Option<UnpackList<Value<'v>>>,
        transitive: Option<UnpackList<crate::UnpackDepset<'v>>>,
        #[starlark(default = crate::Order::Unspecified)] order: crate::Order,
        eval: &mut Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        crate::depset_constructor::<testutils::FakeEvalContext>(direct, transitive, order, &eval.heap(), eval.context_mut())
    }
}

fn new_assert() -> testutils::Assert {
    let mut a = testutils::Assert::new();
    a.globals_add(test_globals);
    a
}

#[test]
fn test_depset() {
    let a = new_assert();
    a.equivalent(
        "depset(['c'], transitive=[depset(['a', 'b'])], order='preorder').to_list()",
        "['c', 'a', 'b']",
    );
    a.equivalent(
        "depset(['c'], transitive=[depset(['a', 'b'])], order='postorder').to_list()",
        "['a', 'b', 'c']",
    );
    a.equivalent(
        "depset(['c'], transitive=[depset(['a', 'b'])], order='topological').to_list()",
        "['c', 'b', 'a']",
    );

    // Truthiness checks.
    a.equivalent("bool(depset(['a']))", "True");
    a.equivalent("bool(depset())", "False");
    a.equivalent("bool(depset(transitive=[depset()]))", "False");

    a.equivalent("repr(depset())", "\"depset(...)\"");
}

#[test]
fn test_depset_invalid_transitive() {
    let a = new_assert();
    a.fail(
        "depset(['c'], transitive=['not a depset'])",
        "Expected value of type `depset` but got `string (repr: \"not a depset\")`",
    );
}

#[test]
fn test_depset_conflicting_orders() {
    let a = new_assert();
    a.fail(
        "depset(transitive=[depset(['a'], order='preorder'), depset(['b'], order='postorder')])",
        "conflicting orders: depset has order preorder, but transitive child has order postorder",
    );
}

#[test]
fn test_depset_type_validation() {
    let mut a = new_assert();
    let frozen_str_depset = a.pass("depset(['a', 'b'])");
    let frozen_file_depset = a.pass("depset([make_file('a.txt'), make_file('b.txt')])");

    a.globals_add(move |builder: &mut starlark::environment::GlobalsBuilder| {
        test_globals(builder);
        builder.set("frozen_str_depset", frozen_str_depset.clone());
        builder.set("frozen_file_depset", frozen_file_depset.clone());
    });

    // Homogeneous File depset.
    a.equivalent(
        "[f.path for f in depset([make_file('a.txt'), make_file('b.txt')]).to_list()]",
        "['a.txt', 'b.txt']",
    );

    // Homogeneous string depset.
    a.equivalent("depset(['a', 'b']).to_list()", "['a', 'b']");

    // Mixing File and String in direct elements.
    a.fail(
        "depset([make_file('a.txt'), 'b'])",
        "depset elements must be of the same type, expected File, got unknown",
    );

    // Mixing String and File in direct elements.
    a.fail(
        "depset(['a', make_file('b.txt')])",
        "depset elements must be of the same type, expected unknown, got File",
    );

    // Mixing File depset and String depset in transitive elements.
    a.fail(
        "depset(transitive=[depset([make_file('a.txt')]), depset(['b'])])",
        "depset elements must be of the same type, expected File, got unknown",
    );

    // Direct File elements mixed with transitive String depset.
    a.fail(
        "depset([make_file('a.txt')], transitive=[depset(['b'])])",
        "depset elements must be of the same type, expected File, got unknown",
    );

    // Transitive depset containing a frozen depset.
    a.equivalent(
        "depset(['c'], transitive=[frozen_str_depset], order='postorder').to_list()",
        "['a', 'b', 'c']",
    );

    // Transitive depset containing a frozen File depset.
    a.equivalent(
        "[f.path for f in depset([make_file('c.txt')], transitive=[frozen_file_depset], order='postorder').to_list()]",
        "['a.txt', 'b.txt', 'c.txt']",
    );
}

#[test]
fn test_depset_deduplication() {
    let a = new_assert();
    let depset_val = a.pass("depset(['a', 'a'])");
    let depset = depset_val.value().downcast_ref::<FrozenDepset>().unwrap();
    assert_eq!(depset.direct().len(), 1);
}
