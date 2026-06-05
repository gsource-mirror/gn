// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use starlark::values::Value;
use super::attr::LabelOrFile;
use starlark::values::type_repr::StarlarkTypeRepr;

#[derive(Debug, Clone, PartialEq, Eq, allocative::Allocative)]
pub enum AllowFiles {
    None,
    All,
    Some(Vec<String>),
}

impl StarlarkTypeRepr for AllowFiles {
    type Canonical = either::Either<bool, starlark::values::list::UnpackList<String>>;

    fn starlark_type_repr() -> starlark::typing::Ty {
        Self::Canonical::starlark_type_repr()
    }
}

impl<'v> starlark::values::UnpackValue<'v> for AllowFiles {
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
    pub(crate) fn validate(&self, path: &std::path::Path) -> Result<(), crate::errors::Error> {
        match self {
            AllowFiles::None => Err(crate::errors::Error::NotALabel(path.to_string_lossy().into_owned())),
            AllowFiles::All => Ok(()),
            AllowFiles::Some(exts) => {
                let file_ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if exts.iter().any(|ext| ext == file_ext) {
                    Ok(())
                } else {
                    Err(crate::errors::Error::DisallowedExtension {
                        file: path.to_path_buf(),
                        allowed: exts.clone(),
                    })
                }
            }
        }
    }
}


impl starlark::values::Freeze for AllowFiles {
    type Frozen = AllowFiles;
    fn freeze(self, _freezer: &starlark::values::Freezer) -> Result<Self::Frozen, starlark::values::FreezeError> {
        Ok(self)
    }
}

pub(crate) fn parse_label_like(
    s: &str,
    allow_files: &AllowFiles,
    relative_to: &crate::label::Package,
    session: &crate::session::StarlarkSession,
    _target: *mut crate::ffi::Target,
) -> Result<LabelOrFile, crate::errors::Error> {
    if s.starts_with("//") || s.starts_with(':') {
        Ok(LabelOrFile::Label(crate::label::Label::parse(s, relative_to.as_ref())?))
    } else {
        // It's a file
        let p = std::path::Path::new(s);
        allow_files.validate(p)?;
        if !std::fs::exists(relative_to.abs_path(session).join(p))
            .map_err(|_| crate::errors::Error::ReadFailed(s.to_owned()))?
        {
            return Err(crate::errors::Error::FileNotFound(relative_to.clone(), s.to_owned()));
        }
        Ok(LabelOrFile::File(relative_to.rel_path(session).join(p)))
    }
}
