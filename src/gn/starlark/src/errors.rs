// Copyright 2026 The GN Authors. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
// Leave this as pub(crate) so we can find unused variants.
pub(crate) enum Error {
    #[error("Attribute `{param}` is mandatory")]
    MandatoryAttribute { param: String },
    #[error("Want non-empty list, got []")]
    EmptyListDisallowed,
    #[error("Want non-empty dict, got {{}}")]
    EmptyDictDisallowed,
    #[error("File \"{file:?}\" has disallowed extension, allowed extensions are: {allowed:?}")]
    DisallowedExtension { file: PathBuf, allowed: Vec<String> },
    #[error("not a label: {0}")]
    NotALabel(String),
    #[error("Value {0} is not in allowed set")]
    IntNotAllowed(i32),
    #[error("Value \"{0}\" is not in allowed set")]
    StringNotAllowed(String),

    #[error("allow_files and allow_single_file are mutually exclusive")]
    AllowFilesMutuallyExclusive,

    #[error("mandatory and default are mutually exclusive")]
    MandatoryAndDefaultMutuallyExclusive,

    #[error("Config transition not implemented: {0}")]
    ConfigTransitionNotImplemented(String),

    #[error("File {1} does not exist in {0}")]
    FileNotFound(crate::label::Package, String),

    // ProviderError variants
    #[error("provider fields must be strings")]
    FieldsMustBeStrings,

    // LabelError variants
    #[error("Absolute label must contain a colon: {0}")]
    AbsoluteLabelWithoutColon(String),
    #[error("Relative label cannot contain a colon: {0}")]
    ColonInRelativeLabel(String),

    // ActionError variants
    #[error("must be an iterable of strings, depsets, or Args objects")]
    IterableRequired,
    #[error("arguments must be strings, depsets, or Args objects, got: {0}")]
    InvalidArgumentType(String),
    #[error("Not formattable")]
    NotFormattable,
    #[error("arguments must be list or depset")]
    ArgumentsMustBeListOrDepset,
    #[error("map_each must return a list[str], str, or None")]
    MapEachInvalidReturn,
    #[error("File '{0}' has already been declared")]
    FileAlreadyDeclared(String),

    // TargetError variants
    #[error("Expected provider type")]
    ExpectedProviderType,
    // The only way we could possibly attempt to freeze a target is if it was placed in a provider.
    #[error("targets cannot be stored in providers")]
    CannotFreezeTarget,

    // GlobalsError variants
    #[error("{0} is only allowed in macros")]
    OnlyAllowedInMacros(String),
    #[error("{0}")]
    TargetCreationError(String),

    // DepsetError variants
    #[error("Invalid order: {0}")]
    InvalidOrder(String),
    #[error("transitive elements must be depsets, got type: {0}")]
    NotADepset(String),
    #[error("conflicting orders: depset has order {order}, but transitive child has order {child_order}")]
    ConflictingOrders { order: crate::depset::Order, child_order: crate::depset::Order },

    // UtilError variants
    #[error("Failed to freeze value: {0}")]
    FreezeFailed(String),
    #[error("Failed to retrieve value: {0}")]
    GetFailed(String),

    // SessionError variants
    #[error("Failed to read {0}")]
    ReadFailed(String),
    #[error("cycle detected: {0:?}")]
    CycleDetected(Vec<String>),

    // RuleError variants
    #[error("Extension requires a target argument")]
    ExtensionTargetRequired,
    #[error("Argument to rule must be a target")]
    ArgumentToRuleMustBeTarget,
    #[error("Custom rule requires a name argument")]
    CustomRuleNameRequired,

    #[error("Attribute 'name' is forbidden")]
    NameAttrForbidden,

    // ArgsError variants
    #[error("Format string must contain exactly one '%s', got: {0}")]
    InvalidFormatString(String),

    #[error("allow_empty = False requires the attribute to be mandatory or have a non-empty default value")]
    AllowEmptyRequiresMandatoryOrDefault,

    #[error("'{0}' must produce a single file")]
    MustProduceSingleFile(String),

    #[error("{0}")]
    GenericError(String),
}

impl From<Error> for starlark::Error {
    fn from(err: Error) -> Self {
        starlark::Error::new_other(err)
    }
}

impl starlark::values::UnpackValueError for Error {
    fn into_error(this: Self) -> starlark::Error {
        starlark::Error::new_other(this)
    }
}
