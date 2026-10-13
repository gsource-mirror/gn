// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use crate::escape::Ostream;

/// Target platform for platform-specific behavior.
pub trait Platform {
    /// Escapes a token inside quotes according to platform-specific rules.
    fn escape_quoted<'a>(iter: impl Iterator<Item = &'a str>, os: &mut impl Ostream);
}

pub struct Windows;

impl Platform for Windows {
    fn escape_quoted<'a>(iter: impl Iterator<Item = &'a str>, os: &mut impl Ostream) {
        let mut pending_backslashes = 0;
        for c in iter.flat_map(|s| s.chars()) {
            match c {
                '\\' => {
                    pending_backslashes += 1;
                },
                '"' => {
                    for _ in 0..pending_backslashes + 1 {
                        os.push('\\');
                    }
                    pending_backslashes = 0;
                },
                '$' => {
                    pending_backslashes = 0;
                    os.push('$');
                },
                _ => {
                    pending_backslashes = 0;
                },
            }
            os.push(c);
        }
        for _ in 0..pending_backslashes {
            os.push('\\');
        }
    }
}

pub struct Posix;

impl Platform for Posix {
    fn escape_quoted<'a>(iter: impl Iterator<Item = &'a str>, os: &mut impl Ostream) {
        for c in iter.flat_map(|s| s.chars()) {
            match c {
                '"' | '\\' | '`' => {
                    os.push('\\');
                    os.push(c);
                },
                '$' => {
                    // $$ to escape ninja, backslash to escape the shell.
                    os.push_str("\\$$");
                },
                _ => os.push(c),
            }
        }
    }
}

/// The platform that would correspond to the GN (host_os, host_cpu).
/// ie. The platform GN is being built for.
#[cfg(windows)]
pub type HostPlatform = Windows;
#[cfg(not(windows))]
pub type HostPlatform = Posix;
