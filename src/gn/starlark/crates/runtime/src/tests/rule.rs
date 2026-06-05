// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use crate::rule::run_target_rule_implementation;
use crate::Assert;
use crate::LabelRef;
use crate::Target;

#[test]
fn test_custom_rule_execution() {
    let mut a = Assert::new();

    let rules_bzl_content = r#"
MyProvider = provider(fields = ["val"])

def _my_rule_impl(ctx):
    return [MyProvider(val = ctx.attr.val)]

my_rule = rule(
    implementation = _my_rule_impl,
    attrs = {
        "val": attr.string(default = "hello"),
    },
)
"#;
    a.with_context(
        crate::session::evaluator::EvalKind::BzlFile(types::Package::from("//".to_owned())),
        |a| {
            a.module("//:rules.bzl", rules_bzl_content);
        },
    );

    // Instantiation in BUILD file using standard Starlark function syntax
    a.with_context(
        crate::session::evaluator::EvalKind::Macro(types::Package::from("//".to_owned())),
        |a| {
            a.pass(
                r#"
load("//:rules.bzl", "my_rule")
my_rule(name = "my_target_instance", val = "custom_val")
"#,
            );
        },
    );

    // Retrieve target from session
    let label = LabelRef::new("//".into(), "my_target_instance");
    let toolchain = a.current_toolchain();
    let target_ref = a.session().get_target_by_label(label, toolchain);

    // Get mutable reference to Target
    let target_static: &'static Target = target_ref.into();
    let target_ptr = target_static as *const Target as *mut Target;

    let err = crate::ffi::Err::new();
    struct ErrGuard(*mut crate::ffi::Err);
    impl Drop for ErrGuard {
        fn drop(&mut self) {
            crate::ffi::Err::free(self.0);
        }
    }
    let _err_guard = ErrGuard(err);
    let err_ref = unsafe { &mut *err };

    let success = unsafe {
        run_target_rule_implementation(target_ptr, a.context().scope, a.session(), err_ref)
    };

    assert!(success, "Execution failed");
    assert!(!err_ref.has_error(), "Error: {}", err_ref.message());

    // Register target as global for verification
    a.globals_add(move |builder| {
        builder.set("my_target", target_ref);
    });

    // Verify providers
    a.pass(
        r#"
load("//:rules.bzl", "MyProvider")
assert_eq(my_target[MyProvider].val, "custom_val")
"#,
    );
}
