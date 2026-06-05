// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use crate::{declare_opaque_type, Settings};

declare_opaque_type!(pub Scope);

impl Scope {
    pub fn settings(&self) -> &Settings {
        extern "C" {
            fn GetSettingsFromScope(scope: &Scope) -> &Settings;
        }
        // Safety: FFI function to retrieve settings reference from Scope
        unsafe { GetSettingsFromScope(self) }
    }
}
