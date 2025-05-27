use std::fmt;

#[derive(Debug)]
pub enum EnrollError {
    FailedEnrollment,
    BadEnrollment,
    EnrollmentNotOk,
}

impl std::error::Error for EnrollError {}

impl fmt::Display for EnrollError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EnrollError::FailedEnrollment => write!(f, "Failed to enroll endpoint"),
            EnrollError::BadEnrollment => write!(f, "Enroll request was bad"),
            EnrollError::EnrollmentNotOk => write!(f, "Server returned non-Ok response"),
        }
    }
}
