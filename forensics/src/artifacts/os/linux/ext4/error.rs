use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub enum Ext4Error {
    RootDir,
    Regex,
    Device,
    QcowDevice,
    QcowExt4Boot,
}

impl std::error::Error for Ext4Error {}

impl fmt::Display for Ext4Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ext4Error::RootDir => write!(f, "Failed to get EXT4 root directory"),
            Ext4Error::Regex => write!(f, "Bad regex provided"),
            Ext4Error::Device => write!(f, "Could not open the device"),
            Ext4Error::QcowDevice => write!(f, "Could not open QCOW disk"),
            Ext4Error::QcowExt4Boot => write!(f, "Could not read the QCOW ext4 boot info"),
        }
    }
}
