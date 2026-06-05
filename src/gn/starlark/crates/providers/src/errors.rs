/// Errors returned by provider validation, unpacking, and extraction.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Expected provider type")]
    ExpectedProviderType,
    #[error("provider fields must be strings")]
    FieldsMustBeStrings,
    #[error("Expected DefaultInfo to be a record")]
    ExpectedDefaultInfoRecord,
    #[error("Expected files in DefaultInfo to be a depset")]
    ExpectedDefaultInfoFilesDepset,
    #[error("Expected string or Args object in substitutions, got: {0}")]
    InvalidSubstitutionType(String),
    #[error("substitutions must be a struct, got type: {0}")]
    SubstitutionsMustBeStruct(String),
    #[error("substitution values must be a list, got type: {0}")]
    SubstitutionValueMustBeList(String),
}

impl From<Error> for starlark::Error {
    fn from(err: Error) -> Self {
        Self::new_other(err)
    }
}
