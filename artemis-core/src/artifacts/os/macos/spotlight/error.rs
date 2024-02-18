use std::fmt;

#[derive(Debug)]
pub(crate) enum SpotlightError {
    Glob,
    ReadFile,
    Header,
    Offsets,
    Data,
    Property,
    Category,
    Indexes1,
    Indexes2,
}

impl std::error::Error for SpotlightError {}

impl fmt::Display for SpotlightError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpotlightError::Glob => write!(f, "Could not glob dstr files"),
            SpotlightError::ReadFile => write!(f, "Could not read file"),
            SpotlightError::Header => write!(f, "Could not parse dbstr header"),
            SpotlightError::Offsets => write!(f, "Could not parse dbstr offsts"),
            SpotlightError::Data => write!(f, "Could not parse dbstr data"),
            SpotlightError::Property => write!(f, "Could not parse dbstr property"),
            SpotlightError::Category => write!(f, "Could not parse dbstr category"),
            SpotlightError::Indexes1 => write!(f, "Could not parse dbstr indexes1"),
            SpotlightError::Indexes2 => write!(f, "Could not parse dbstr indexes2"),
        }
    }
}
