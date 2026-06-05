// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#![allow(improper_ctypes_definitions, private_interfaces)]

pub use attr;
pub mod ctx;
pub mod errors;
pub mod ninja;
pub mod rule;
pub mod session;
pub mod target;
pub mod target_ref;
pub mod testing;

pub use errors::Error;
pub use session::{
    invoke_starlark_function, to_cxx_value, EvalContext, EvalKind, StarlarkSession, TestWithScope,
};
pub use target::Target;
pub use target_ref::TargetRef;
pub use testing::Assert;
pub mod ffi;

// Re-export type and depset dependencies for convenience
pub use depset::{Depset, FrozenDepset, Kind, Order};
pub use types::{File, Label, LabelRef, Package, PackageRef};

#[cfg(test)]
mod tests;
