// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use cxx::CxxString;

/// Extension trait for working with raw pointers returned by C++ functions.
pub trait PointerExt<Ref> {
    /// Dereferences the pointer.
    /// The caller is responsible for ensuring that it is not null.
    fn non_null(self) -> Ref;
}

impl<'a, T> PointerExt<&'a T> for *const T {
    fn non_null(self) -> &'a T {
        debug_assert!(!self.is_null());
        // Safety: Caller is responsible
        unsafe { &*self }
    }
}

impl<'a, T> PointerExt<&'a mut T> for *mut T {
    fn non_null(self) -> &'a mut T {
        debug_assert!(!self.is_null());
        // Safety: Caller is responsible
        unsafe { &mut *self }
    }
}

impl<'a, T> PointerExt<&'a T> for *mut T {
    fn non_null(self) -> &'a T {
        debug_assert!(!self.is_null());
        // Safety: Caller is responsible
        unsafe { &*self }
    }
}

pub trait CxxStringExt {
    fn as_str<'a>(&'a self) -> &'a str;
}

impl CxxStringExt for CxxString {
    fn as_str<'a>(&'a self) -> &'a str {
        // Safety: was a valid C++ string.
        unsafe { std::str::from_utf8_unchecked(self.as_bytes()) }
    }
}
