// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use types::{LabelRef, PackageRef};

use crate::{Label, SourceDir};

impl Label {
    /// Returns the directory part of the label (the package path).
    pub fn package(&self) -> &PackageRef {
        self.dir().as_rust()
    }

    /// Returns a `LabelRef` referencing the directory and name of this label.
    pub fn as_ref(&self) -> LabelRef<'_> {
        LabelRef::new(self.package(), self.name())
    }
}

impl SourceDir {
    pub fn as_rust(&self) -> &types::PackageRef {
        let s = self.SourceWithNoTrailingSlash();
        // While source dirs aren't guaranteed to start with // (they may be
        // absolute), we only convert source dirs to rust for either labels or
        // BUILD.gn directories, both of which are guaranteed to be
        // source-relative.
        debug_assert!(s.starts_with("//"));
        // Safety: Guaranteed to start with "//"
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
        );
    }
}
