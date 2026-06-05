// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::{
    ffi::c_void,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use crate::{
    iter::ContiguousIterator,
    opaque::{NonOpaque, OpaqueSized},
};

/// A &[T]-like object.
///
/// Use this for one of two reasons:
/// * C++ returns a slice of an opaque type, for which &[T] does not work.
/// * C++ returns a std::vector<T>, in which case you should use
///   `OwnedSlice<T>`.
pub struct Slice<T> {
    raw: crate::bridge::SliceAny,
    _marker: PhantomData<T>,
}

impl<T> From<crate::bridge::SliceAny> for Slice<T> {
    #[inline(always)]
    fn from(raw: crate::bridge::SliceAny) -> Self {
        Self {
            raw,
            _marker: PhantomData,
        }
    }
}

impl<T: OpaqueSized> Slice<T> {
    /// Returns an iterator over the elements of the slice.
    #[inline(always)]
    pub fn iter(&self) -> ContiguousIterator<'_, T> {
        ContiguousIterator::new(self.raw)
    }
}

/// A Vec<T> owned by rust.
///
/// To create an OwnedSlice, create a std::vector and call `ReleaseVector`
/// to release ownership of the slice.
///
/// For opaque types, the only method available is iter().
/// For non-opaque types, the only method available is as_slice().
pub struct OwnedSlice<T> {
    slice: Slice<T>,
}

impl<T> From<crate::bridge::SliceAny> for OwnedSlice<T> {
    #[inline(always)]
    fn from(raw: crate::bridge::SliceAny) -> Self {
        Self {
            slice: Slice::from(raw),
        }
    }
}

impl<T> Deref for OwnedSlice<T> {
    type Target = Slice<T>;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.slice
    }
}

impl<T> DerefMut for OwnedSlice<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.slice
    }
}

impl<T> Drop for OwnedSlice<T> {
    #[inline(always)]
    fn drop(&mut self) {
        // We don't write this function, this is the libc free function.
        extern "C" {
            fn free(ptr: *mut c_void);
        }

        // Safety: casting to c_void is safe.
        unsafe {
            free(self.slice.raw.ptr.cast::<c_void>());
        }
    }
}

impl<T: NonOpaque> Slice<T> {
    /// Returns a read-only view of the slice as a standard Rust slice.
    #[inline(always)]
    pub fn as_slice(&mut self) -> &mut [T] {
        if self.raw.len == 0 {
            &mut []
        } else {
            // Safety: T implements SafeToSlice, guaranteeing its size and layout
            // are safe for standard slice construction.
            unsafe { std::slice::from_raw_parts_mut(self.raw.ptr.cast::<T>(), self.raw.len) }
        }
    }
}
