use std::fmt;

#[derive(Debug)]
pub(crate) enum FormatError {
    Output,
}

impl std::error::Error for FormatError {}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormatError::Output => write!(f, "Could not output data"),
        }
    }
}
