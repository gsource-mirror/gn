// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use attr::Session;
use types::{Label, LabelRef, PackageRef};

use crate::FakeTargetRef;

/// A mock implementation of the `Session` trait for testing.
pub struct FakeSession {
    /// The preconfigured default toolchain label.
    pub default_toolchain: Label,
    /// A map of mock targets populated for testing, indexed by (label, toolchain).
    pub targets: Mutex<HashMap<(Label, Label), FakeTargetRef>>,
    /// Recorded target dependencies registered during test runs.
    pub registered_deps: Mutex<HashMap<FakeTargetRef, HashSet<(Label, Label)>>>,
}

impl Default for FakeSession {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeSession {
    /// Creates a new `FakeSession` instance with empty targets and a preconfigured default toolchain.
    pub fn new() -> Self {
        Self {
            default_toolchain: Label::new(
                PackageRef::root().to_owned(),
                "default_toolchain".to_owned(),
            ),
            targets: Mutex::new(HashMap::new()),
            registered_deps: Mutex::new(HashMap::new()),
        }
    }

    /// Helper to insert a target under the default toolchain.
    pub fn insert_target(&self, label: Label, target: FakeTargetRef) {
        self.targets
            .lock()
            .unwrap()
            .insert((label, self.default_toolchain.clone()), target);
    }
}

impl Session for FakeSession {
    type TargetRef = FakeTargetRef;

    fn get_target(&self, label: LabelRef<'_>, current_toolchain: LabelRef<'_>) -> Self::TargetRef {
        self.targets
            .lock()
            .unwrap()
            .get(&(label.to_owned(), current_toolchain.to_owned()))
            .cloned()
            .unwrap_or_default()
    }

    fn register_dependency<'a>(
        &self,
        source: Self::TargetRef,
        target: LabelRef<'a>,
        toolchain: LabelRef<'a>,
    ) {
        self.registered_deps
            .lock()
            .unwrap()
            .entry(source)
            .or_default()
            .insert((target.to_owned(), toolchain.to_owned()));
    }
}
