// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

#[macro_use]
pub mod bindings;
pub mod err;
pub mod label;
pub mod output_file;
pub mod result;
pub mod scope;
pub mod settings;
pub mod source_dir;
pub mod target;
pub mod test_with_scope;
pub mod value;

// Rust-side FFI implementations
pub mod session;
pub mod starlark_module;

// Opaque types with no methods defined directly here
declare_opaque_type!(ParseNode);
declare_opaque_type!(SourceFile);

// Re-exports
pub use bindings::Pair;
pub(crate) use err::Err;
pub use result::{handle_result, handle_result_with_message};
pub(crate) use scope::Scope;
pub(crate) use settings::Settings;
pub(in crate::ffi) use source_dir::SourceDir;
pub(crate) use target::Target;
pub(crate) use test_with_scope::TestWithScope;
pub(crate) use value::GnValue;
