use std::fmt;

#[derive(Debug)]
pub enum TaskError {
    ReadXml,
    UtfType,
}

impl std::error::Error for TaskError {}

impl fmt::Display for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskError::ReadXml => write!(f, "Failed to read Schedule Task XML"),
            TaskError::UtfType => write!(f, "Failed to determine UTF16 type"),
        }
    }
}
