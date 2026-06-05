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
    #[error("Target creation error: {0}")]
    TargetCreationError(String),
    #[error("Rule must be assigned to a global variable to be used")]
    RuleMustBeNamed,
    #[error("Parent must be a rule")]
    ParentMustBeARule,
    #[error("Attribute '{0}' is reserved")]
    ReservedAttribute(String),
    #[error("attribute '{0}': only attr.label and attr.label_list types may be overridden")]
    OnlyLabelTypesMayBeOverridden(String),
    #[error("attribute '{0}': types of parent and child's attributes mismatch")]
    AttributeTypeMismatch(String),
}

impl From<Error> for starlark::Error {
    fn from(err: Error) -> Self {
        starlark::Error::new_other(err)
    }
}
