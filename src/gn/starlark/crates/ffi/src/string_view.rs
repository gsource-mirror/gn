// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

/// std::string_view is a complex type that involves templates. Thus, it is
/// opaque and cannot be passed by value. Thus, we cannot use FFI functions
/// that involve string_view.
///
/// To bypass this, we tell cxx "the C++ string_view type is really this type".
/// This layout is not guarunteed but should in practice always hold. We have
/// a unittest in ffi_unittest.cc to verify this.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct StringView {
    len: usize,
    ptr: *const u8,
}

// Safety: We assert that StringView matches C++ std::string_view in layout and
// ABI.
unsafe impl cxx::ExternType for StringView {
    type Id = cxx::type_id!("std::string_view");
    type Kind = cxx::kind::Trivial;
}

impl StringView {
    /// Converts the string_view to a rust string slice.
    pub fn as_str<'a>(&self) -> &'a str {
        // Rust slices require non-null pointers. std::string_view can be {len=0,
        // ptr=null}.
        if self.len == 0 {
            ""
        } else {
            // Safety: was a valid string_view, so should be a valid &str
            unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(self.ptr, self.len)) }
        }
    }
}
