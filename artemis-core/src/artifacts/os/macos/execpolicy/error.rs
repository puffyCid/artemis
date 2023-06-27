use std::fmt;

#[derive(Debug)]
pub enum ExecPolicyError {
    PathError,
    SQLITEParseError,
    BadSQL,
}

impl std::error::Error for ExecPolicyError {}

impl fmt::Display for ExecPolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecPolicyError::PathError => write!(f, "Failed to get execpolicy file"),
            ExecPolicyError::BadSQL => write!(f, "Could not compose sqlite query"),
            ExecPolicyError::SQLITEParseError => {
                write!(f, "Failed to parse SQLITE execpolicy file")
            }
        }
    }
}
