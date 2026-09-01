use crate::accessor::{
    error::{AccessorError, AccessorResult},
    location::path::is_absolute_host_path,
};

/// Accces method to use when accessing data
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Scheme {
    /// Access the data using OS APIs
    Host,
    /// Access the data using raw NTFS disk access
    Ntfs,
    /// Access the data inside a zip file
    Zip,
}

impl Scheme {
    /// Return the `Scheme` as string
    pub(crate) fn as_str(&self) -> &str {
        match self {
            Self::Host => "host",
            Self::Ntfs => "ntfs",
            Self::Zip => "zip",
        }
    }

    //// Parse the input into a supported `Scheme`
    pub(crate) fn parse(value: &str) -> AccessorResult<Self> {
        match value.to_ascii_lowercase().as_str() {
            "host" => Ok(Self::Host),
            "ntfs" => Ok(Self::Ntfs),
            "zip" => Ok(Self::Zip),
            _ => Err(AccessorError::unsupported_scheme(value)),
        }
    }
}

/// Determine the input `Scheme` when parsing the input location
/// If a location path is not absolute. Then we
/// default to `Scheme::Host`
///
/// `ntfs:C:\\Users\\test.txt` returns `Scheme::Ntfs`
///
/// `zip:/home/downloads/test.zip!./test` returns `Scheme::Zip`
///
/// `zip:file.zip!.test` is returns `Scheme::Host` since its a relative path
pub(crate) fn location_scheme(input: &str) -> Option<Scheme> {
    let scheme = scheme_prefix(input)?;
    let remainder = strip_scheme(input);

    if remainder.is_empty() {
        return Some(scheme);
    }

    match scheme {
        Scheme::Host | Scheme::Ntfs => is_absolute_host_path(remainder).then_some(scheme),
        Scheme::Zip => {
            let archive = remainder
                .split_once('!')
                .map_or(remainder, |(archive, _)| archive);
            is_absolute_host_path(archive).then_some(scheme)
        }
    }
}

/// Strip a supported scheme prefix. Windows drive letters such as `C:` are kept
pub(crate) fn strip_scheme(location: &str) -> &str {
    let Some((scheme, remainder)) = location.split_once(':') else {
        return location;
    };
    if scheme.len() <= 1 || Scheme::parse(scheme).is_err() {
        return location;
    }
    remainder
}

/// Split the scheme part of the input
///
/// Example: `ntfs:C:\Users\test.txt` into ('ntfs', and 'C:\Users\test.txt')
pub(crate) fn split_scheme_prefix(input: &str) -> Option<(&str, &str)> {
    let (scheme, remainder) = input.split_once(':')?;
    // If we get a drive letter for Windows treat that as live system
    // Ex: 'C:\\Users\\test.txt' The scheme would be 'C'
    if scheme.is_empty() || scheme.len() == 1 {
        return None;
    }
    Some((scheme, remainder))
}

/// Check the input path to see if matches a supported `Scheme`
pub(crate) fn scheme_prefix(input: &str) -> Option<Scheme> {
    let (scheme, _) = split_scheme_prefix(input)?;
    Scheme::parse(scheme).ok()
}
