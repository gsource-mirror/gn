// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::ops::{Deref, DerefMut};

use allocative::Allocative;
use starlark::{
    coerce::Coerce,
    values::{Freeze, FreezeResult, Freezer, Trace},
};

/// Wrapper around Option to allow implementing Coerce.
#[derive(Clone, Debug, Trace)]
#[repr(transparent)]
pub struct CoercableOption<T>(Option<T>);

impl<T> From<Option<T>> for CoercableOption<T> {
    fn from(val: Option<T>) -> Self {
        Self(val)
    }
}

// Safety: Option<From> and Option<To> have the same layout if From and To have
// the same layout and Coerce.
unsafe impl<From: Coerce<To>, To> Coerce<CoercableOption<To>> for CoercableOption<From> {}
unsafe impl<From: Coerce<To>, To> Coerce<Option<To>> for CoercableOption<From> {}

impl<T: Freeze> Freeze for CoercableOption<T> {
    type Frozen = CoercableOption<T::Frozen>;

    fn freeze(self, freezer: &Freezer) -> FreezeResult<Self::Frozen> {
        Ok(CoercableOption(self.0.freeze(freezer)?))
    }
}

impl<T: Allocative> Allocative for CoercableOption<T> {
    fn visit<'a, 'b: 'a>(&self, visitor: &'a mut allocative::Visitor<'b>) {
        self.0.visit(visitor);
    }
}

impl<T> Deref for CoercableOption<T> {
    type Target = Option<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for CoercableOption<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
