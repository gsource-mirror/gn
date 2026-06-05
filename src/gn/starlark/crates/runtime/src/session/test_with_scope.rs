// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use crate::ffi;
use crate::StarlarkSession;

/// A wrapper around C++ `TestWithScope`, which manages the lifetime of a C++
/// GN build environment (including Settings, BuildSettings, and Toolchain) for testing.
pub struct TestWithScope {
    ptr: *mut ffi::TestWithScope,
}

impl TestWithScope {
    /// Creates a new C++ `TestWithScope` environment.
    /// Sets the source root path to `testdata` and the build directory to `//out/Default`.
    pub fn new() -> Self {
        let testdata_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata");
        let testdata_str = testdata_path.to_str().unwrap();

        Self {
            ptr: ffi::TestWithScope::new(testdata_str, "//out/Default"),
        }
    }

    /// Retrieves a raw pointer to the underlying C++ `Scope` object.
    pub fn scope(&self) -> *mut ffi::Scope {
        unsafe { ffi::TestWithScope::scope(&mut *self.ptr) as *mut ffi::Scope }
    }

    /// Returns a reference to the `StarlarkSession` associated with this test scope.
    pub fn session(&self) -> &StarlarkSession {
        unsafe { (&*self.scope()).starlark_session() }
    }
}

impl Drop for TestWithScope {
    fn drop(&mut self) {
        ffi::TestWithScope::free(self.ptr);
    }
}
