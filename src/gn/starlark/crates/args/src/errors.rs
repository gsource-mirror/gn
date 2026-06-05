/// Errors returned by action command line argument parsing and formatting.
#[derive(thiserror::Error, Debug)]
pub enum Error {
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
    #[error("Format string must contain exactly one '%s', got: {0}")]
    InvalidFormatString(String),
    #[error("Expected Args object")]
    ExpectedArgsObject,
    #[error("Expected first argument of add to be a string flag when value is specified")]
    ExpectedAddStringFlag,
    #[error("Expected first argument of add_all to be a string flag when values is specified")]
    ExpectedAddAllStringFlag,
    #[error("Expected first argument of add_joined to be a string flag when values is specified")]
    ExpectedAddJoinedStringFlag,
}

impl From<Error> for starlark::Error {
    fn from(err: Error) -> Self {
        Self::new_other(err)
    }
}
