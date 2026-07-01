use crate::accessor::{
    error::{AccessorError, AccessorResult},
    location::{
        path::{InnerPath, SourcePath, is_absolute_host_path, is_host_path, is_relative_host_path},
        scheme::Scheme,
    },
};
use std::path::PathBuf;

/// Parsed accessor location string
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Location {
    /// Scheme used to acccess the data
    pub(crate) scheme: Scheme,
    /// Optional source of the path
    pub(crate) source: Option<SourcePath>,
    /// Path to the data
    pub(crate) inner_path: InnerPath,
}

impl Location {
    /// Parse the provided input string into a `Location` structure
    pub(crate) fn parse(input: &str) -> AccessorResult<Self> {
        let value = input.trim();
        if value.is_empty() {
            return Err(AccessorError::location(value, "location cannot be empty"));
        }

        if let Some((source_part, inner_part)) = value.split_once('!') {
            return parse_schemed_location(source_part, Some(inner_part));
        }

        if let Some((scheme, remainder)) = split_scheme_prefix(value) {
            return parse_schemed_location(&format!("{scheme}:{remainder}"), None);
        }

        if is_host_path(input) {
            return Ok(Self {
                scheme: Scheme::Host,
                source: None,
                inner_path: InnerPath::new(PathBuf::from(input)),
            });
        }

        Err(AccessorError::location(
            value,
            "expected an absolute host path or a scheme prefix such as host:, raw:, or zip:",
        ))
    }

    /// Parse just the source of the data into a `Location` structure
    pub(crate) fn parse_source(input: &str) -> AccessorResult<Self> {
        let value = input.trim();
        if value.is_empty() {
            return Err(AccessorError::location(value, "source cannot be empty"));
        }

        if value.contains('!') {
            return Err(AccessorError::location(
                value,
                "source strings cannot contain '!'",
            ));
        }

        if let Some((scheme, remainder)) = split_scheme_prefix(value) {
            let scheme_value = Scheme::parse(scheme)?;
            if scheme_value == Scheme::Host && !remainder.is_empty() {
                return Err(AccessorError::location(
                    value,
                    "host source must be written as host: with no trailing path",
                ));
            }
            let source = parse_source_path(scheme_value, remainder)?;
            return Ok(Self {
                scheme: scheme_value,
                source,
                inner_path: InnerPath::empty(),
            });
        }

        if is_absolute_host_path(value) {
            return Err(AccessorError::location(
                input,
                "expected a source spec such as host:, raw:C:, or zip:/path/archive.zip",
            ));
        }

        Err(AccessorError::location(
            input,
            "expected a source spec such as host:, raw:C:, or zip:/path/archive.zip",
        ))
    }

    /// Split a glob or read call input into location prefix and trailing pattern
    ///
    /// Example: `/var/log/*.log` -> (`/var/log/`, `*.log`)
    pub(crate) fn split_glob_pattern(input: &str) -> AccessorResult<(Self, String)> {
        let value = input.trim();
        if value.is_empty() {
            return Err(AccessorError::location(value, "glob input cannot be empty"));
        }

        let split_at = value.rfind(['*', '?', '[']).ok_or_else(|| {
            AccessorError::location(input, "glob pattern must contain a wildcard")
        })?;
        let (location_part, pattern) = value.split_at(split_at);
        let pattern = pattern.trim_start_matches('/').trim_start_matches('\\');

        if pattern.is_empty() {
            return Err(AccessorError::location(
                value,
                "glob pattern must contain a wildcard",
            ));
        }

        let location = if location_part.is_empty() {
            Self {
                scheme: Scheme::Host,
                source: None,
                inner_path: InnerPath::empty(),
            }
        } else {
            Self::parse(location_part.trim_end_matches('/').trim_end_matches('\\'))?
        };

        Ok((location, pattern.to_string()))
    }
}

/// Parse Scheme prefix into a `Location` structure
fn parse_schemed_location(source_part: &str, inner_part: Option<&str>) -> AccessorResult<Location> {
    let (scheme, remainder) = split_scheme_prefix(source_part).ok_or_else(|| {
        AccessorError::location(
            source_part,
            "expected a scheme prefix such as host:, raw:, or zip:",
        )
    })?;

    let scheme = Scheme::parse(scheme)?;
    let source = parse_source_path(scheme, remainder)?;
    let inner_path = match inner_part {
        Some(value) => InnerPath::normalize_container_path(value)?,
        None => parse_inner_path(scheme, remainder)?,
    };

    Ok(Location {
        scheme,
        source,
        inner_path,
    })
}

/// Split the scheme part of the input
///
/// Example: `raw:C:\Users\test.txt` into ('raw', and 'C:\Users\test.txt')
fn split_scheme_prefix(input: &str) -> Option<(&str, &str)> {
    let (scheme, remainder) = input.split_once(':')?;
    if scheme.is_empty() || !scheme.chars().all(|ch| ch.is_ascii_alphabetic()) {
        return None;
    }
    Some((scheme, remainder))
}

