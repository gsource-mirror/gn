// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::{pin::Pin, ptr::NonNull};

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

/// Converts a `NonNull<T>` into a `Pin<&mut T>`.
pub fn pin_mut_ptr<'a, T: ?Sized>(mut ptr: NonNull<T>) -> Pin<&'a mut T> {
    // Safety: Pointers managed in FFI contexts are non-null, valid, and pinned.
    unsafe { Pin::new_unchecked(ptr.as_mut()) }
}

/// Dereferences a `NonNull<T>` into a shared reference.
pub fn deref_ptr<'a, T: ?Sized>(ptr: NonNull<T>) -> &'a T {
    // Safety: Pointers managed in FFI contexts are non-null and valid for the
    // context duration.
    unsafe { ptr.as_ref() }
}

/// Converts a `Pin<&mut T>` into a `NonNull<T>`.
pub fn as_non_null<T: ?Sized>(mut pin: Pin<&mut T>) -> NonNull<T> {
    // Safety: Pinned references are guaranteed to be non-null and point to valid
    // memory.
    unsafe { NonNull::new_unchecked(pin.as_mut().get_unchecked_mut()) }
}
