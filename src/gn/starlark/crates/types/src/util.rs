// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

/// Like std::mem::transmute, but can only affect lifetime.
pub unsafe fn extend_lifetime<'to, 'from: 'from, T: ?Sized>(val: &'from T) -> &'to T {
    unsafe { std::mem::transmute(val) }
}

/// Like std::mem::transmute, but limited to only adding mutability.
#[allow(invalid_reference_casting)]
pub unsafe fn add_mut<'to, 'from, T: ?Sized>(val: &'from T) -> &'to mut T {
    unsafe { &mut *(val as *const T as *mut T) }
}

#[allow(dead_code)]
fn dummy_to_force_cxx_linking() {
    cxx::let_cxx_string!(s = "dummy");
    let _ = s;
}
