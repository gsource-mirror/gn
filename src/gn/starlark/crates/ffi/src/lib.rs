// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

//! Low-level FFI bindings and types for interoperating between the C++
//! GN codebase and the Rust Starlark interpreter crates using `cxx`.
//!
//! **Architecture & Bridge design**
//!
//! Rather than manually compiling raw `extern "C"` FFI wrappers, all FFI
//! boundary mappings are consolidated within `bridge.rs` under a single
//! `#[cxx::bridge]` module.
//!
//! This module is then transpiled with cxxbridge into a C++ header and source
//! file.
//!
//! Safe APIs for C++ types are then exposed in the impl functions for each of
//! these types in their own files.
mod bridge;
mod label;
mod output_file;
mod scope;
mod settings;
mod test_with_scope;

pub use bridge::{Label, OutputFile, Scope, Settings, SourceDir};
pub use test_with_scope::TestWithScope;
