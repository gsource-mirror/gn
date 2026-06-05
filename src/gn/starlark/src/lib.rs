// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.
// Dummy comment to trigger cargo rebuild 39

pub mod action;
pub mod attr;
pub mod ctx;
pub mod eval_context;
pub mod depset;
pub mod ffi;
pub mod file;
pub mod globals;
pub mod label;
pub mod package;
pub mod session;
pub mod provider;
pub mod rule;
pub mod target;
pub mod util;
pub mod errors;

pub(crate) use errors::Error;
pub use session::StarlarkSession;
pub use label::{Label, LabelRef};
pub use target::{Target, TargetRef};
pub use package::{Package, PackageRef};
pub use file::File;

#[cfg(test)]
pub mod testing;
#[cfg(test)]
pub use testing::Assert;
