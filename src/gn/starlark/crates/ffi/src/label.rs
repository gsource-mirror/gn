// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use types::{LabelRef, PackageRef};

use crate::bridge::{Label, SourceDir};
use crate::types::CxxStringExt;

impl Label {
    /// Returns the directory part of the label (the package path).
    pub fn package(&self) -> &PackageRef {
        self.dir().as_rust()
    }

    /// Returns the name part of the label.
    pub fn name(&self) -> &str {
        self.name_cxx().as_str()
    }

    /// Returns a `LabelRef` referencing the directory and name of this label.
    pub fn as_ref(&self) -> LabelRef<'_> {
        LabelRef::new(self.package(), self.name())
    }
}

impl SourceDir {
    pub fn as_rust(&self) -> &types::PackageRef {
        let s = self.SourceWithNoTrailingSlash().as_str();
        debug_assert!(s.starts_with("//"));
        // Safety: All SourceDir objects passed to rust should start with //
        unsafe { types::PackageRef::new_unchecked(s) }
    }
}

#[cfg(test)]
mod tests {
    use types::{LabelRef, PackageRef};

    use crate::TestWithScope;

    #[test]
    fn test_label() {
        let mut setup = TestWithScope::new();
        assert_eq!(
            setup.scope().settings().toolchain(),
            LabelRef::new(PackageRef::new_for_testing("//toolchain"), "default")
        )
    }
}
