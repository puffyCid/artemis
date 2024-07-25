use std::fmt;

#[derive(Debug)]
pub enum StoreError {
    CreateDirectory,
    WriteFile,
    ReadFile,
    Serialize,
    Deserialize,
    BadGlob,
    NoCollection,
    DuplicateCollectionId,
}

impl fmt::Display for StoreError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::CreateDirectory => write!(f, "Could not create endpoint directory"),
            StoreError::WriteFile => write!(f, "Could not write file"),
            StoreError::ReadFile => write!(f, "Could not read file"),
            StoreError::Serialize => write!(f, "Could not serialize filestore data"),
            StoreError::Deserialize => write!(f, "Could not deserialize filestore data"),
            StoreError::BadGlob => write!(f, "Bad glob provided"),
            StoreError::NoCollection => write!(f, "No collection id found"),
            StoreError::DuplicateCollectionId => write!(f, "Collection ID already created"),
        }
    }
}
