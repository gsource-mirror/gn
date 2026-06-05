// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use crate::bridge::Scope;
use crate::types::PointerExt;

impl Scope {
    /// Returns the settings for the given scope.
    pub fn settings(&self) -> &crate::Settings {
        self.settings_cxx().non_null()
    }
}
