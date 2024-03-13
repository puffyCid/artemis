use std::fmt;

#[derive(Debug)]
pub(crate) enum CronError {
    FileRead,
    ReadPath,
}

impl std::error::Error for CronError {}

impl fmt::Display for CronError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CronError::FileRead => {
                write!(f, "Failed to read cron file")
            }
            CronError::ReadPath => {
                write!(f, "Failed to get cron path")
            }
        }
    }
}
