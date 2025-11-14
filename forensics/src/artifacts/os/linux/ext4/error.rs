use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum Ext4Error {
    RootDir,
    Regex,
    Device,
}

impl std::error::Error for Ext4Error {}

impl fmt::Display for Ext4Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ext4Error::RootDir => write!(f, "Failed to get EXT4 root directory"),
            Ext4Error::Regex => write!(f, "Bad regex provided"),
            Ext4Error::Device => write!(f, "Could not open the device"),
        }
    }
}
