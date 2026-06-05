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
    struct Any {
        _private: u8,
    }

    #[derive(Clone, Copy)]
    struct SliceAny {
        len: usize,
        ptr: *mut Any,
    }

    struct ScopePair<'a> {
        key: &'a str,
        value: &'a Value,
    }
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
        include!("gn/value.h");

        include!("gn/ffi/value.h");
        include!("gn/ffi/scope.h");

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
        // Constructs a new child Scope, populates placeholder Values for the given
        // keys, and returns OwnedSlice<&Value>.
        // Fills the new scope in out_scope.
        pub(in crate::scope) fn NewScope(
            parent_scope: &Scope,
            keys: &[&str],
            out_scope: &mut UniquePtr<Scope>,
        ) -> SliceAny;
        // Returns an OwnedSlice<ScopePair>
        pub(in crate::scope) fn GetScopeItems(scope: &Scope) -> SliceAny;
        #[rust_name = "settings_cxx"]
        pub(in crate::scope) fn settings(self: &Scope) -> *const Settings;

        // Test helper

        type TestWithScope;
        pub(in crate::test_with_scope) fn NewTestWithScope() -> UniquePtr<TestWithScope>;
        #[rust_name = "scope_cxx"]
        pub(in crate::test_with_scope) fn scope(self: Pin<&mut TestWithScope>) -> *mut Scope;

        type Value;
        type ParseNode;
        #[allow(dead_code)]
        pub(in crate::value) fn NewValueForTesting() -> UniquePtr<Value>;
        #[cxx_name = "GetValueType"]
        pub(in crate::value) fn type_cxx(val: &Value) -> u8;
        pub(in crate::value) fn boolean_value(self: &Value) -> &bool;
        pub(in crate::value) fn int_value(self: &Value) -> &i64;
        pub(in crate::value) fn string_value(self: &Value) -> &CxxString;
        #[cxx_name = "GetValueList"]
        pub(in crate::value) fn list_value_cxx(val: &Value) -> SliceAny;
        pub(in crate::value) fn scope_value(self: &Value) -> *const Scope;
        // Returns sizeof(Value) dynamically
        pub(in crate::value) fn ValueSize() -> usize;
        pub(in crate::value) unsafe fn SetValueNone(val: Pin<&mut Value>, origin: *const ParseNode);
        pub(in crate::value) unsafe fn SetValueBool(
            val: Pin<&mut Value>,
            origin: *const ParseNode,
            b: bool,
        );
        pub(in crate::value) unsafe fn SetValueInt(
            val: Pin<&mut Value>,
            origin: *const ParseNode,
            i: i64,
        );
        pub(in crate::value) unsafe fn SetValueString(
            val: Pin<&mut Value>,
            origin: *const ParseNode,
            s: &str,
        );
        // Initialises self as a list of `size` elements and returns a pointer to the
        // start.
        pub(in crate::value) unsafe fn SetValueList(
            val: Pin<&mut Value>,
            origin: *const ParseNode,
            size: usize,
        ) -> *mut u8;
        pub(in crate::value) unsafe fn SetValueScope(
            val: Pin<&mut Value>,
            origin: *const ParseNode,
            scope: UniquePtr<Scope>,
        );
    }
}

pub use dummy::*;
