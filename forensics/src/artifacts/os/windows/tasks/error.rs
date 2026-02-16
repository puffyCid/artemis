use std::fmt;

#[derive(Debug)]
pub enum TaskError {
    ReadXml,
    ReadJob,
    DriveLetter,
    Glob,
    FixedSection,
    VariableSection,
    Jobs,
    Serialize,
}

impl std::error::Error for TaskError {}

impl fmt::Display for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskError::ReadXml => write!(f, "Failed to read Schedule Task XML"),
            TaskError::ReadJob => write!(f, "Failed to read Schedule Task Job"),
            TaskError::DriveLetter => write!(f, "Could not get drive letter"),
            TaskError::Glob => write!(f, "Could not glob data"),
            TaskError::FixedSection => write!(f, "Could not parse fixed data"),
            TaskError::VariableSection => write!(f, "Could not parse variable data"),
            TaskError::Jobs => write!(f, "Could not get jobs"),
            TaskError::Serialize => write!(f, "Could not serialize tasks"),
        }
    }
}
