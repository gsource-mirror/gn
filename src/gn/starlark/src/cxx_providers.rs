// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use crate::ffi::{self};
use starlark::values::{
    Freeze, FreezeResult, Freezer, Heap, ProvidesStaticType, StarlarkValue, Value, Trace, Tracer,
};
use starlark_derive::{starlark_value, NoSerialize};

#[derive(Clone, Debug, ProvidesStaticType, NoSerialize, allocative::Allocative)]
pub struct CxxDefaultInfo {
    #[allocative(skip)]
    pub(crate) target: *mut ffi::Target,
}

impl CxxDefaultInfo {
    pub fn new(target: *mut ffi::Target) -> Self {
        Self { target }
    }
}

starlark::starlark_simple_value!(CxxDefaultInfo);

unsafe impl Send for CxxDefaultInfo {}
unsafe impl Sync for CxxDefaultInfo {}

impl std::fmt::Display for CxxDefaultInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DefaultInfo(target={:?})", self.target)
    }
}

unsafe impl<'v> Trace<'v> for CxxDefaultInfo {
    fn trace(&mut self, _tracer: &Tracer<'v>) {}
}

impl Freeze for CxxDefaultInfo {
    type Frozen = CxxDefaultInfo;
    fn freeze(self, _freezer: &Freezer) -> FreezeResult<Self::Frozen> {
        Ok(self)
    }
}

#[starlark_value(type = "DefaultInfo")]
impl<'v> StarlarkValue<'v> for CxxDefaultInfo {
    fn get_attr(&self, attribute: &str, heap: Heap<'v>) -> Option<Value<'v>> {
        match attribute {
            // TODO: should return true or false once cxx targets expose whether they're executable
            "executable" => Some(Value::new_none()),
            "files" => {
                let outputs_cxx = unsafe { ffi::GetTargetOutputFiles(&*self.target) };
                let mut files = Vec::new();
                use crate::ffi::ToRust;
                for out in outputs_cxx.iter() {
                    let file = out.to_rust();
                    let val = heap.alloc(file);
                    files.push(val);
                }

                let depset = crate::depset::Depset::new(
                    crate::depset::Order::Unspecified,
                    files,
                    Vec::new(),
                );
                Some(heap.alloc(depset))
            }
            _ => None,
        }
    }

    fn dir_attr(&self) -> Vec<String> {
        vec!["executable".to_owned(), "files".to_owned()]
    }
}