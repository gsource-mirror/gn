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
///
/// Note that either as_slice or iter is implemented, but not both, depending
/// on whether T is opaque.
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

impl<T: NonOpaque> Slice<T> {
    /// Returns a view of the slice as a standard Rust slice.
    #[inline(always)]
    pub fn as_slice(&mut self) -> &mut [T] {
        if self.raw.len == 0 {
            &mut []
        } else {
            // Safety: T implements NonOpaque, thus guaranteeing its size is correct.
            // (If T is opaque, rust is lied to and thinks a T is a u8, but &T remains
            // correct).
            unsafe { std::slice::from_raw_parts_mut(self.raw.ptr.cast::<T>(), self.raw.len) }
        }
    }
}

/// A std::vector<T> for which ownership has been papssed to rust.
///
/// To create an OwnedSlice, create a std::vector and call `ReleaseVector`
/// to release ownership of the slice.
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

        // Safety: Calling free is safe. The pointer is guarunteed to be valid and owned
        // by us.
        unsafe {
            free(self.slice.raw.ptr.cast::<c_void>());
        }
    }
}
