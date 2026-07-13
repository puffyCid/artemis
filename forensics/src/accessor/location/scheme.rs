use crate::accessor::error::{AccessorError, AccessorResult};

/// Accces method to use when accessing data
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Scheme {
    /// Access the data using OS APIs
    Host,
    /// Access the data using raw NTFS disk access
    RawNtfs,
    /// Access the data inside a zip file
    Zip,
}

impl Scheme {
    /// Return the `Scheme` as string
    pub(crate) fn as_str(&self) -> &str {
        match self {
            Self::Host => "host",
            Self::RawNtfs => "ntfs",
            Self::Zip => "zip",
        }
    }

    //// Parse the input into a supported `Scheme`
    pub(crate) fn parse(value: &str) -> AccessorResult<Self> {
        match value.to_ascii_lowercase().as_str() {
            "host" => Ok(Self::Host),
            "ntfs" => Ok(Self::RawNtfs),
            "zip" => Ok(Self::Zip),
            _ => Err(AccessorError::unsupported_scheme(value)),
        }
    }
}
