// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use crate::{declare_opaque_type, CxxOwned, Scope};

declare_opaque_type!(pub TestWithScope);

impl TestWithScope {
    pub fn new() -> CxxOwned<&'static mut Self> {
        extern "C" {
            fn NewTestWithScope() -> CxxOwned<&'static mut TestWithScope>;
        }
        // Safety: Just an FFI function
        unsafe { NewTestWithScope() }
    }

    pub fn scope(&mut self) -> &mut Scope {
        extern "C" {
            fn GetScopeFromTestWithScope(setup: &mut TestWithScope) -> &mut Scope;
        }
        // Safety: Just an FFI function
        unsafe { GetScopeFromTestWithScope(self) }
    }
}

impl crate::types::CxxDrop for &mut TestWithScope {
    fn drop(&mut self) {
        extern "C" {
            fn FreeTestWithScope(setup: &mut TestWithScope);
        }
        // Safety: Just an FFI function
        unsafe { FreeTestWithScope(*self) }
    }
}
