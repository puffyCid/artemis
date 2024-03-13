use std::fmt;

#[derive(Debug)]
pub(crate) enum WinArtifactError {
    Prefetch,
    EventLogs,
    Ntfs,
    Output,
    Serialize,
    Shimdb,
    Registry,
    UserAssist,
    Shimcache,
    Shellbag,
    Amcache,
    Shortcuts,
    UsnJrnl,
    Bits,
    Srum,
    Users,
    Search,
    Tasks,
    Services,
    Jumplists,
    RecycleBin,
    WmiPersist,
}

impl std::error::Error for WinArtifactError {}

impl fmt::Display for WinArtifactError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WinArtifactError::Prefetch => write!(f, "Failed to parse Prefetch"),
            WinArtifactError::EventLogs => write!(f, "Failed to parse EventLogs"),
            WinArtifactError::Ntfs => write!(f, "Failed to parse NTFS"),
            WinArtifactError::Output => write!(f, "Failed to output data"),
            WinArtifactError::Serialize => write!(f, "Artemis failed serialize artifact data"),
            WinArtifactError::Shimdb => write!(f, "Failed to parse Shimdb"),
            WinArtifactError::Registry => write!(f, "Failed to parse Registry"),
            WinArtifactError::UserAssist => write!(f, "Failed to parse UserAssist"),
            WinArtifactError::Shimcache => write!(f, "Failed to parse Shimcache"),
            WinArtifactError::Shellbag => write!(f, "Failed to parse Shellbags"),
            WinArtifactError::Amcache => write!(f, "Failed to parse Amcache"),
            WinArtifactError::Shortcuts => write!(f, "Failed to parse Shortcuts"),
            WinArtifactError::UsnJrnl => write!(f, "Failed to parse UsnJrnl"),
            WinArtifactError::Bits => write!(f, "Failed to parse Bits"),
            WinArtifactError::Srum => write!(f, "Failed to parse SRUM"),
            WinArtifactError::Search => write!(f, "Failed to parse Search"),
            WinArtifactError::Users => write!(f, "Failed to parse Users"),
            WinArtifactError::Tasks => write!(f, "Failed to parse Schedule Tasks"),
            WinArtifactError::Services => write!(f, "Failed to parse Services"),
            WinArtifactError::Jumplists => write!(f, "Failed to parse Jumplists"),
            WinArtifactError::RecycleBin => write!(f, "Failed to parse Recycle Bin"),
            WinArtifactError::WmiPersist => write!(f, "Failed to parse WMI persist"),
        }
    }
}
