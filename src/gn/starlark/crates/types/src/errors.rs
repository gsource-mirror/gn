use std::path::PathBuf;

use starlark::values::UnpackValueError;

use crate::Package;

/// Errors returned by this crate.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// The string is not a valid label (e.g. doesn't start with "//" or ":").
    #[error("Not a label: {0}")]
    NotALabel(String),
    /// An absolute label is missing a colon.
    #[error("Absolute label must contain a colon: {0}")]
    AbsoluteLabelWithoutColon(String),
    /// A relative label contains a colon (which is invalid).
    #[error("Relative label cannot contain a colon: {0}")]
    ColonInRelativeLabel(String),
    /// The referenced file does not exist on disk.
    #[error("File {1} does not exist in {0}")]
    FileNotFound(Package, String),
    /// Failed to freeze value.
    #[error("Failed to freeze value: {0}")]
    FreezeFailed(String),
    /// Error during target creation.
    #[error("Target creation error: {0}")]
    TargetCreationError(String),

    /// Attribute is mandatory but was not provided.
    #[error("Attribute `{param}` is mandatory")]
    MandatoryAttribute {
        /// The parameter name.
        param: String,
    },
    /// Empty list is not allowed for this attribute.
    #[error("Want non-empty list, got []")]
    EmptyListDisallowed,
    /// Empty dict is not allowed for this attribute.
    #[error("Want non-empty dict, got {{}}")]
    EmptyDictDisallowed,
    /// File extension is not allowed.
    #[error("File \"{file:?}\" has disallowed extension, allowed extensions are: {allowed:?}")]
    DisallowedExtension {
        /// The file path.
        file: PathBuf,
        /// The allowed extensions.
        allowed: Vec<String>,
    },
    /// Config transition is not implemented.
    #[error("Config transition not implemented: {0}")]
    ConfigTransitionNotImplemented(String),
    /// Failed to read a file or source.
    #[error("Failed to read {0}")]
    ReadFailed(String),
    /// mandatory and default attributes are mutually exclusive.
    #[error("mandatory and default are mutually exclusive")]
    MandatoryAndDefaultMutuallyExclusive,
    /// The target must produce a single file.
    #[error("'{0}' must produce a single file")]
    MustProduceSingleFile(String),

    /// Integer value is not in the allowed set.
    #[error("Value {0} is not in allowed set")]
    IntNotAllowed(i32),
    /// String value is not in the allowed set.
    #[error("Value \"{0}\" is not in allowed set")]
    StringNotAllowed(String),

    /// allow_files and allow_single_file are mutually exclusive.
    #[error("allow_files and allow_single_file are mutually exclusive")]
    AllowFilesMutuallyExclusive,
    /// allow_empty = False requires the attribute to be mandatory or have a non-empty default.
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
