// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

/// A container for a value owned by C++.
#[repr(transparent)]
pub struct CxxOwned<T: CxxDrop>(pub(crate) T);

impl<T: CxxDrop> Drop for CxxOwned<T> {
    fn drop(&mut self) {
        self.0.drop();
    }
}

impl<T: CxxDrop> std::ops::Deref for CxxOwned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Implement this trait to indicate how to drop a value owned by C++.
pub trait CxxDrop {
    fn drop(&mut self);
}
