// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use crate::Package;

/// &PackageRef is to Package as &str is to String.
#[repr(transparent)]
#[derive(Debug, Eq, PartialEq, Hash)]
pub struct PackageRef(str);

impl PackageRef {
    /// Creates a new `PackageRef` from a string slice.
    pub fn new(s: &str) -> &Self {
        // Safety: PackageRef is #[repr(transparent)] wrapping str, so their memory layouts are identical.
        unsafe { &*(s as *const str as *const PackageRef) }
    }

    /// Returns the full package name, eg. "//foo/bar"
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the package name with the leading "//" stripped (eg. "foo/bar")
    pub fn as_str_without_slashes(&self) -> &str {
        &self.0[2..]
    }
}

impl std::fmt::Display for PackageRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0)
    }
}

impl ToOwned for PackageRef {
    type Owned = Package;
    fn to_owned(&self) -> Self::Owned {
        Package::from(self.0.to_owned())
    }
}

impl<'a> From<&'a str> for &'a PackageRef {
    fn from(s: &'a str) -> Self {
        PackageRef::new(s)
    }
}
