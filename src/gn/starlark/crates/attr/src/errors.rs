// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::path::PathBuf;

use starlark::values::UnpackValueError;

/// Errors returned by target attribute validation and coercion.
#[derive(thiserror::Error, Debug, Clone)]
pub enum Error {
    #[error("value is mandatory")]
    MandatoryAttribute,
    #[error("value cannot be empty")]
    Empty,
    #[error("file \"{file:?}\" has disallowed extension, allowed extensions are: {allowed:?}")]
    DisallowedExtension { file: PathBuf, allowed: Vec<String> },
    #[error("config transition not implemented: {0}")]
    ConfigTransitionNotImplemented(String),
    #[error("mandatory and default are mutually exclusive")]
    MandatoryAndDefaultMutuallyExclusive,
    #[error("value {0} is not in allowed set")]
    IntNotAllowed(i32),
    #[error("value {0:?} is not in allowed set")]
    StringNotAllowed(String),
    #[error("allow_files and allow_single_file are mutually exclusive")]
    AllowFilesMutuallyExclusive,
    #[error("allow_empty = False requires the attribute to be mandatory or have a non-empty default value")]
    AllowEmptyRequiresMandatoryOrDefault,
    #[error("value `{0}` is not a label")]
    NotALabel(String),
    #[error("label `{0}` is duplicated")]
    DuplicateLabel(crate::LabelOrFile),
    #[error("got {0}, want value in signed 32-bit range")]
    Int32Expected(i64),
}

impl From<Error> for starlark::Error {
    fn from(err: Error) -> Self {
        Self::new_other(err)
    }
}

impl UnpackValueError for Error {
    fn into_error(this: Self) -> starlark::Error {
        starlark::Error::new_other(this)
    }
}
