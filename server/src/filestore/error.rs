use std::fmt;

#[derive(Debug)]
pub enum StoreError {
    Endpoint,
    Job,
    NoFile,
    CreateDirectory,
    WriteFile,
    ReadFile,
    Serialize,
    Deserialize,
}

impl fmt::Display for StoreError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::Endpoint => write!(f, "Could not add endpoint to DB"),
            StoreError::Job => write!(f, "Could not add job to DB"),
            StoreError::NoFile => write!(f, "No file to open"),
            StoreError::CreateDirectory => write!(f, "Could not create endpoint directory"),
            StoreError::WriteFile => write!(f, "Could not write file"),
            StoreError::ReadFile => write!(f, "Could not read file"),
            StoreError::Serialize => write!(f, "Could not serialize DB data"),
            StoreError::Deserialize => write!(f, "Could not deserialize DB data"),
        }
    }
}
