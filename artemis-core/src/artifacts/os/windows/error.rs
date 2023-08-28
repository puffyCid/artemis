use std::fmt;

#[derive(Debug)]
pub enum WinArtifactError {
    Prefetch,
    EventLogs,
    Ntfs,
    Output,
    BadToml,
    Serialize,
    Process,
    File,
    Shimdb,
    Registry,
    Format,
    UserAssist,
    Shimcache,
    Shellbag,
    Amcache,
    Shortcuts,
    UsnJrnl,
    Bits,
    Srum,
    FilterOutput,
    Users,
    Search,
    Tasks,
    Services,
    Jumplists,
    RecycleBin,
}

impl std::error::Error for WinArtifactError {}

impl fmt::Display for WinArtifactError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WinArtifactError::Prefetch => write!(f, "Failed to parse Prefetch"),
            WinArtifactError::EventLogs => write!(f, "Failed to parse EventLogs"),
            WinArtifactError::Ntfs => write!(f, "Failed to parse NTFS"),
            WinArtifactError::Output => write!(f, "Failed to output data"),
            WinArtifactError::BadToml => write!(f, "Artemis failed to parse TOML data"),
            WinArtifactError::Serialize => write!(f, "Artemis failed serialize artifact data"),
            WinArtifactError::Process => write!(f, "Failed to parse Procsses"),
            WinArtifactError::File => write!(f, "Failed to get file listing"),
            WinArtifactError::Shimdb => write!(f, "Failed to parse Shimdb"),
            WinArtifactError::Registry => write!(f, "Failed to parse Registry"),
            WinArtifactError::Format => write!(f, "Bad format option"),
            WinArtifactError::UserAssist => write!(f, "Failed to parse UserAssist"),
            WinArtifactError::Shimcache => write!(f, "Failed to parse Shimcache"),
            WinArtifactError::Shellbag => write!(f, "Failed to parse Shellbags"),
            WinArtifactError::Amcache => write!(f, "Failed to parse Amcache"),
            WinArtifactError::Shortcuts => write!(f, "Failed to parse Shortcuts"),
            WinArtifactError::UsnJrnl => write!(f, "Failed to parse UsnJrnl"),
            WinArtifactError::Bits => write!(f, "Failed to parse Bits"),
            WinArtifactError::Srum => write!(f, "Failed to parse SRUM"),
            WinArtifactError::Search => write!(f, "Failed to parse Search"),
            WinArtifactError::FilterOutput => write!(f, "Failed to filter windows data"),
            WinArtifactError::Users => write!(f, "Failed to parse Users"),
            WinArtifactError::Tasks => write!(f, "Failed to parse Schedule Tasks"),
            WinArtifactError::Services => write!(f, "Failed to parse Services"),
            WinArtifactError::Jumplists => write!(f, "Failed to parse Jumplists"),
            WinArtifactError::RecycleBin => write!(f, "Failed to parse Recycle Bin"),
        }
    }
}
