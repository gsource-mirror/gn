// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use starlark::environment::GlobalsBuilder;
use starlark::environment::LibraryExtension;
use starlark::values::list::UnpackList;
use starlark::values::UnpackValue as _;

use super::*;

fn new_assert() -> testutils::Assert {
    let mut a = testutils::Assert::new();
    let mut builder = GlobalsBuilder::extended_by(&[LibraryExtension::StructType]);
    register_provider(&mut builder);
    a.globals(builder.build());
    a
}

#[test]
fn test_provider_unpack() {
    let a = new_assert();
    a.fail(
        "provider(fields=['a'])(a=1)",
        "assigned to a global variable",
    );
    a.fail(
        r#"
p = provider(fields=['a'])
p()
"#,
        "Missing named-only parameter `a`",
    );
    a.fail(
        r#"
p = provider(fields=['a'])
p(a=1, b=2)
"#,
        "Found `b` extra named parameter",
    );
    a.eq(
        r#"
p = provider(fields=['a'])
p(a=1).a
"#,
        1,
    );
    let val = a.pass(
        r#"
p1 = provider(fields = ['a'])
p2 = provider(fields = ['a'])
[p1(a=1), 1]
"#,
    );
    let val_ref = unsafe { types::util::extend_lifetime(&val) };
    let err = <UnpackList<UnpackProvider>>::unpack_param(val_ref.value()).unwrap_err();
    assert_eq!(err.to_string(), "Expected provider type");

    let providers = a
        .eval::<UnpackList<UnpackProvider>>(
            r#"
p1 = provider(fields = ['a'])
p2 = provider(fields = ['a'])
[p1(a=1), p2(a=2), p1(a=3)]
"#,
        )
        .items;
    assert_eq!(providers[0].0, providers[2].0);
    assert_ne!(providers[0].0, providers[1].0);
}

#[test]
fn test_builtin_providers() {
    let builtins = load_builtin_providers();
    let mut a = testutils::Assert::new();
    let mut builder = GlobalsBuilder::extended_by(&[LibraryExtension::StructType]);
    register_provider(&mut builder);
    builder.set("DefaultInfo", builtins.default_info);
    builder.set("GnSubstitutionInfo", builtins.gn_substitution_info);
    builder.set("GnInputsInfo", builtins.gn_inputs_info);
    builder.set("empty_default_info", builtins.empty_default_info);
    a.globals(builder.build());

    a.eq("DefaultInfo != None", true);
    a.eq("GnSubstitutionInfo != None", true);

    a.eq(
        r#"
info = GnSubstitutionInfo(substitutions = struct(foo = "bar"))
info.substitutions.foo
"#,
        "bar",
    );
}
