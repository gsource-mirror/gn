// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use starlark::values::UnpackValueError;

/// Errors returned by the GN Starlark runtime system.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Expected provider type")]
    ExpectedProviderType,

    #[error("{0} is only allowed in macros")]
    OnlyAllowedIn(String),
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

/// Each type in starlark-rust "Freezes" to another type.
/// Some types are only visible in rule implementations (eg. Target, ctx).
///
/// Values can only escape rule implementations by being returned in providers.
/// We explicitly disallow freezing of certain types to ensure that a user does not, for example, write:
/// return [FooInfo(bar=ctx)]
#[macro_export]
macro_rules! cannot_freeze {
    ($type:ty) => {
        $crate::cannot_freeze!($type, $type);
    };
    ($type:ty, $frozen:ty) => {
        impl<'v> starlark::values::Freeze for $type {
            type Frozen = $frozen;
            fn freeze(
                self,
                _freezer: &starlark::values::Freezer,
            ) -> starlark::values::FreezeResult<Self::Frozen> {
                use starlark::values::type_repr::StarlarkTypeRepr;
                let ty = <$type as StarlarkTypeRepr>::starlark_type_repr();
                // This is for types only accessible in rule evaluation
                // contexts. The only way data can escape a rule evaluation
                // is via providers.
                Err(starlark::values::FreezeError::new(format!(
                    "{ty} cannot be stored in providers"
                )))
            }
        }
    };
}
