use std::fmt;

#[derive(Debug)]
pub enum DbError {
    EndpointDb,
    JobDb,
    NoDb,
    BeginRead,
    OpenTable,
    Get,
    Open,
    BeginWrite,
    Insert,
    Serialize,
    Deserialize,
    Commit,
}

impl fmt::Display for DbError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DbError::EndpointDb => write!(f, "Could not add endpoint to DB"),
            DbError::JobDb => write!(f, "Could not add job to DB"),
            DbError::NoDb => write!(f, "No DB file to open"),
            DbError::BeginRead => write!(f, "Could not begin DB read"),
            DbError::BeginWrite => write!(f, "Could not begin DB write"),
            DbError::OpenTable => write!(f, "Could not open table"),
            DbError::Get => write!(f, "Could not get DB value"),
            DbError::Insert => write!(f, "Could not insert DB value"),
            DbError::Open => write!(f, "Could not open DB"),
            DbError::Serialize => write!(f, "Could not serialize DB data"),
            DbError::Deserialize => write!(f, "Could not deserialize DB data"),
            DbError::Commit => write!(f, "Could not commit DB data"),
        }
    }
}
