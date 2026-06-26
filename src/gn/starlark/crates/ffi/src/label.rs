// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use crate::declare_opaque_type;

declare_opaque_type!(pub Label);

impl Label {
    pub fn dir(&self) -> &str {
        extern "C" {
            fn GetLabelDir(label: &Label) -> &str;
        }
        unsafe { GetLabelDir(self) }
    }

    pub fn name(&self) -> &str {
        extern "C" {
            fn GetLabelName(label: &Label) -> &str;
        }
        unsafe { GetLabelName(self) }
    }

    pub fn as_label_ref<'a>(&'a self) -> types::LabelRef<'a> {
        types::LabelRef {
            package: types::PackageRef::new(self.dir()),
            name: self.name(),
        }
    }
}
