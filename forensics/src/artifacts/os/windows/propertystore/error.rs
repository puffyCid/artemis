use std::fmt;

#[derive(Debug)]
pub enum StoreError {
    ParseProperty,
}

impl std::error::Error for StoreError {}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::ParseProperty => write!(f, "Failed to parse store data"),
        }
    }
}
