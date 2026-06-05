// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use starlark::environment::{Methods, MethodsBuilder, MethodsStatic};
use starlark::starlark_simple_value;
use starlark::values::{
    Freeze, FreezeResult, Freezer, ProvidesStaticType, StarlarkValue, Trace, Tracer,
};
use starlark_derive::{starlark_module, starlark_value, NoSerialize};
use std::path::Path;


/// File is an extremely lightweight type which is basically just a &Path.
/// Paths are relative to the output directory, and thus it either:
/// * Reads OutputFile::value() from rust
/// * Refers to a PathBuf stored in the Target object.
#[derive(Clone, Debug, ProvidesStaticType, NoSerialize, allocative::Allocative)]
pub struct File(#[allocative(skip)] pub(crate) &'static Path);

starlark_simple_value!(File);

impl File {
    pub fn as_path(&self) -> &Path {
        self.0
    }
}

// Safety: File wraps a static reference to Path which is thread-safe.
unsafe impl Send for File {}
unsafe impl Sync for File {}

impl PartialEq for File {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}
impl Eq for File {}

impl std::hash::Hash for File {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(self.0, state);
    }
}

impl std::fmt::Display for File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.to_string_lossy())
    }
}

unsafe impl<'v> Trace<'v> for File {
    fn trace(&mut self, _tracer: &Tracer<'v>) {}
}

impl Freeze for File {
    type Frozen = File;
    fn freeze(self, _freezer: &Freezer) -> FreezeResult<Self::Frozen> {
        Ok(self)
    }
}

#[starlark_value(type = "File")]
impl<'v> StarlarkValue<'v> for File {
    fn get_methods() -> Option<&'static Methods>
    where
        Self: Sized,
    {
        static RES: MethodsStatic = MethodsStatic::new("File", file_methods);
        Some(RES.methods())
    }

    fn write_hash(&self, hasher: &mut starlark::collections::StarlarkHasher) -> starlark::Result<()> {
        use std::hash::Hash;
        self.hash(hasher);
        Ok(())
    }
}

#[starlark_module]
fn file_methods(methods: &mut MethodsBuilder) {
    #[starlark(attribute)]
    fn basename<'v>(this: &'v File) -> starlark::Result<&'v str> {
        Ok(this.0.file_name().and_then(|s| s.to_str()).unwrap_or(""))
    }

    #[starlark(attribute)]
    fn dirname<'v>(this: &'v File) -> starlark::Result<&'v str> {
        Ok(this.0.parent().and_then(|s| s.to_str()).unwrap_or(""))
    }

    #[starlark(attribute)]
    fn extension<'v>(this: &'v File) -> starlark::Result<&'v str> {
        Ok(this.0.extension().and_then(|s| s.to_str()).unwrap_or(""))
    }

    #[starlark(attribute)]
    fn is_source(this: &File) -> starlark::Result<bool> {
        Ok(this.0.starts_with(".."))
    }

    #[starlark(attribute)]
    fn path<'v>(this: &'v File) -> starlark::Result<&'v str> {
        Ok(this.0.to_str().unwrap_or(""))
    }
}
