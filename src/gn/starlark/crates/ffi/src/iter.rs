// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::{ffi::c_void, marker::PhantomData, pin::Pin};

use crate::{bridge::SliceAny, opaque::OpaqueSized};

/// An iterator over a slice of OpaqueSized types.
pub struct ContiguousIterator<'a, T: OpaqueSized> {
    current: *mut c_void,
    end: *mut c_void,
    _marker: PhantomData<&'a mut T>,
}

impl<'a, T: OpaqueSized> ContiguousIterator<'a, T> {
    /// Creates a new `ContiguousIterator` from a `SliceAny`.
    #[inline(always)]
    pub fn new(s: SliceAny) -> Self {
        let end = unsafe { s.ptr.add(s.len * T::size()) };
        Self {
            current: s.ptr.cast::<c_void>(),
            end: end.cast::<c_void>(),
            _marker: PhantomData,
        }
    }
}

impl<'a, T: OpaqueSized> Iterator for ContiguousIterator<'a, T> {
    type Item = Pin<&'a mut T>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.end {
            let ptr = self.current;
            self.current = unsafe { self.current.add(T::size()) };
            unsafe { Some(Pin::new_unchecked(&mut *ptr.cast::<T>())) }
        } else {
            None
        }
    }
}
