// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

/// Errors returned by provider validation, unpacking, and extraction.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("The elements of provider fields must be strings")]
    FieldsMustBeStrings,
    #[error("Provider fields must be an iterable")]
    FieldsMustBeIterable,
    #[error("Duplicate field name: {0}")]
    DuplicateFieldName(String),
    #[error("The result of provider() must be assigned to a variable")]
    ProviderNotExported,
}

impl From<Error> for starlark::Error {
    fn from(err: Error) -> Self {
        Self::new_other(err)
    }
}
