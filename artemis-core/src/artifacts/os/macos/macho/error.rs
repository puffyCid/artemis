use std::fmt;

#[derive(Debug)]
pub(crate) enum MachoError {
    FatHeader,
    Header,
    Data,
    Magic,
}

impl std::error::Error for MachoError {}

impl fmt::Display for MachoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MachoError::Header => write!(f, "Failed to read macho header"),
            MachoError::FatHeader => write!(f, "Failed to read fat macho header"),
            MachoError::Data => write!(f, "Failed to read macho data"),
            MachoError::Magic => write!(f, "Failed to check macho magic signature"),
        }
    }
}
