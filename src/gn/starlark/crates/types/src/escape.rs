// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use crate::platform::Platform;

/// Sink for writing escaped characters and strings directly to a stream.
pub trait Ostream {
    /// Writes a single character to the stream.
    fn push(&mut self, c: char);
    /// Writes a string slice to the stream.
    fn push_str(&mut self, s: &str);
}

impl Ostream for String {
    fn push(&mut self, c: char) {
        self.push(c);
    }

    fn push_str(&mut self, s: &str) {
        self.push_str(s);
    }
}

/// Escapes a composite token made of string slices into the stream for a specific platform.
pub fn escape_token<'a, P: Platform>(
    parts: impl IntoIterator<Item = &'a str, IntoIter: Clone>,
    os: &mut impl Ostream,
    needs_space: &mut bool,
) {
    if *needs_space {
        os.push(' ');
    } else {
        *needs_space = true;
    }

    let iter = parts.into_iter();
    if iter.clone().all(|s| s.is_empty()) {
        os.push_str("\"\"");
        return;
    }

    let mut escape_quoting = None;
    for c in iter.clone().flat_map(|s| s.chars()) {
        match c {
            ' ' | '"' | '\'' => {
                escape_quoting = Some(true);
                break;
            },
            '$' => escape_quoting = Some(false),
            _ => {},
        }
    }

    match escape_quoting {
        None => {
            for s in iter {
                os.push_str(s);
            }
        },
        Some(false) => {
            for s in iter {
                for c in s.chars() {
                    if c == '$' {
                        os.push('$');
                    }
                    os.push(c);
                }
            }
        },
        Some(true) => {
            os.push('"');
            P::escape_quoted(iter, os);
            os.push('"');
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{Posix, Windows};

    fn escape<P: Platform>(parts: &[&str]) -> String {
        let mut needs_space = false;
        let mut out = String::new();
        escape_token::<P>(parts.iter().copied(), &mut out, &mut needs_space);
        out
    }

    #[test]
    fn test_escape_composite_slices() {
        assert_eq!(
            escape::<Windows>(&["--flag=", "hello world", ""]),
            r#""--flag=hello world""#
        );
        assert_eq!(
            escape::<Windows>(&["--flag=", "foo/bar.cc", ""]),
            r#"--flag=foo/bar.cc"#
        );
        assert_eq!(escape::<Windows>(&["a", "", "b"]), "ab");
    }

    #[test]
    fn test_escape_empty_tokens() {
        assert_eq!(escape::<Windows>(&[]), r#""""#);
        assert_eq!(escape::<Windows>(&[""]), r#""""#);
    }

    #[test]
    fn test_escape_windows_paths_and_quotes() {
        assert_eq!(
            escape::<Windows>(&[r#"C:\Program Files\App"#]),
            r#""C:\Program Files\App""#
        );
        assert_eq!(
            escape::<Windows>(&[r#"C:\Program Files\"#]),
            r#""C:\Program Files\\""#
        );
        assert_eq!(
            escape::<Windows>(&[r#"-DMSG="hello world""#]),
            r#""-DMSG=\"hello world\"""#
        );
    }

    #[test]
    fn test_escape_posix_shell_and_dollars() {
        assert_eq!(
            escape::<Posix>(&["-DMSG='hello $USER'"]),
            r#""-DMSG='hello \$$USER'""#
        );
        assert_eq!(
            escape::<Posix>(&[r#"-DMSG="hello world""#]),
            r#""-DMSG=\"hello world\"""#
        );
    }
}
