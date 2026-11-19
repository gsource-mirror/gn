// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use allocative::Allocative;
use strum::{Display, EnumIter, IntoStaticStr};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumIter, Display, IntoStaticStr, Allocative)]
#[strum(serialize_all = "snake_case")]
/// Copied from target.h's OutputType enum.
/// Extended with all other types of Item, because as far as rust is concerned,
/// these other item types are just targets.
pub enum OutputType {
    // Unknown = 0
    Group = 1,
    Executable,
    SharedLibrary,
    LoadableModule,
    StaticLibrary,
    SourceSet,
    Copy,
    Action,
    ActionForeach,
    BundleData,
    CreateBundle,
    GeneratedFile,
    RustLibrary,
    RustProcMacro,
    NoOp,

    // As far as starlark is concerned, these are regular targets that just
    // happen to return providers and no files.
    Config,
    Toolchain,
    Pool,
}

/// Compile-time string equality test
const fn str_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() && a[i] == b[i] {
        i += 1;
    }
    i == a.len()
}

/// Compile-time check if slice contains a string.
const fn contains(slice: &[&'static str], needle: &'static str) -> bool {
    let mut i = 0;
    while i < slice.len() {
        if str_eq(slice[i], needle) {
            return true;
        }
        i += 1;
    }
    false
}

impl OutputType {
    const MAX: u8 = Self::Pool as u8;
    const NOOP: u8 = Self::NoOp as u8;
    const TARGET_MAX: u8 = Self::NOOP - 1;

    /// Returns true if this type represents a GN target (rather than a `Config`
    /// or `Pool`).
    pub const fn is_target(self) -> bool {
        (self as u8) <= Self::TARGET_MAX
    }

    /// Converts a u8 discriminant from GN C++ `Item` (`Target::OutputType` or
    /// non-target item tag) to `Option<OutputType>`.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 | Self::NOOP => None,
            1..=Self::MAX => {
                // Safety: OutputType is #[repr(u8)] and contiguous for 1..=MAX
                Some(unsafe { std::mem::transmute::<u8, Self>(value) })
            },
            _ => panic!("Invalid output type: {value}"),
        }
    }

    /// Returns (attr-and-files attributes, attr-only attributes).
    ///
    /// Any attributes that should fill ctx.files.* should go in the former.
    /// Other attributes should go in the latter.
    pub const fn attrs(self) -> (&'static [&'static str], &'static [&'static str]) {
        match self {
            Self::Executable
            | Self::SharedLibrary
            | Self::LoadableModule
            | Self::StaticLibrary
            | Self::SourceSet => (&["sources", "public"], &["deps", "public_deps"]),
            _ => (&[], &[]),
        }
    }

    pub const fn has_sources(self) -> bool {
        contains(self.attrs().0, "sources")
    }

    pub const fn has_public(self) -> bool {
        contains(self.attrs().0, "public")
    }

    pub const fn has_deps(self) -> bool {
        contains(self.attrs().1, "deps")
    }

    pub const fn has_public_deps(self) -> bool {
        contains(self.attrs().1, "public_deps")
    }
}

impl From<OutputType> for usize {
    fn from(value: OutputType) -> Self {
        value as Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_capabilities() {
        assert!(OutputType::Executable.has_deps());
        assert!(!OutputType::Copy.has_deps());
    }

    #[test]
    fn test_non_target_item_types() {
        assert_eq!(OutputType::from_u8(0), None);
        assert_eq!(OutputType::from_u8(15), None);
        assert_eq!(OutputType::from_u8(16), Some(OutputType::Config));
        assert_eq!(OutputType::from_u8(18), Some(OutputType::Pool));
        assert!(OutputType::SourceSet.is_target());
        assert!(!OutputType::Config.is_target());
    }
}
