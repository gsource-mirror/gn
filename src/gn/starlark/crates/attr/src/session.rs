// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

/// Re-export Session and EvalContext from types so caller crates can access them seamlessly.
pub use types::{Session, EvalContext};

/// Extension trait for EvalContext to support target creation.
/// This is defined in the attr crate because it depends on the concrete Attr type.
pub trait EvalContextExt: types::EvalContext {
    /// Creates and registers a new target on the C++ side and wraps it in Rust.
    fn create_starlark_target(
        &self,
        target_name: &str,
        rule: starlark::values::FrozenValue,
        attrs: Vec<crate::Attr>,
    ) -> Result<<Self::Session as types::Session>::TargetRef, types::Error>;
}

/// Extension trait to allow rule invocation to set attributes on a target.
pub trait TargetRefExt {
    fn set_attrs(&self, attrs: Vec<crate::Attr>);
}


