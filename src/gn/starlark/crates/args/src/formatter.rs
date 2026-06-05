// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use allocative::Allocative;
use starlark::{
    typing::Ty,
    values::{type_repr::StarlarkTypeRepr, Freeze, FreezeResult, Freezer},
};

use crate::errors::Error;

/// Helper to format argument values using a template containing `%s`.
#[derive(Debug, Clone, Allocative)]
pub struct Formatter {
    before: String,
    after: String,
}

fn consume_partial(chars: &mut std::str::Chars<'_>, fmt: &str) -> starlark::Result<(String, bool)> {
    let mut s = String::new();
    while let Some(c) = chars.next() {
        if c == '%' {
            match chars.next() {
                Some('%') => s.push('%'),
                Some('s') => return Ok((s, false)),
                _ => return Err(Error::InvalidFormatString(fmt.to_owned()).into()),
            }
        } else {
            s.push(c);
        }
    }
    Ok((s, true))
}

impl Formatter {
    /// Parses a format string and returns a `Formatter` if valid (must contain
    /// exactly one `%s`). Literal percents may be escaped as `%%`.
    pub fn new(fmt: &str) -> starlark::Result<Self> {
        let mut chars = fmt.chars();
        let (before, eof) = consume_partial(&mut chars, fmt)?;
        if eof {
            return Err(Error::InvalidFormatString(fmt.to_owned()).into());
        }
        let (after, eof) = consume_partial(&mut chars, fmt)?;
        if !eof {
            return Err(Error::InvalidFormatString(fmt.to_owned()).into());
        }
        Ok(Self { before, after })
    }

    /// Formats the string by replacing `%s` with the input string.
    pub fn format(&self, s: &str) -> String {
        format!("{}{}{}", self.before, s, self.after)
    }
}

impl Freeze for Formatter {
    type Frozen = Self;

    fn freeze(self, _freezer: &Freezer) -> FreezeResult<Self::Frozen> {
        Ok(self)
    }
}

impl StarlarkTypeRepr for Formatter {
    type Canonical = String;

    fn starlark_type_repr() -> Ty {
        String::starlark_type_repr()
    }
}

#[cfg(test)]
mod tests {
    use super::Formatter;

    #[test]
    fn test_formatter_parsing_and_formatting() {
        let f = Formatter::new("rate=%s").unwrap();
        assert_eq!(f.format("10"), "rate=10");

        let f = Formatter::new("rate=%s%%").unwrap();
        assert_eq!(f.format("10"), "rate=10%");

        let f = Formatter::new("%%rate=%s").unwrap();
        assert_eq!(f.format("10"), "%rate=10");

        let f = Formatter::new("%%rate=%%s%%%s").unwrap();
        assert_eq!(f.format("val"), "%rate=%s%val");
    }

    #[test]
    fn test_formatter_parsing_failures() {
        assert!(Formatter::new("no placeholder").is_err());
        assert!(Formatter::new("rate=%").is_err());
        assert!(Formatter::new("rate=%%").is_err());
        assert!(Formatter::new("rate=%s%s").is_err());
        assert!(Formatter::new("rate=%d").is_err());
        assert!(Formatter::new("rate=%s%d").is_err());
    }
}
