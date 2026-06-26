use std::path::PathBuf;

use starlark::values::UnpackValueError;

use crate::Package;

/// Errors returned by types crate operations (such as label parsing or file validation).
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Not a label: {0}")]
    NotALabel(String),
    #[error("Absolute label must contain a colon: {0}")]
    AbsoluteLabelWithoutColon(String),
    #[error("Relative label cannot contain a colon: {0}")]
    ColonInRelativeLabel(String),
    #[error("File {1} does not exist in {0}")]
    FileNotFound(Package, String),
    #[error("Failed to freeze value: {0}")]
    FreezeFailed(String),
    #[error("Target creation error: {0}")]
    TargetCreationError(String),

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
