use std::fmt;

#[derive(Debug)]
pub enum TaskError {
    ReadXml,
    UtfType,
    DriveLetter,
    Glob,
}

impl std::error::Error for TaskError {}

impl fmt::Display for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskError::ReadXml => write!(f, "Failed to read Schedule Task XML"),
            TaskError::UtfType => write!(f, "Failed to determine UTF16 type"),
            TaskError::DriveLetter => write!(f, "Could not get drive letter"),
            TaskError::Glob => write!(f, "Could not glob data"),
        }
    }
}
