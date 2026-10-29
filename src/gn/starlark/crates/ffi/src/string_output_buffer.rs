// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::{fmt, pin::Pin};

use crate::bridge;

/// A write cursor into the pre-reserved output buffer page.
pub type Cursor = *mut u8;

/// A trait for types that can serialize themselves directly into a pre-reserved
/// output buffer.
///
/// This mirrors the C++ `Writable` concept in `src/gn/string_output_buffer.h`.
///
/// # Safety
///
/// Implementations must guarantee that `write_to_buf` never writes more than
/// `maximum_size()` bytes starting at `cursor`, and returns the advanced cursor pointer.
pub unsafe trait Writable {
    /// Returns the maximum number of bytes this value could write.
    fn maximum_size(&self) -> usize;

    /// Writes the serialized representation starting at `cursor` and returns
    /// the advanced cursor pointer.
    ///
    /// # Safety
    ///
    /// `cursor` must have at least `maximum_size()` bytes of valid writable capacity.
    unsafe fn write_to_buf(&self, cursor: Cursor) -> Cursor;
}

// Safety: `maximum_size` returns `self.len()`, and `write_to_buf` copies exactly
// `self.len()` bytes and returns the pointer advanced by `self.len()`.
unsafe impl Writable for &str {
    fn maximum_size(&self) -> usize {
        self.len()
    }

    unsafe fn write_to_buf(&self, cursor: Cursor) -> Cursor {
        // Safety: Caller guarantees `cursor` has capacity for `self.len()` bytes.
        unsafe {
            std::ptr::copy_nonoverlapping(self.as_ptr(), cursor, self.len());
            cursor.add(self.len())
        }
    }
}

// Safety: `maximum_size` returns 1, and `write_to_buf` writes exactly 1 byte
// and returns the pointer advanced by 1.
unsafe impl Writable for u8 {
    fn maximum_size(&self) -> usize {
        1
    }

    unsafe fn write_to_buf(&self, cursor: Cursor) -> Cursor {
        // Safety: Caller guarantees `cursor` has capacity for 1 byte.
        unsafe {
            cursor.write(*self);
            cursor.add(1)
        }
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

    /// Reserves at least `additional` bytes in the buffer and returns a mutable
    /// reference to the write cursor pointer.
    ///
    /// # Safety
    ///
    /// The caller is responsible for ensuring that writes do not exceed `additional`
    /// bytes and that `*cursor` is properly advanced to reflect written bytes.
    pub unsafe fn reserve(&mut self, additional: usize) -> &mut Cursor {
        // Safety: Reserve guarantees the write cursor is non-null and has at least `additional`
        // capacity.
        unsafe { &mut *bridge::Reserve(self.inner.as_mut(), additional).cast::<Cursor>() }
    }

    /// Appends a `Writable` value into the buffer.
    pub fn append<W: Writable>(&mut self, val: W) {
        // Safety: maximum_size should always be >= #bytes written to the buffer.
        let cur = unsafe { self.reserve(val.maximum_size()) };
        // Safety: Total capacity was reserved for `val`.
        *cur = unsafe { val.write_to_buf(*cur) };
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
        // Safety: total matches the maximum size reserved for all arguments.
        let cur = unsafe { buffer.reserve(total) };
        let mut cursor = *cur;
        $(
            // Safety: Total capacity was reserved for all arguments.
            cursor = unsafe { $crate::Writable::write_to_buf(&$arg, cursor) };
        )+
        *cur = cursor;
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
        buf_write!(buffer, " gn", " user", b'\n');
        assert_eq!(buffer.to_string(), "hello gn user\n");
    }
}
