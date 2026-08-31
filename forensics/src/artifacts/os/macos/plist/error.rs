use std::fmt;

#[derive(Debug, PartialEq)]
pub(crate) enum PlistError {
    Dictionary,
    Array,
    Float,
    File,
}

impl std::error::Error for PlistError {}

impl fmt::Display for PlistError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlistError::Dictionary => write!(f, "Not a plist dictionary value"),
            PlistError::Array => write!(f, "Not a plist array value"),
            PlistError::Float => write!(f, "Not a plist float value"),
            PlistError::File => write!(f, "Could not read plist file"),
        }
    }
}
