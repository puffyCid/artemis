use std::fmt;

#[derive(Debug)]
pub enum DecryptError {
    AesDecrypt,
    WrongAesKeyLength,
}

impl std::error::Error for DecryptError {}

impl fmt::Display for DecryptError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecryptError::AesDecrypt => write!(f, "Failed to decrypt AES"),
            DecryptError::WrongAesKeyLength => write!(f, "Key fewer than 64 bytes"),
        }
    }
}
