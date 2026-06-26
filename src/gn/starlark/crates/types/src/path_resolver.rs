// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::path::PathBuf;

use crate::{File, PackageRef};

/// Resolves paths to Files relative to the build directory.
#[derive(Clone, Debug)]
pub struct PathResolver {
    /// Absolute path to the root source dir on disk.
    root_source_dir_abs: PathBuf,
    /// Path to the root source dir relative to the output directory.
    /// *Must* end with a trailing slash.
    root_source_dir_rel: String,
}

impl PathResolver {
    /// Creates a new `PathResolver` with the given absolute and relative root paths.
    pub fn new(root_source_dir_abs: PathBuf, root_source_dir_rel: String) -> Self {
        Self {
            root_source_dir_abs,
            root_source_dir_rel,
        }
    }

    /// Calculates where the file should exist on disk.
    pub fn absolute_path(&self, pkg: &PackageRef, s: &str) -> PathBuf {
        self.root_source_dir_abs
            .join(pkg.as_str_without_slashes())
            .join(s)
    }

    /// Creates a `File` object for a file path relative to a package.
    /// Validates that the file exists on disk.
    pub fn source_file(&self, pkg: &PackageRef, s: &str) -> Result<File, crate::Error> {
        // Gn *does not* check that the files you refer to exist on disk.
        // This is because non-starlark needs a way to refer to generated files.
        // Starlark has no such problem, so we validate that the files exist on
        // disk for the user's convenience.
        let abs_path = self.absolute_path(pkg, s);
        if !abs_path.exists() {
            return Err(crate::Error::FileNotFound(pkg.to_owned(), s.to_owned()));
        }
        Ok(File::from_rust(if pkg.as_str().len() > 2 {
            format!(
                "{}{}/{}",
                self.root_source_dir_rel,
                pkg.as_str_without_slashes(),
                s
            )
        } else {
            format!("{}{}", self.root_source_dir_rel, s)
        }))
    }
}
