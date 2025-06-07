use std::fmt;

#[derive(Debug)]
pub(crate) enum CollectionError {
    Output,
    FilterOutput,
    Format,
}

impl std::error::Error for CollectionError {}

impl fmt::Display for CollectionError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CollectionError::Output => write!(f, "Failed to output data"),
            CollectionError::FilterOutput => write!(f, "Failed to filter macos data"),
            CollectionError::Format => write!(f, "Unknown formatter provided"),
        }
    }
}
