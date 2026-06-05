// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use types::{LabelRef, PackageRef};

use crate::declare_opaque_type;
declare_opaque_type!(pub Label);

impl Label {
    /// Returns the directory part of the label (the package path).
    pub fn package(&self) -> &PackageRef {
        extern "C" {
            fn GetLabelDir(label: &Label) -> &SourceDir;
        }
        // Safety: Just an FFI function.
        unsafe { GetLabelDir(self) }.as_rust()
    }

    /// Returns the name part of the label.
    pub fn name(&self) -> &str {
        extern "C" {
            fn GetLabelName(label: &Label) -> &str;
        }
        // Safety: Just an FFI function.
        unsafe { GetLabelName(self) }
    }

    /// Returns a `LabelRef` referencing the directory and name of this label.
    pub fn as_ref(&self) -> LabelRef<'_> {
        LabelRef::new(self.package(), self.name())
    }
}

declare_opaque_type!(pub SourceDir);

impl SourceDir {
    pub fn as_rust(&self) -> &types::PackageRef {
        extern "C" {
            fn GetSourceDirValue(dir: &SourceDir) -> &str;
        }
        // Safety: Just an FFI function.
        let s = unsafe { GetSourceDirValue(self) };
        // While source dirs aren't guarunteed to start with "//" (they may be
        // absolute), we only convert source dirs to rust for either labels or
        // BUILD.gn directories, both of which are guarunteed to be
        // source-relative.
        debug_assert!(s.starts_with("//"));
        // Safety: Guarunteed to be a valid package.
        unsafe { types::PackageRef::new_unchecked(s) }
    }
}
