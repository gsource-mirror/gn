// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::path::{Path, PathBuf};
use crate::session::StarlarkSession;

#[repr(transparent)]
#[derive(Debug, Eq, PartialEq, Hash)]
pub struct PackageRef(pub(crate) str);

impl PackageRef {
    pub fn new(s: &str) -> &Self {
        unsafe { &*(s as *const str as *const PackageRef) }
    }

    pub(crate) fn relative_to(&self, path: &Path) -> PathBuf {
        // Packages always start with '//'
        if self.0.len() <= 2 {
            path.to_path_buf()
        } else {
            path.join(&self.0[2..])
        }
    }
    
    pub(crate) fn rel_path(&self, session: &StarlarkSession) -> PathBuf {
        self.relative_to(&session.root_source_dir_rel)
    }

    pub(crate) fn abs_path(&self, session: &StarlarkSession) -> PathBuf {
        self.relative_to(&session.root_source_dir_abs)
    }

    pub fn as_str(&self) -> &str {
        &self.0
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
        Package(self.0.to_owned())
    }
}

impl<'a> From<&'a crate::ffi::SourceDir> for &'a PackageRef {
    fn from(source_dir: &'a crate::ffi::SourceDir) -> Self {
        unsafe {
            let s = crate::util::from_utf8_unchecked(source_dir.value());
            PackageRef::new(if s.len() > 2 {
                // SourceDir always ends with / in C++, but packages should not.
                s.trim_end_matches('/')
            } else {
                s
            })
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, allocative::Allocative)]
pub struct Package(pub(crate) String);

impl std::ops::Deref for Package {
    type Target = PackageRef;
    fn deref(&self) -> &Self::Target {
        PackageRef::new(&self.0)
    }
}

impl AsRef<PackageRef> for Package {
    fn as_ref(&self) -> &PackageRef {
        self
    }
}

impl std::borrow::Borrow<PackageRef> for Package {
    fn borrow(&self) -> &PackageRef {
        self
    }
}

impl std::fmt::Display for Package {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
