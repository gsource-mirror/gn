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

impl Formatter {
    /// Parses a format string and returns a `Formatter` if valid (must contain
    /// exactly one `%s`).
    pub fn new(fmt: &str) -> starlark::Result<Self> {
        let mut split = fmt.split("%s");
        match (split.next(), split.next(), split.next()) {
            (Some(before), Some(after), None) => Ok(Self {
                before: before.to_owned(),
                after: after.to_owned(),
            }),
            _ => Err(Error::InvalidFormatString(fmt.to_owned()).into()),
        }
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
