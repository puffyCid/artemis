use std::fmt;

#[derive(Debug)]
pub(crate) enum RuntimeError {
    Decode,
    CreateUri,
    ExecuteScript,
    ScriptResult,
    Format,
    Output,
}

impl std::error::Error for RuntimeError {}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::Decode => write!(f, "Could not base64 javascript script"),
            RuntimeError::CreateUri => write!(f, "Could not create dummy uri"),
            RuntimeError::ExecuteScript => write!(f, "Could not run script"),
            RuntimeError::ScriptResult => write!(f, "Could not get script result"),
            RuntimeError::Format => write!(f, "Unknown output format"),
            RuntimeError::Output => write!(f, "Could not output data"),
        }
    }
}
