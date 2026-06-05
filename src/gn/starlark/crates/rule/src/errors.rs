// Copyright 2026 The Chromium Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

/// Errors returned by the GN Starlark rule system.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Rule extensions cannot be invoked directly - invoke the underlying rule instead")]
    ArgumentToRuleMustBeATarget,
    #[error("Custom rule requires a name argument")]
    CustomRuleNameRequired,
    #[error("Rule extension requires a target argument")]
    ExtensionTargetRequired,
    #[error("Attribute 'name' is forbidden")]
    NameAttrForbidden,
    #[error("Target creation error: {0}")]
    TargetCreationError(String),
    #[error("Rule must be assigned to a global variable to be used")]
    RuleMustBeNamed,
}

impl From<Error> for starlark::Error {
    fn from(err: Error) -> Self {
        starlark::Error::new_other(err)
    }
}
