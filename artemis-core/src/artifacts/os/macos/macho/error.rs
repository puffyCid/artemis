use std::fmt;

#[derive(Debug, PartialEq)]
pub(crate) enum MachoError {
    FatHeader,
    Header,
    Data,
    Magic,
    Path,
    Buffer,
}

impl std::error::Error for MachoError {}

impl fmt::Display for MachoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MachoError::Header => write!(f, "Failed to read macho header"),
            MachoError::FatHeader => write!(f, "Failed to read fat macho header"),
            MachoError::Data => write!(f, "Failed to read macho data"),
            MachoError::Magic => write!(f, "Failed to check macho magic signature"),
            MachoError::Path => write!(f, "Failed to get file reader from provided path"),
            MachoError::Buffer => write!(f, "Failed to read contents into buffer"),
        }
    }
}
