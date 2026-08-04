// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::pin::Pin;

use crate::LabelRef;

/// An interface for mutating a target in the build graph during configuration.
pub trait TargetMut {
    /// Registers a single target dependency on this target.
    fn register_dependency(self: Pin<&mut Self>, label: LabelRef<'_>, toolchain: LabelRef<'_>);
}

impl TargetMut for () {
    fn register_dependency(self: Pin<&mut Self>, _label: LabelRef<'_>, _toolchain: LabelRef<'_>) {}
}
