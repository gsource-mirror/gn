// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use crate::Scope;
impl Scope {
    /// Returns the settings for the given scope.
    pub fn settings(&self) -> &crate::Settings {
        // Safety: Settings pointer is always valid and non-null.
        unsafe { self.settings_cxx().as_ref() }.unwrap()
    }
}
