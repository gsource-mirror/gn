// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use crate::ffi::bindings as ffi;
use crate::label::Package;
use crate::ffi::ToRust;

impl ToRust<Package> for ffi::SourceDir {
    fn to_rust(&self) -> Package {
        Package(self.value().to_rust())
    }
}
