use std::fmt;

#[derive(Debug, PartialEq)]
pub(crate) enum CollectError {
    FailedCollect,
    BadCollect,
    CollectNotOk,
    NoCollection,
}

impl std::error::Error for CollectError {}

impl fmt::Display for CollectError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CollectError::FailedCollect => write!(f, "Failed to get collection for endpoint"),
            CollectError::BadCollect => write!(f, "Collection request data was bad"),
            CollectError::CollectNotOk => write!(f, "Server returned non-Ok response"),
            CollectError::NoCollection => write!(f, "Server does not have any collections for us"),
        }
    }
}
