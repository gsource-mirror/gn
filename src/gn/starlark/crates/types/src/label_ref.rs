// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use crate::Label;
use crate::PackageRef;

/// A borrowed reference to a `Label`.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct LabelRef<'a> {
    /// The package part of the label reference.
    pub package: &'a PackageRef,
    /// The name part of the label reference.
    pub name: &'a str,
}

impl<'a> std::fmt::Display for LabelRef<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.package, self.name)
    }
}

impl<'a> std::fmt::Debug for LabelRef<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Label(\"{}:{}\")", self.package, self.name)
    }
}

impl<'a> LabelRef<'a> {
    /// Creates a new `LabelRef`.
    pub fn new(package: &'a PackageRef, name: &'a str) -> Self {
        Self { package, name }
    }

    /// Converts this reference into an owned `Label`.
    pub fn to_owned(&self) -> Label {
        Label {
            package: self.package.to_owned(),
            name: self.name.to_owned(),
        }
    }
}
