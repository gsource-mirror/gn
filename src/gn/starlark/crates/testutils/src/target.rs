// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::rc::Rc;

use allocative::Allocative;
use starlark::starlark_simple_value;
use starlark::values::{ProvidesStaticType, StarlarkValue};
use starlark_derive::{starlark_value, NoSerialize};
use types::File;
use types::TargetRef;

/// A mock target struct for testing.
#[derive(Debug, Clone, PartialEq, Allocative)]
pub struct FakeTarget {
    /// A list of mock files returned as outputs of the target.
    pub outputs: Vec<File>,
}

/// A reference to a mock target.
#[derive(Debug, ProvidesStaticType, NoSerialize, Allocative, Clone, PartialEq)]
pub struct FakeTargetRef(pub Rc<FakeTarget>);

unsafe impl Send for FakeTargetRef {}
unsafe impl Sync for FakeTargetRef {}

starlark_simple_value!(FakeTargetRef);

impl std::fmt::Display for FakeTargetRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FakeTargetRef")
    }
}

#[starlark_value(type = "Target")]
impl<'v> StarlarkValue<'v> for FakeTargetRef {}

impl TargetRef for FakeTargetRef {
    fn outputs(&self) -> Vec<File> {
        self.0.outputs.clone()
    }

    fn target_out_dir(&self, prefix: &str, suffix: &str, separator: &str) -> String {
        format!("{prefix}{separator}fake_out_dir{separator}{suffix}")
    }
}
