use std::fmt;

#[derive(Debug)]
pub(crate) enum SpotlightError {
    Glob,
    ReadFile,
    Header,
    Offsets,
    Property,
    Category,
    Indexes1,
    Indexes2,
    StoreHeader,
    StoreSeek,
    StoreRead,
    StoreMap,
    Serialize,
}

impl std::error::Error for SpotlightError {}

impl fmt::Display for SpotlightError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpotlightError::Glob => write!(f, "Could not glob dstr files"),
            SpotlightError::ReadFile => write!(f, "Could not read file"),
            SpotlightError::Header => write!(f, "Could not parse dbstr header"),
            SpotlightError::Offsets => write!(f, "Could not parse dbstr offsts"),
            SpotlightError::Property => write!(f, "Could not parse dbstr property"),
            SpotlightError::Category => write!(f, "Could not parse dbstr category"),
            SpotlightError::Indexes1 => write!(f, "Could not parse dbstr indexes1"),
            SpotlightError::Indexes2 => write!(f, "Could not parse dbstr indexes2"),
            SpotlightError::StoreHeader => write!(f, "Could not parse store header"),
            SpotlightError::StoreSeek => write!(f, "Could not seek store data"),
            SpotlightError::StoreRead => write!(f, "Could not read store data"),
            SpotlightError::StoreMap => write!(f, "Could not parse store map"),
            SpotlightError::Serialize => write!(f, "Could not serialize spotlight data"),
        }
    }
}
