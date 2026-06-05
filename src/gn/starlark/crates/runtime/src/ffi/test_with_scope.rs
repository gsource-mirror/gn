// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use crate::ffi::scope::Scope;

declare_opaque_type!(TestWithScope);

impl TestWithScope {
    pub fn new(root_path: &str, build_dir: &str) -> *mut TestWithScope {
        extern "C" {
            fn NewTestWithScope(root_path: &str, build_dir: &str) -> *mut TestWithScope;
        }
        unsafe { NewTestWithScope(root_path, build_dir) }
    }

    pub fn free(ptr: *mut TestWithScope) {
        extern "C" {
            fn FreeTestWithScope(setup: *mut TestWithScope);
        }
        unsafe {
            FreeTestWithScope(ptr);
        }
    }

    pub fn scope(ptr: &mut TestWithScope) -> &mut Scope {
        extern "C" {
            fn GetScopeFromTestWithScope(setup: &mut TestWithScope) -> &mut Scope;
        }
        unsafe { GetScopeFromTestWithScope(ptr) }
    }
}
