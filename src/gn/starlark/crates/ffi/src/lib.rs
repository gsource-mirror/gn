// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

//! Low-level FFI bindings and types for interoperating between the C++
//! GN codebase and the Rust Starlark interpreter crates using `cxx`.

mod bridge;
mod label;
mod output_file;
mod scope;
mod settings;
mod test_with_scope;
mod value;

pub mod iter;

pub use bridge::{Label, OutputFile, Scope, ScopePair, Settings, SourceDir, Value, ValueType};
pub use test_with_scope::TestWithScope;

pub mod slice;
pub use slice::{OwnedSlice, Slice};

pub mod opaque;
pub use opaque::{NonOpaque, OpaqueSized};
