use std::fmt;

#[derive(Debug)]
pub enum ProcessError {
    Empty,
    ParseProcFile,
    ProcessList,
    Serialize,
}

impl std::error::Error for ProcessError {}

impl fmt::Display for ProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcessError::Empty => {
                write!(f, "Got empty process listing on system")
            }
            ProcessError::ParseProcFile => {
                write!(f, "Failed to parse process binary")
            }
            ProcessError::ProcessList => {
                write!(f, "Failed to get process listing")
            }
            ProcessError::Serialize => {
                write!(f, "Failed to serialize process listing")
            }
        }
    }
}
