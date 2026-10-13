// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::borrow::Cow;

/// Escapes a command-line token for Ninja.
pub fn escape_ninja_token<'a>(token: &'a str) -> Cow<'a, str> {
    match escape_mode(token) {
        (_, 0) => Cow::Borrowed(token),
        (false, n) => {
            let mut result = String::with_capacity(token.len() + n);
            for c in token.chars() {
                if c == '$' {
                    result.push('$');
                }
                result.push(c);
            }
            Cow::Owned(result)
        },
        (true, n) => {
            let mut result = String::with_capacity(token.len() + n);
            result.push('"');
            for c in token.chars() {
                match c {
                    '$' => result.push('$'),
                    '"' | '\\' => result.push('\\'),
                    _ => {},
                }
                result.push(c);
            }
            result.push('"');
            Cow::Owned(result)
        },
    }
}

// Escapes a command-line token for Ninja.
pub fn escape_ninja_token_owned(token: String) -> String {
    match escape_ninja_token(&token) {
        Cow::Borrowed(_) => token,
        Cow::Owned(s) => s,
    }
}

// Returns (quoting, minimum number of escape characters required)
fn escape_mode(token: &str) -> (bool, usize) {
    let mut n_escapes = 0;
    for &b in token.as_bytes() {
        match b {
            // Assume there must be a closing character escaped as well.
            b' ' | b'"' | b'\'' => return (true, n_escapes + 2),
            b'$' => n_escapes += 1,
            _ => {},
        }
    }
    if token.is_empty() {
        (true, 2)
    } else {
        (false, n_escapes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_ninja_token() {
        assert!(matches!(
            escape_ninja_token("-DFOO=bar"),
            Cow::Borrowed("-DFOO=bar")
        ));
        assert_eq!(escape_ninja_token("-DFOO=$BAR"), "-DFOO=$$BAR");
        assert_eq!(
            escape_ninja_token("-I/path with space/"),
            r#""-I/path with space/""#
        );
        assert_eq!(
            escape_ninja_token(r#"-DMSG="hello world""#),
            r#""-DMSG=\"hello world\"""#
        );
        assert_eq!(
            escape_ninja_token("-DMSG='hello $USER'"),
            r#""-DMSG='hello $$USER'""#
        );
    }
}
