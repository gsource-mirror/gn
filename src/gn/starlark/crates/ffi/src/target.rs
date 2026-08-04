// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::ptr::NonNull;

use types::util::deref_ptr;

#[derive(Copy, Clone)]
pub struct Target {
    cxx: NonNull<crate::bridge::CxxTarget>,
}

impl std::ops::Deref for Target {
    type Target = crate::bridge::CxxTarget;

    fn deref(&self) -> &Self::Target {
        deref_ptr(self.cxx)
    }
}

impl allocative::Allocative for Target {
    fn visit<'a, 'b: 'a>(&self, visitor: &'a mut allocative::Visitor<'b>) {
        let visitor = visitor.enter_self_sized::<Self>();
        visitor.exit();
    }
}

// Safety: Target pointers in GN are heap-allocated and thread-safe to transfer
// across evaluation boundaries.
unsafe impl Send for Target {}
// Safety: Target pointers in GN are thread-safe to reference across evaluation
// boundaries.
unsafe impl Sync for Target {}

impl crate::bridge::CxxTarget {
    /// Returns the settings for the target.
    pub fn settings(&self) -> &crate::Settings {
        // Safety: Settings pointer is always valid and non-null on constructed Targets.
        unsafe { self.settings_cxx().as_ref() }.unwrap()
    }

    /// Returns the toolchain label for the target.
    pub fn toolchain(&self) -> types::LabelRef<'_> {
        self.settings().toolchain_label().as_ref()
    }

    /// Registers a dependency on this target.
    pub fn register_dependency(
        self: std::pin::Pin<&mut Self>,
        label: types::LabelRef<'_>,
        toolchain: types::LabelRef<'_>,
    ) {
        crate::bridge::register_dependency(
            self,
            label.package().as_str(),
            label.name(),
            toolchain.package().as_str(),
            toolchain.name(),
        );
    }
}
