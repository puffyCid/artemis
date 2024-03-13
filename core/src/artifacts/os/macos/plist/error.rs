use std::fmt;

#[derive(Debug)]
pub(crate) enum PlistError {
    Dictionary,
    Data,
    String,
    Array,
    Float,
    File,
    Bool,
    SignedInt,
}

impl std::error::Error for PlistError {}

impl fmt::Display for PlistError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlistError::Dictionary => write!(f, "Not a plist dictionary value"),
            PlistError::Data => write!(f, "Not a plist binary value"),
            PlistError::String => write!(f, "Not a plist string value"),
            PlistError::Array => write!(f, "Not a plist array value"),
            PlistError::Float => write!(f, "Not a plist float value"),
            PlistError::File => write!(f, "Could not read plist file"),
            PlistError::Bool => write!(f, "Not a plist bool value"),
            PlistError::SignedInt => write!(f, "Not a plist signed int value"),
        }
    }
}
