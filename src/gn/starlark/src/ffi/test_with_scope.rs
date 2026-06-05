// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use crate::ffi::bindings as ffi;

/// A wrapper around C++ `TestWithScope`, which manages the lifetime of a C++
/// GN build environment (including Settings, BuildSettings, and Toolchain) for testing.
pub struct TestWithScope {
    ptr: *mut ffi::TestWithScope,
}

impl TestWithScope {
    /// Creates a new C++ `TestWithScope` environment.
    /// Sets the source root path to `testdata` and the build directory to `//out/Default`.
    pub fn new() -> Self {
        Self {
            ptr: ffi::NewTestWithScope(
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("testdata")
                    .to_str()
                    .unwrap(),
                "//out/Default",
            ),
        }
    }

    /// Retrieves a raw pointer to the underlying C++ `Scope` object.
    pub fn scope(&self) -> *mut ffi::Scope {
        unsafe { ffi::GetScopeFromTestWithScope(std::pin::Pin::new_unchecked(&mut *self.ptr)) }
    }

    
}

impl Drop for TestWithScope {
    fn drop(&mut self) {
        unsafe { ffi::FreeTestWithScope(self.ptr); }
    }
}
