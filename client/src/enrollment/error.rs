use std::fmt;

#[derive(Debug)]
pub enum EnrollError {
    Enroll,
    EnrollSerialize,
    EnrollDeserialize,
    EnrollRequest,
    EnrollBadResponse,
    CreateLayout,
}

impl fmt::Display for EnrollError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EnrollError::Enroll => write!(f, "Failed to enroll system"),
            EnrollError::EnrollSerialize => write!(f, "Failed to serialize enrollment"),
            EnrollError::EnrollDeserialize => write!(f, "Failed to deserialize enrollment"),
            EnrollError::EnrollRequest => write!(f, "Failed to send enroll request"),
            EnrollError::EnrollBadResponse => write!(f, "Bad response for enrollment"),
            EnrollError::CreateLayout => write!(f, "Could not create client layout"),
        }
    }
}
