use std::fmt;

#[derive(Debug)]
pub enum AccountError {
    DefaultDrive,
    GetUserInfo,
}

impl std::error::Error for AccountError {}

impl fmt::Display for AccountError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccountError::DefaultDrive => write!(f, "Failed to get default driver letter"),
            AccountError::GetUserInfo => write!(f, "Failed to get user info"),
        }
    }
}
