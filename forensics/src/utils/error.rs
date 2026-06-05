use std::fmt;

#[derive(Debug)]
pub enum ArtemisError {
    BadToml,
    Regex,
    Env,
    #[cfg(feature = "network")]
    Remote,
    ReadXml,
    UtfType,
    #[cfg(feature = "yarax")]
    Encoding,
    #[cfg(feature = "yarax")]
    YaraRule,
    #[cfg(feature = "yarax")]
    YaraScan,
    BadTime,
    Protobuf,
}

impl std::error::Error for ArtemisError {}

impl fmt::Display for ArtemisError {
    fn fmt<'a>(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArtemisError::BadToml => write!(f, "Failed to parse TOML data"),
            ArtemisError::Regex => write!(f, "Invalid regex provided"),
            ArtemisError::Env => write!(f, "Could not get environment variable"),
            #[cfg(feature = "network")]
            ArtemisError::Remote => write!(f, "Failed output data to remote URL"),
            ArtemisError::ReadXml => write!(f, "Failed to read XML"),
            ArtemisError::UtfType => write!(f, "Failed to determine UTF XML type"),
            #[cfg(feature = "yarax")]
            ArtemisError::Encoding => write!(f, "Failed to parse decoding/encoding"),
            #[cfg(feature = "yarax")]
            ArtemisError::YaraRule => write!(f, "Failed to add rule"),
            #[cfg(feature = "yarax")]
            ArtemisError::YaraScan => write!(f, "Failed to scan file"),
            ArtemisError::BadTime => write!(f, "Failed to parse rfc 33339 timestamp"),
            ArtemisError::Protobuf => write!(f, "Failed to parse protobuf bytes"),
        }
    }
}
