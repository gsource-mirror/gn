// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Mutex;

use types::{Label, LabelRef, Package};
use attr::Session;

use crate::{FakeTarget, FakeTargetRef};

/// A mock implementation of the `Session` trait for testing.
pub struct FakeSession {
    /// The preconfigured default toolchain label.
    pub default_toolchain: Label,
    /// A map of mock targets populated for testing, indexed by (label, toolchain).
    pub targets: Mutex<HashMap<(Label, Label), FakeTargetRef>>,
    /// Recorded target dependencies registered during test runs.
    pub registered_deps: Mutex<Vec<(FakeTargetRef, Label, Label)>>,
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
                Package::from("//".to_owned()),
                "default_toolchain".to_owned(),
            ),
            targets: Mutex::new(HashMap::new()),
            registered_deps: Mutex::new(Vec::new()),
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
        let targets = self.targets.lock().unwrap();
        targets
            .get(&(label.to_owned(), current_toolchain.to_owned()))
            .cloned()
            .unwrap_or_else(|| FakeTargetRef(Rc::new(FakeTarget { outputs: vec![] })))
    }

    fn register_dependency<'a>(&self, source: Self::TargetRef, target: LabelRef<'a>, toolchain: LabelRef<'a>) {
        self.registered_deps.lock().unwrap().push((source, target.to_owned(), toolchain.to_owned()));
    }
}