/// Determine the `SourcePath` based on `Scheme` and remaining path
fn parse_source_path(scheme: Scheme, remainder: &str) -> AccessorResult<Option<SourcePath>> {
    match scheme {
        Scheme::Host => Ok(None),
        Scheme::Raw => parse_raw_source(remainder),
        Scheme::Zip => {
            if remainder.is_empty() {
                return Err(AccessorError::location(
                    remainder,
                    "zip source requires an archive path",
                ));
            }
            if !is_host_path(remainder) {
                return Err(AccessorError::location(
                    remainder,
                    "zip archive paths must be absolute or relative host paths",
                ));
            }
            Ok(Some(SourcePath::new(PathBuf::from(remainder))))
        }
    }
}

/// Parse `SourcePath` if using raw access
fn parse_raw_source(remainder: &str) -> AccessorResult<Option<SourcePath>> {
    if remainder.is_empty() {
        return Err(AccessorError::location(
            remainder,
            "raw source requires a drive letter such as raw:C:",
        ));
    }

    if is_absolute_host_path(remainder) {
        let drive = remainder
            .chars()
            .next()
            .ok_or_else(|| AccessorError::location(remainder, "raw path missing drive letter"))?;
        return Ok(Some(SourcePath::new(PathBuf::from(format!("{drive}:")))));
    }

    let drive = remainder
        .trim_end_matches(':')
        .chars()
        .next()
        .ok_or_else(|| AccessorError::location(remainder, "raw source requires a drive letter"))?;

    if !drive.is_ascii_alphabetic() {
        return Err(AccessorError::location(
            remainder,
            "raw source drive letter must be alphabetic",
        ));
    }

    Ok(Some(SourcePath::new(PathBuf::from(format!("{drive}:")))))
}

/// Identify the inner path of a `Scheme`
///
/// Example: `zip:data.zip!/home/test.txt` returns `/home/test.txt` for `InnerPath`
fn parse_inner_path(scheme: Scheme, remainder: &str) -> AccessorResult<InnerPath> {
    match scheme {
        Scheme::Host => {
            if remainder.is_empty() {
                return Err(AccessorError::location(
                    remainder,
                    "host location requires a path",
                ));
            }
            Ok(InnerPath::new(PathBuf::from(remainder)))
        }
        Scheme::Raw => {
            if remainder.is_empty() {
                return Err(AccessorError::location(
                    remainder,
                    "raw location requires a path",
                ));
            }
            if is_relative_host_path(remainder) {
                return Err(AccessorError::location(
                    remainder,
                    "raw locations require an absolute path",
                ));
            }
            Ok(InnerPath::new(PathBuf::from(remainder)))
        }
        Scheme::Zip => Ok(InnerPath::empty()),
    }
}

#[cfg(test)]
mod tests {
    use crate::accessor::{
        error::AccessorError,
        location::{location::Location, scheme::Scheme},
    };

    #[test]
    fn test_location() {
        let test = "zip:data.zip!./home/test.txt";
        let result = Location::parse(test).unwrap();
        assert_eq!(result.scheme, Scheme::Zip);
        assert_eq!(result.inner_path.display(), "home/test.txt");
        assert_eq!(result.source.unwrap().display(), "data.zip");
    }

    #[test]
    fn test_location_raw() {
        let test = "raw:C:\\home\\test.txt";
        let result = Location::parse(test).unwrap();
        assert_eq!(result.scheme, Scheme::Raw);
        assert_eq!(result.inner_path.display(), "C:\\home\\test.txt");
        assert_eq!(result.source.unwrap().display(), "C:");
    }

    #[test]
    fn test_location_host() {
        let test = "/etc/host";
        let result = Location::parse(test).unwrap();
        assert_eq!(result.scheme, Scheme::Host);
        assert_eq!(result.inner_path.display(), "/etc/host");
        assert!(result.source.is_none());
    }

    #[test]
    fn test_location_source() {
        let test = "zip:/home/test.zip";
        let result = Location::parse_source(test).unwrap();
        assert_eq!(result.source.unwrap().display(), "/home/test.zip");
        assert_eq!(result.scheme, Scheme::Zip);
    }

    #[test]
    fn test_location_glob() {
        let test = "/var/logs/*.log";
        let (result, pattern) = Location::split_glob_pattern(test).unwrap();
        assert!(result.source.is_none());
        assert_eq!(result.scheme, Scheme::Host);
        assert_eq!(pattern, "*.log");
    }

    #[test]
    fn test_location_empty() {
        let err = Location::parse("").unwrap_err();
        assert!(
            matches!(err, AccessorError::Location { reason,.. } if reason.contains("location cannot be empty"))
        );
    }
}
