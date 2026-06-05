// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use allocative::Allocative;
use attr::Attr;
use starlark::{
    starlark_simple_value,
    values::{ProvidesStaticType, StarlarkValue},
};
use starlark_derive::{starlark_value, NoSerialize};
use types::{File, Label, TargetRef};

/// A fake target struct for testing.
#[derive(Debug, Allocative, Default)]
pub struct FakeTarget {
    /// A list of fake files returned as outputs of the target.
    pub outputs: Vec<File>,
    /// A list of attributes.
    pub attrs: Vec<Attr>,
    /// Registered target dependencies.
    #[allocative(skip)]
    pub dependencies: Mutex<HashSet<(Label, Label)>>,
}

/// A reference to a fake target.
#[derive(Debug, ProvidesStaticType, NoSerialize, Allocative, Clone)]
pub struct FakeTargetRef(#[allocative(skip)] Arc<FakeTarget>);

impl Default for FakeTargetRef {
    fn default() -> Self {
        Self::new(FakeTarget::default())
    }
}

impl FakeTargetRef {
    /// Creates a new `FakeTargetRef` containing the given `FakeTarget`.
    pub fn new(target: FakeTarget) -> Self {
        Self(Arc::new(target))
    }

    /// Returns a shared reference to the underlying target.
    pub fn get(&self) -> &FakeTarget {
        &self.0
    }

    /// Returns the registered dependencies of this target.
    pub fn registered_deps(&self) -> HashSet<(Label, Label)> {
        self.get().dependencies.lock().unwrap().clone()
    }
}

impl PartialEq for FakeTargetRef {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for FakeTargetRef {}

impl std::hash::Hash for FakeTargetRef {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.0).hash(state);
    }
}

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
        self.get().outputs.clone()
    }

    fn target_out_dir(&self, prefix: &str, suffix: &str, _separator: &str) -> String {
        format!("{prefix}$TOOLCHAIN/{suffix}$LABEL")
    }
}
