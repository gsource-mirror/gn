// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use allocative::Allocative;
use starlark::collections::SmallMap;
use starlark::starlark_complex_value;
use crate::ffi::ToRust;
use starlark::values::{
    Coerce, Freeze, FreezeResult, Freezer, Heap, ProvidesStaticType, StarlarkValue, Trace, Value,
    ValueLike,
};
use starlark_derive::{starlark_value, NoSerialize};
use std::fmt::{self, Display, Formatter};

#[derive(Clone, Debug, Trace, Coerce, ProvidesStaticType, NoSerialize, Allocative)]
#[repr(C)]
pub struct CtxFilesGen<V> {
    pub(crate) target: usize,
    pub(crate) resolved_files: SmallMap<String, V>,
}

starlark_complex_value!(pub CtxFiles);

impl<'v, V: ValueLike<'v>> Display for CtxFilesGen<V>
where
    Self: ProvidesStaticType<'v>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "ctx.files")
    }
}

#[starlark_value(type = "ctx_files")]
impl<'v, V: ValueLike<'v>> StarlarkValue<'v> for CtxFilesGen<V>
where
    Self: ProvidesStaticType<'v>,
{
    fn get_attr(&self, attribute: &str, heap: Heap<'v>) -> Option<Value<'v>> {
        let target = self.target as *mut crate::ffi::Target;

        if attribute == "private" || attribute == "public" {
            let sources_cxx = unsafe {
                if attribute == "private" {
                    crate::ffi::GetTargetPrivateSources(&*target)
                } else {
                    crate::ffi::GetTargetPublicSources(&*target)
                }
            };
            let mut files = Vec::new();
            for item in sources_cxx.iter() {
                let f = crate::file::File(std::path::Path::new(item.to_rust()));
                files.push(heap.alloc(f));
            }
            Some(heap.alloc(files))
        } else if let Some(resolved_val) = self.resolved_files.get(attribute) {
            Some(resolved_val.to_value())
        } else {
            None
        }
    }

    fn dir_attr(&self) -> Vec<String> {
        let mut res = vec!["private".to_string(), "public".to_string()];
        res.extend(self.resolved_files.keys().cloned());
        res
    }
}

impl<'v> Freeze for CtxFiles<'v> {
    type Frozen = FrozenCtxFiles;
    fn freeze(self, freezer: &Freezer) -> FreezeResult<Self::Frozen> {
        let resolved_files = self.resolved_files.freeze(freezer)?;
        Ok(FrozenCtxFiles {
            target: self.target,
            resolved_files,
        })
    }
}

#[derive(Clone, Debug, Trace, Coerce, ProvidesStaticType, NoSerialize, Allocative)]
#[repr(C)]
pub struct CtxFileGen<V> {
    pub(crate) resolved_files: SmallMap<String, V>,
}

starlark_complex_value!(pub CtxFile);

impl<'v, V: ValueLike<'v>> Display for CtxFileGen<V>
where
    Self: ProvidesStaticType<'v>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "ctx.file")
    }
}

#[starlark_value(type = "ctx_file")]
impl<'v, V: ValueLike<'v>> StarlarkValue<'v> for CtxFileGen<V>
where
    Self: ProvidesStaticType<'v>,
{
    fn get_attr(&self, attribute: &str, _heap: Heap<'v>) -> Option<Value<'v>> {
        if let Some(resolved_val) = self.resolved_files.get(attribute) {
            Some(resolved_val.to_value())
        } else {
            None
        }
    }

    fn dir_attr(&self) -> Vec<String> {
        self.resolved_files.keys().cloned().collect()
    }
}

impl<'v> Freeze for CtxFile<'v> {
    type Frozen = FrozenCtxFile;
    fn freeze(self, freezer: &Freezer) -> FreezeResult<Self::Frozen> {
        let resolved_files = self.resolved_files.freeze(freezer)?;
        Ok(FrozenCtxFile {
            resolved_files,
        })
    }
}
