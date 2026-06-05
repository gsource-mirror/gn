// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::path::PathBuf;

use starlark::values::UnpackValueError;

/// Errors returned by target attribute validation and coercion.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Attribute `{param}` is mandatory")]
    MandatoryAttribute { param: String },
    #[error("Want non-empty list, got []")]
    EmptyListDisallowed,
    #[error("Want non-empty dict, got {{}}")]
    EmptyDictDisallowed,
    #[error("File \"{file:?}\" has disallowed extension, allowed extensions are: {allowed:?}")]
    DisallowedExtension { file: PathBuf, allowed: Vec<String> },
    #[error("Config transition not implemented: {0}")]
    ConfigTransitionNotImplemented(String),
    #[error("Failed to read {0}")]
    ReadFailed(String),
    #[error("mandatory and default are mutually exclusive")]
    MandatoryAndDefaultMutuallyExclusive,
    #[error("'{0}' must produce a single file")]
    MustProduceSingleFile(String),

    #[error("Value {0} is not in allowed set")]
    IntNotAllowed(i32),
    #[error("Value \"{0}\" is not in allowed set")]
    StringNotAllowed(String),

    #[error("allow_files and allow_single_file are mutually exclusive")]
    AllowFilesMutuallyExclusive,
    #[error("allow_empty = False requires the attribute to be mandatory or have a non-empty default value")]
    AllowEmptyRequiresMandatoryOrDefault,

    #[error(transparent)]
    Types(#[from] types::Error),
}

impl From<Error> for starlark::Error {
    fn from(err: Error) -> Self {
        starlark::Error::new_other(err)
    }
}

impl UnpackValueError for Error {
    fn into_error(this: Self) -> starlark::Error {
        starlark::Error::new_other(this)
    }
}
