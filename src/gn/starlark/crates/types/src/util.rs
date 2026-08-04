// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::ptr::NonNull;

/// Like std::mem::transmute, but can only affect lifetime.
///
/// # Safety
///
/// The caller must ensure that the returned reference is not used after the
/// underlying data is dropped.
pub unsafe fn extend_lifetime<'to, T: ?Sized>(val: &T) -> &'to T {
    // Safety: Transmuting lifetime of reference is unsafe, safety is guaranteed by
    // the caller.
    unsafe { std::mem::transmute(val) }
}

/// Dereferences a `NonNull<T>` into a shared reference.
pub fn deref_ptr<'a, T: ?Sized>(ptr: NonNull<T>) -> &'a T {
    // Safety: Pointers managed in FFI contexts are non-null and valid for the
    // context duration.
    unsafe { ptr.as_ref() }
}
