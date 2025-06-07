use std::fmt;

#[derive(Debug)]
pub(crate) enum ShellError {
    UserPaths,
    File,
    Regex,
    SessionPath,
    Timestamp,
}

impl std::error::Error for ShellError {}

impl fmt::Display for ShellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShellError::UserPaths => {
                write!(f, "Failed to user paths")
            }
            ShellError::File => {
                write!(f, "Failed to open zsh file for user")
            }
            ShellError::Regex => {
                write!(f, "Failed to compile zsh regex")
            }
            ShellError::SessionPath => {
                write!(f, "Failed to list sessions")
            }
            ShellError::Timestamp => {
                write!(f, "Could not get timestamp for shell item")
            }
        }
    }
}
