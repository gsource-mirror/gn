// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

declare_opaque_type!(pub(in crate::ffi) SourceDir);

impl SourceDir {
    pub fn as_rust(&self) -> &types::PackageRef {
        extern "C" {
            fn GetSourceDirValue(dir: &SourceDir) -> &str;
        }
        let s = unsafe { GetSourceDirValue(self) };
        // strip the trailing "/" that source dirs always have
        let pkg_path = if s.ends_with('/') && s.len() > 2 {
            &s[..s.len() - 1]
        } else {
            s
        };
        types::PackageRef::new(pkg_path)
    }
}
