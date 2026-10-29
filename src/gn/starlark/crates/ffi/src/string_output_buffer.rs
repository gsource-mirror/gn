// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::{fmt, pin::Pin};

use crate::bridge;

/// A reference to the write cursor pointer of an output buffer guaranteed
/// to have sufficient capacity for writing without bounds checks.
pub struct BigEnoughBuffer<'a>(pub &'a mut *mut u8);

impl<'a> BigEnoughBuffer<'a> {
    /// Writes a slice of bytes into the buffer and advances the underlying
    /// pointer in place.
    pub fn write_slice(&mut self, slice: &[u8]) {
        // Safety: This buffer is guaranteed to be big enough.
        unsafe {
            std::ptr::copy_nonoverlapping(slice.as_ptr(), *self.0, slice.len());
            *self.0 = (*self.0).add(slice.len());
        }
    }

    /// Writes a Copy value into the buffer and advances the underlying
    /// pointer in place.
    pub fn write<T: Copy>(&mut self, val: T) {
        // Safety: This buffer is guaranteed to be big enough.
        unsafe {
            (*self.0).cast::<T>().write_unaligned(val);
            *self.0 = (*self.0).add(std::mem::size_of::<T>());
        }
    }
}

/// A trait for types that can serialize themselves directly into a pre-reserved
/// output buffer.
///
/// This mirrors the C++ `Writable` concept in `src/gn/string_output_buffer.h`.
pub trait Writable {
    /// Returns the maximum number of bytes this value could write.
    fn maximum_size(&self) -> usize;

    /// Writes the serialized representation into `buf`.
    fn write_to_buf(&self, buf: &mut BigEnoughBuffer<'_>);
}

impl Writable for &str {
    fn maximum_size(&self) -> usize {
        self.len()
    }

    fn write_to_buf(&self, buf: &mut BigEnoughBuffer<'_>) {
        buf.write_slice(self.as_bytes());
    }
}

impl Writable for char {
    fn maximum_size(&self) -> usize {
        1
    }

    fn write_to_buf(&self, buf: &mut BigEnoughBuffer<'_>) {
        buf.write(*self as u8);
    }
}

/// Safe wrapper around a C++ `StringOutputBuffer`.
pub struct StringOutputBuffer<'a> {
    inner: Pin<&'a mut bridge::StringOutputBuffer>,
}

impl<'a> StringOutputBuffer<'a> {
    /// Creates a new wrapper around a pinned C++ `StringOutputBuffer`.
    pub fn new(inner: Pin<&'a mut bridge::StringOutputBuffer>) -> Self {
        Self { inner }
    }

    /// Reserves at least `additional` bytes in the buffer and returns a write cursor.
    pub fn reserve(&mut self, additional: usize) -> BigEnoughBuffer<'_> {
        // Safety: Reserve guarantees the write cursor is non-null and has at least `additional`
        // capacity.
        unsafe { BigEnoughBuffer(&mut *bridge::Reserve(self.inner.as_mut(), additional)) }
    }

    /// Appends a `Writable` value into the buffer.
    pub fn append<W: Writable>(&mut self, val: W) {
        val.write_to_buf(&mut self.reserve(val.maximum_size()));
    }
}

impl<'a> fmt::Display for StringOutputBuffer<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&bridge::StringOutputBufferToString(&self.inner))
    }
}

/// Writes one or multiple `Writable` values into an output buffer.
///
/// Works the same as chaining appends, but only calls reserve once.
#[macro_export]
macro_rules! buf_write {
    ($buffer:expr, $($arg:expr),+ $(,)?) => {{
        let buffer: &mut $crate::StringOutputBuffer<'_> = &mut ($buffer);
        let total = $($crate::Writable::maximum_size(&$arg) +)+ 0;
        let mut cursor = buffer.reserve(total);
        $($crate::Writable::write_to_buf(&$arg, &mut cursor);)+
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_output_buffer() {
        let mut raw_buf = bridge::NewStringOutputBuffer();
        let mut buffer = StringOutputBuffer::new(raw_buf.pin_mut());
        buffer.append("hello");
        buf_write!(buffer, " gn", " user", '\n');
        assert_eq!(buffer.to_string(), "hello gn user\n");
    }
}
