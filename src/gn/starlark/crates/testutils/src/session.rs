// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use attr::Session;
use types::{Label, PackageRef, util::extend_lifetime};

use crate::FakeTargetRef;

/// A fake implementation of the `Session` trait for testing.
pub struct FakeSession {
    /// The preconfigured default toolchain label.
    pub default_toolchain: Label,
    /// Builtin rules registered for the session.
    builtin_rules: Vec<&'static rule::Rule<'static, crate::FakeEvalContext>>,
    /// The module associated with the default rules.
    /// Never used, just an RAII guard for builtin_rules.
    _module: starlark::environment::FrozenModule,
}

impl Default for FakeSession {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeSession {
    /// Creates a new `FakeSession` instance with a preconfigured default toolchain.
    pub fn new() -> Self {
        let module = rule::register_builtin_rules::<crate::FakeEvalContext>();
        Self {
            default_toolchain: Label::new(
                PackageRef::root().to_owned(),
                "default_toolchain".to_owned(),
            ),
            // Safety: The module lives as long as the session lasts, which is as long as the test
            // lasts, which is 'static.
            builtin_rules: rule::extract_builtin_rules(unsafe { extend_lifetime(&module) }),
            _module: module,
        }
    }

    /// Returns the default rule used for testing.
    pub fn default_rule(&self) -> &'static rule::Rule<'static, crate::FakeEvalContext> {
        self.builtin_rules[types::OutputType::SourceSet as usize]
    }
}

impl Session for FakeSession {
    type TargetRef = FakeTargetRef;

    fn run_builtin_rule<'v>(
        &self,
        heap: &starlark::values::Heap<'v>,
        _target: &Self::TargetRef,
        _state: &mut types::CtxState<Self::TargetRef>,
    ) -> starlark::values::Value<'v> {
        heap.alloc(Vec::<starlark::values::Value<'v>>::new())
    }
}
