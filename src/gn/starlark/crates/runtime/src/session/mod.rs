// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

pub mod evaluator;
pub mod globals;
pub mod session;
pub mod test_with_scope;

pub use evaluator::{invoke_starlark_function, EvalContext, EvalKind};
pub use session::StarlarkSession;
pub use session::BUILD_FILE_DIALECT;
pub use session::BZL_FILE_DIALECT;
pub use test_with_scope::TestWithScope;

pub use crate::ffi::value::to_cxx_value;
