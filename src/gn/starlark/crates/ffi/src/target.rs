// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

pub(crate) struct Target {
    pub(crate) ffi: &'static crate::bridge::Target,
}

impl allocative::Allocative for Target {
    fn visit<'a, 'b: 'a>(&self, visitor: &'a mut allocative::Visitor<'b>) {
        let visitor = visitor.enter_self_sized::<Self>();
        visitor.exit();
    }
}

unsafe impl Send for Target {}
unsafe impl Sync for Target {}
