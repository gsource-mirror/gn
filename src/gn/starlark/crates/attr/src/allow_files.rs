// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use starlark::typing::Ty;
use starlark::values::list::UnpackList;
use starlark::values::type_repr::StarlarkTypeRepr;
use starlark::values::Freeze;
use starlark::values::FreezeError;
use starlark::values::Freezer;
use starlark::values::UnpackValue;
use starlark::values::Value;
use types::{Label, PackageRef, PathResolver, Error as TypeError};

use crate::attr::LabelOrFile;

/// The rust type for the starlark value passed to attr.label(allow_files = ...)
#[derive(Debug, Clone, PartialEq, Eq, allocative::Allocative)]
pub enum AllowFiles {
    None,
    All,
    Some(Vec<String>),
}

impl StarlarkTypeRepr for AllowFiles {
    type Canonical = either::Either<bool, UnpackList<String>>;

    fn starlark_type_repr() -> Ty {
        Self::Canonical::starlark_type_repr()
    }
}

impl<'v> UnpackValue<'v> for AllowFiles {
    type Error = starlark::Error;

    fn unpack_value_impl(value: Value<'v>) -> Result<Option<Self>, Self::Error> {
        let canonical = Self::Canonical::unpack_value(value)?;
        Ok(canonical.map(|c| match c {
            either::Either::Left(false) => AllowFiles::None,
            either::Either::Left(true) => AllowFiles::All,
            either::Either::Right(list) => AllowFiles::Some(list.items),
        }))
    }
}

impl AllowFiles {
    pub(crate) fn validate(&self, path: &str) -> Result<(), crate::Error> {
        match self {
            AllowFiles::None => Err(TypeError::NotALabel(path.to_owned()).into()),
            AllowFiles::All => Ok(()),
            AllowFiles::Some(exts) => {
                let p = std::path::Path::new(path);
                let file_ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
                if exts.iter().any(|ext| ext == file_ext) {
                    Ok(())
                } else {
                    Err(crate::Error::DisallowedExtension {
                        file: p.to_path_buf(),
                        allowed: exts.clone(),
                    })
                }
            }
        }
    }
}

impl Freeze for AllowFiles {
    type Frozen = AllowFiles;
    fn freeze(self, _freezer: &Freezer) -> Result<Self::Frozen, FreezeError> {
        Ok(self)
    }
}

pub(crate) fn parse_label_like(
    s: &str,
    allow_files: &AllowFiles,
    relative_to: &PackageRef,
    path_resolver: &PathResolver,
) -> Result<LabelOrFile, crate::Error> {
    if s.starts_with("//") || s.starts_with(':') {
        // It's a label.
        Ok(LabelOrFile::Label(Label::parse(s, relative_to)?))
    } else {
        // It's a file.
        allow_files.validate(s)?;
        Ok(LabelOrFile::File(
            path_resolver.source_file(relative_to, s)?,
        ))
    }
}
