// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::marker::PhantomData;
use std::ops::Deref;

use starlark::values::{FrozenHeapRef, OwnedFrozenValue, UnpackValue};

/// A type-safe wrapper that holds an unpacked Starlark value and the
/// FrozenHeapRef that keeps the underlying memory alive.
pub struct UnpackedOwnedValue<'v, T> {
    value: T,
    #[allow(dead_code)] // Keeps the heap alive.
    heap: FrozenHeapRef,
    _marker: PhantomData<&'v ()>,
}

impl<'v, T> Deref for UnpackedOwnedValue<'v, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<'v, T: UnpackValue<'v>> TryFrom<OwnedFrozenValue> for UnpackedOwnedValue<'v, T> {
    type Error = starlark::Error;

    fn try_from(val: OwnedFrozenValue) -> Result<Self, Self::Error> {
        let heap = val.owner().clone();
        // Safety: We extract the raw FrozenValue from OwnedFrozenValue. This is safe because
        // we clone the heap ref and store it in UnpackedOwnedValue, ensuring the heap outlives T.
        let value = unsafe { val.unchecked_frozen_value() }.to_value();
        Ok(UnpackedOwnedValue {
            value: T::unpack_value_err(value)?,
            heap,
            _marker: PhantomData,
        })
    }
}
