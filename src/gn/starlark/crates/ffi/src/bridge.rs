// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

/// The consolidated cxx FFI bridge defining all shared C++ classes, structs,
/// methods, and constructors utilized by the high-level Rust wrappers.
///
/// This file does several things:
/// * It generates types usable by rust.
/// * The `cxxbridge --header` command can be ran to re-generate the C++
///   headers.
///   * This allows for C++ code to #include rust types
/// * The `cxxbridge` command generates shims to allow us to use C++ types in
///   rust.
#[cxx::bridge]
// CxxBridge requires a module, but we don't want one. So we make a private one
// and re-export all fields.
mod dummy {
    unsafe extern "C++" {
        // include! simply tells cxxbridge to put the #include in the generated C++
        // source code. It does not do anything on the rust side.
        include!("gn/ffi/test_with_scope.h");
        include!("gn/label.h");
        include!("gn/output_file.h");
        include!("gn/scope.h");
        include!("gn/settings.h");
        include!("gn/source_dir.h");
        include!("gn/test_with_scope.h");

        type OutputFile;
        #[cxx_return_type = "std::string_view"]
        pub(in crate::output_file) fn value(self: &OutputFile) -> &str;

        type SourceDir;
        #[cxx_return_type = "std::string_view"]
        pub(in crate::label) fn SourceWithNoTrailingSlash(self: &SourceDir) -> &str;

        type Label;
        pub(in crate::label) fn dir(self: &Label) -> &SourceDir;
        #[cxx_return_type = "const std::string&"]
        pub fn name(self: &Label) -> &str;

        type Settings;
        pub(in crate::settings) fn toolchain_label(self: &Settings) -> &Label;

        type Scope;
        #[rust_name = "settings_cxx"]
        pub(crate) fn settings(self: &Scope) -> *const Settings;

        type TestWithScope;
        pub(in crate::test_with_scope) fn NewTestWithScope() -> UniquePtr<TestWithScope>;
        #[rust_name = "scope_cxx"]
        pub(in crate::test_with_scope) fn scope(self: Pin<&mut TestWithScope>) -> *mut Scope;
    }
}

pub use dummy::*;
