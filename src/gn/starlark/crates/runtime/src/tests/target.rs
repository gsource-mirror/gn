// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use crate::Assert;
use crate::LabelRef;

#[test]
fn test_target_in_operator() {
    let mut a = Assert::new();
    let label = LabelRef::new("//".into(), "default_target");
    let toolchain = a.current_toolchain();
    let target = a.session().get_target_by_label(label, toolchain);

    a.globals_add(move |builder| {
        builder.set("my_target", target);
    });

    a.pass(
        r#"
res1 = DefaultInfo in my_target
assert_eq(res1, True)

MyProvider = provider(fields = [])
res2 = MyProvider in my_target
assert_eq(res2, False)

res3 = 'not_a_provider' in my_target
assert_eq(res3, False)
"#,
    );
}
