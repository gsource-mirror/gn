// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use starlark::values::UnpackValue as _;

use crate::{
    ArgVisitor, args::register_args_test_globals, expand::expand_args, unpack::FrozenArgsSequence,
};

#[derive(Default)]
struct VecCollector {
    command: Vec<String>,
    current: String,
}

impl ArgVisitor for VecCollector {
    fn add_arg_chunk(&mut self, part: &str) {
        self.current.push_str(part);
    }

    fn finish_arg(&mut self) {
        self.command.push(std::mem::take(&mut self.current));
    }
}

#[test]
fn test_mutate_frozen_args_fails() {
    let mut a = testutils::Assert::default();
    a.modify_globals(|builder| {
        register_args_test_globals(builder);
    });
    a.module("frozen.scl", "frozen_args = args()");
    a.fail(
        "load('frozen.scl', 'frozen_args'); frozen_args.add('foo')",
        "trying to mutate a frozen Args value",
    );
}

#[test]
fn test_args_format_validation() {
    let mut a = testutils::Assert::default();
    a.modify_globals(|builder| {
        register_args_test_globals(builder);
    });
    a.fail(
        r#"args().add("--val", 1, format="no percent s")"#,
        "Format string must contain exactly one '%s'",
    );
    a.fail(
        r#"args().add("--val", 1, format="two %s %s")"#,
        "Format string must contain exactly one '%s'",
    );
}

#[test]
fn test_args_expansion_with_vec_collector() {
    let mut a = testutils::Assert::default();
    a.modify_globals(|builder| {
        register_args_test_globals(builder);
    });
    let val = a.pass(
        r#"
[
    args()
        .add("--flag")
        .add("val")
        .add_all(["alpha", "beta"], before_each = "-b")
        .add_joined(["x", "y"], join_with = ":")
]
"#,
    );
    let res = starlark::environment::Module::with_temp_heap(|module| {
        let mut eval = starlark::eval::Evaluator::new(&module);
        let val_ref = val.as_ref().add_to_heap(module.heap());
        let seq = FrozenArgsSequence::unpack_value_err(val_ref).unwrap();
        let mut collector = VecCollector::default();
        for item in &seq.0 {
            match item {
                either::Either::Left(s) => collector.add_arg(s),
                either::Either::Right(arg) => expand_args(arg, &mut eval, &mut collector)?,
            }
        }
        starlark::Result::Ok(collector.command)
    });
    assert_eq!(
        res.unwrap(),
        vec!["--flag", "val", "-b", "alpha", "-b", "beta", "x:y"]
    );
}
