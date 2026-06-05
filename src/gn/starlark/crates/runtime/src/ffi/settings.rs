// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use crate::ffi::label::Label;

declare_opaque_type!(pub(crate) Settings);

impl Settings {
    pub fn toolchain_label<'a>(&'a self) -> types::LabelRef<'a> {
        extern "C" {
            fn GetToolchainLabelFromSettings(settings: &Settings) -> &Label;
        }
        unsafe { GetToolchainLabelFromSettings(self).as_label_ref() }
    }
}
