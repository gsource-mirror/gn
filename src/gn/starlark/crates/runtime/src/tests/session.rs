// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::path::PathBuf;

use starlark::environment::FrozenModule;

use crate::TestWithScope;
use crate::{PackageRef, StarlarkSession};

pub fn run_starlark(path: &str) -> starlark::Result<FrozenModule> {
    let testdata_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata");
    let loader = StarlarkSession::new(testdata_path, "../../../../../".into());
    let setup = TestWithScope::new();
    let scope_ptr = setup.scope();
    // always load from the root dir.
    loader
        .load(&path, PackageRef::new("//"), scope_ptr, std::ptr::null())
        .map(|m| (*m).clone())
}

#[test]
fn test_load_dependencies() {
    let module = run_starlark("//load:root.bzl").unwrap();
    assert_eq!(module.get("root").unwrap().unpack_i32(), Some(2));
}

#[test]
fn test_cycle_detection_single_thread() {
    let res = run_starlark("//cycle:a.bzl");
    assert!(res.is_err());
    let err_msg = res.unwrap_err().to_string();
    assert!(
        err_msg.contains("cycle detected"),
        "expected cycle error, got: {}",
        err_msg
    );
}
