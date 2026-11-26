// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

/// Re-export the traits from types so caller crates can access them seamlessly.
pub use types::{EvalContext, OutputType, Session, TargetRef};

/// Represents a target with attributes that can be executed by custom Starlark
/// rules.
pub trait TargetAttrExt: TargetRef {
    /// Returns the resolved custom attributes of the target.
    fn attrs(&self) -> &[crate::Attr];

    /// Returns an iterator over the resolved dependencies of this target in
    /// attribute traversal order.
    fn deps(&self) -> impl Iterator<Item = Self> + '_;
}

/// Extension trait for EvalContext to support target creation.
pub trait EvalContextAttrExt: types::EvalContext {
    /// Creates and registers a new target.
    fn create_target(
        &self,
        target_type: Option<OutputType>,
        target_name: &str,
        scope: std::pin::Pin<&mut Self::Scope>,
        rule: starlark::values::Value<'static>,
        attrs: Vec<crate::Attr>,
    ) -> starlark::Result<()>;
}
