// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use crate::{declare_opaque_type, CxxOwned, Scope};

declare_opaque_type!(pub TestWithScope);

impl TestWithScope {
    pub fn new() -> CxxOwned<&'static Self> {
        extern "C" {
            fn NewTestWithScope() -> CxxOwned<&'static TestWithScope>;
        }
        // Safety: Just an FFI function
        unsafe { NewTestWithScope() }
    }

    pub fn scope(&self) -> &mut Scope {
        extern "C" {
            fn GetScopeFromTestWithScope(setup: &TestWithScope) -> &mut Scope;
        }
        // Safety: Just an FFI function
        unsafe { GetScopeFromTestWithScope(self) }
    }
}

impl crate::types::CxxDrop for &'static TestWithScope {
    fn drop(&mut self) {
        extern "C" {
            fn FreeTestWithScope(setup: CxxOwned<&'static TestWithScope>);
        }
        // Safety: Just an FFI function
        unsafe { FreeTestWithScope(CxxOwned(*self)) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_with_scope_creation() {
        let setup = TestWithScope::new();
        let _scope = setup.scope();
    }
}
