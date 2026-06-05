// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::pin::Pin;

use starlark::values::FrozenValue;

use crate::{bridge::Value, Immutable, OwnedSlice, Scope};

impl Scope {
    pub(crate) fn new<'b>(
        parent: &Self,
        keys: &[&str],
    ) -> (cxx::UniquePtr<Self>, OwnedSlice<Pin<&'b mut Value>>) {
        let mut nested_scope = cxx::UniquePtr::<Self>::null();
        let values = crate::bridge::NewScope(parent, keys, &mut nested_scope);
        (nested_scope, values.into())
    }

    /// Returns the settings for the given scope.
    pub fn settings(&self) -> &crate::Settings {
        // Safety: Settings pointer is always valid and non-null.
        unsafe { self.settings_cxx().as_ref() }.unwrap()
    }

    /// Returns the items currently in the scope (not including parent scopes).
    pub fn items(&self) -> Immutable<OwnedSlice<crate::bridge::KeyValue<'_>>> {
        let slice = crate::bridge::GetScopeItems(self);
        Immutable::from(crate::OwnedSlice::<crate::bridge::KeyValue>::from(slice))
    }

    /// Converts Scope items to Starlark key-value pairs.
    pub fn get_kv<'a>(
        &'a self,
        frozen_heap: &starlark::values::FrozenHeap,
    ) -> Vec<(&'a str, FrozenValue)> {
        let owned = self.items();
        let mut items = Vec::new();
        // Iterate over the KeyValue contiguous slice:
        for pair in owned.as_slice() {
            items.push((pair.key, pair.value.to_rust(frozen_heap)));
        }
        items
    }
}
