// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use types::File;

pub use crate::bridge::EscapingPlatform;
use crate::{BigEnoughBuffer, Writable, bridge};

/// Writable wrapper that escapes strings for Ninja syntax.
///
/// In Ninja, '$', ':', and ' ' are escaped with a '$' prefix.
pub struct Ninja<'a>(pub &'a str);

impl From<File> for Ninja<'static> {
    fn from(file: File) -> Self {
        Self(file.as_str())
    }
}

impl<'a> Writable for Ninja<'a> {
    fn maximum_size(&self) -> usize {
        self.0.len() * 2
    }

    fn write_to_buf(&self, buf: &mut BigEnoughBuffer<'_>) {
        // Safety: bridge::EscapeNinja writes at most maximum_size() bytes,
        // and buf is guaranteed to have at least maximum_size() capacity.
        unsafe {
            *buf.0 = bridge::EscapeNinja(self.0, *buf.0);
        }
    }
}

/// Writable wrapper that escapes strings for command lines executed in Ninja.
///
/// This escapes for both Ninja and the shell so the string is treated as a single token.
pub struct Command<'a>(pub &'a str, pub EscapingPlatform);

impl<'a> Writable for Command<'a> {
    fn maximum_size(&self) -> usize {
        self.0.len() * 3 + 2
    }

    fn write_to_buf(&self, buf: &mut BigEnoughBuffer<'_>) {
        // Safety: bridge::EscapeNinjaCommand writes at most maximum_size() bytes,
        // and buf is guaranteed to have at least maximum_size() capacity.
        unsafe {
            *buf.0 = bridge::EscapeNinjaCommand(self.0, self.1, *buf.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{StringOutputBuffer, buf_write};

    #[test]
    fn test_escape_ninja_basic() {
        let mut raw_buf = bridge::NewStringOutputBuffer();
        let mut buffer = StringOutputBuffer::new(raw_buf.pin_mut());
        buf_write!(
            buffer,
            Ninja::from(File::new("foo/bar:my_phony")),
            ": ",
            Ninja::from(File::new("real")),
            '\n'
        );
        assert_eq!(buffer.to_string(), "foo/bar$:my_phony: real\n");
    }

    #[test]
    fn test_escape_ninja_command_posix() {
        let mut raw_buf = bridge::NewStringOutputBuffer();
        let mut buffer = StringOutputBuffer::new(raw_buf.pin_mut());
        buf_write!(
            buffer,
            Command(File::new("my tool").as_str(), EscapingPlatform::Posix,),
            " --input=",
            Command(File::new("foo bar.cc").as_str(), EscapingPlatform::Posix,),
            '\n'
        );
        assert_eq!(buffer.to_string(), "my\\$ tool --input=foo\\$ bar.cc\n");
    }

    #[test]
    fn test_escape_ninja_command_windows() {
        let mut raw_buf = bridge::NewStringOutputBuffer();
        let mut buffer = StringOutputBuffer::new(raw_buf.pin_mut());
        buf_write!(
            buffer,
            Command(File::new("my tool").as_str(), EscapingPlatform::Win,),
            " --input=",
            Command(File::new("foo bar.cc").as_str(), EscapingPlatform::Win,),
            '\n'
        );
        assert_eq!(buffer.to_string(), "\"my$ tool\" --input=\"foo$ bar.cc\"\n");
    }
}
