// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use allocative::Allocative;
use starlark::values::{AllocValue, Heap, ProvidesStaticType, StarlarkValue, Value, ValueLike};
use starlark_derive::{starlark_value, NoSerialize};
use types::{LabelRef, TargetRef as _};

use crate::target::Target;

#[derive(Clone, Allocative, ProvidesStaticType, NoSerialize)]
pub struct TargetRef(pub(crate) &'static Target);

impl PartialEq for TargetRef {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0.ffi, other.0.ffi)
    }
}
impl Eq for TargetRef {}

impl std::hash::Hash for TargetRef {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(self.0.ffi, state);
    }
}

impl std::fmt::Debug for TargetRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Delegate to Display
        write!(f, "{self}")
    }
}

impl std::fmt::Display for TargetRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

impl types::IPromiseToImplementStarlarkEqAndHash for TargetRef {}

#[starlark_value(type = "Target")]
impl<'v> StarlarkValue<'v> for TargetRef {
    fn equals(&self, other: Value<'v>) -> starlark::Result<bool> {
        Ok(other
            .downcast_ref::<TargetRef>()
            .is_some_and(|other| other == self))
    }

    fn write_hash(
        &self,
        hasher: &mut starlark::collections::StarlarkHasher,
    ) -> starlark::Result<()> {
        use std::hash::Hash as _;
        self.hash(hasher);
        Ok(())
    }
}

impl<'v> AllocValue<'v> for TargetRef {
    fn alloc_value(self, heap: Heap<'v>) -> Value<'v> {
        heap.alloc_simple(self)
    }
}

impl types::TargetRef for TargetRef {
    type Cxx = crate::bridge::Target;

    fn label(&self) -> LabelRef<'_> {
        self.0.ffi.label().as_ref()
    }

    fn toolchain(&self) -> LabelRef<'_> {
        let settings = unsafe { &*self.0.ffi.settings() };
        settings.toolchain_label().as_ref()
    }

    fn outputs(&self) -> Vec<types::File> {
        todo!()
    }

    fn target_out_dir(&self, _prefix: &str, _suffix: &str, _separator: &str) -> String {
        todo!()
    }
}

impl types::TargetMut for crate::bridge::Target {
    fn register_dependency(
        self: std::pin::Pin<&mut Self>,
        label: LabelRef<'_>,
        toolchain: LabelRef<'_>,
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
