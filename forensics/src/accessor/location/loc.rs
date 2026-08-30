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
    /// `Scheme` used to access the data
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

        if let Some(scheme) = scheme_prefix(value) {
            return match scheme {
                Scheme::Host | Scheme::Ntfs => parse_schemed_location(value, None),
                Scheme::Zip => {
                    if let Some((source_part, inner_part)) = value.split_once('!') {
                        parse_schemed_location(source_part, Some(inner_part))
                    } else {
                        parse_schemed_location(value, None)
                    }
                }
            };
        }

        // If we do not have a location scheme
        // We try to represent the input as data on a live system
        if is_host_path(value) {
            return Ok(Self {
                scheme: Scheme::Host,
                source: None,
                inner_path: InnerPath::new(PathBuf::from(value)),
            });
        }

        Err(AccessorError::location(
            value,
            "expected an absolute host path or a scheme prefix such as host:, ntfs:, or zip:",
        ))
    }

    /// Parse just the source of the data into a `Location` structure
    pub(crate) fn parse_source(input: &str) -> AccessorResult<Self> {
        let value = input.trim();
        if value.is_empty() {
            return Err(AccessorError::location(value, "source cannot be empty"));
        }

        // Determine the scheme of the data
        // Can be ntfs, host, zip, or others
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
                "expected a source spec such as host:, ntfs:C:, or zip:/path/archive.zip",
            ));
        }

        Err(AccessorError::location(
            input,
            "expected a source spec such as host:, ntfs:C:, or zip:/path/archive.zip",
        ))
    }

    /// Split a glob or read call input into location prefix and trailing pattern
    ///
    /// Example: `/var/log/*.log` -> (`/var/log/`, `*.log`)
    pub(crate) fn split_glob_pattern(input: &str) -> AccessorResult<(Self, String)> {
        // Check for disk images or container files
        // 'zip:test.zip!*' or in future 'dd:image.raw!/users/*/*.txt'
        if matches!(scheme_prefix(input), Some(Scheme::Zip))
            && let Some((source_path, inner_glob)) = input.split_once('!')
        {
            let (directory, pattern) = Self::parse_glob_pattern(inner_glob)?;

            // If the user provides a glob path of "zip:test.zip!/"
            // Trim forward or backslash and just glob the root directory
            let directory = directory.trim_start_matches(['/', '\\']);
            let location_str = if directory.is_empty() {
                source_path.to_string()
            } else {
                format!("{source_path}!{directory}")
            };
            let location = Self::parse(&location_str)?;

            return Ok((location, pattern));
        }

        let (directory, pattern) = Self::parse_glob_pattern(input)?;
        let location = if directory.is_empty() {
            Self {
                scheme: Scheme::Host,
                source: None,
                inner_path: InnerPath::empty(),
            }
        } else if matches!(directory.as_str(), "\\" | "/") {
            Self {
                scheme: Scheme::Host,
                source: None,
                inner_path: InnerPath::new(PathBuf::from(&directory)),
            }
        } else {
            Self::parse(directory.trim_end_matches('/').trim_end_matches('\\'))?
        };

        Ok((location, pattern))
    }

    /// Parse the provided glob pattern
    pub(crate) fn parse_glob_pattern(input: &str) -> AccessorResult<(String, String)> {
        let value = input.trim();
        if value.is_empty() {
            return Err(AccessorError::location(value, "glob input cannot be empty"));
        }

        let glob_at = value
            .char_indices()
            .find(|(_, ch)| matches!(ch, '*' | '?' | '['))
            .map_or(value.len(), |(index, _)| index);

        let (location_part, pattern) = match value[..glob_at].rfind(['/', '\\']) {
            Some(0) if matches!(value.as_bytes().first(), Some(b'/' | b'\\')) => {
                (value[..1].to_string(), value[1..].to_string())
            }
            Some(sep) => (value[..sep].to_string(), value[sep + 1..].to_string()),
            None => (String::new(), value.to_string()),
        };

        // If the user provide a directory path but no pattern. Treat as a single glob
        if value.ends_with(['/', '\\']) && pattern.is_empty() {
            return Ok((location_part, String::from("*")));
        }

        if pattern.is_empty() {
            return Err(AccessorError::location(
                value,
                "empty glob pattern must contain a wildcard",
            ));
        }

        Ok((location_part, pattern))
    }
}

/// Check the input path to see if matches a supported `Scheme`
fn scheme_prefix(input: &str) -> Option<Scheme> {
    let (scheme, _) = split_scheme_prefix(input)?;
    Scheme::parse(scheme).ok()
}

/// Parse Scheme prefix into a `Location` structure
fn parse_schemed_location(source_part: &str, inner_part: Option<&str>) -> AccessorResult<Location> {
    let (scheme, remainder) = split_scheme_prefix(source_part).ok_or_else(|| {
        AccessorError::location(
            source_part,
            "expected a scheme prefix such as host:, ntfs:, or zip:",
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
/// Example: `ntfs:C:\Users\test.txt` into ('ntfs', and 'C:\Users\test.txt')
fn split_scheme_prefix(input: &str) -> Option<(&str, &str)> {
    let (scheme, remainder) = input.split_once(':')?;
    // If we get a drive letter for Windows treat that as live system
    // Ex: 'C:\\Users\\test.txt' The scheme would be 'C'
    if scheme.is_empty() || scheme.len() == 1 {
        return None;
    }
    Some((scheme, remainder))
}

/// Determine the `SourcePath` based on `Scheme` and remaining path
fn parse_source_path(scheme: Scheme, remainder: &str) -> AccessorResult<Option<SourcePath>> {
    match scheme {
        Scheme::Host => Ok(None),
        Scheme::Ntfs => parse_raw_source(remainder, RawFileSystem::Ntfs),
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

/// Supported raw filesystem access
#[derive(Debug, PartialEq)]
enum RawFileSystem {
    /// Windows NTFS
    Ntfs,
}

/// Parse `SourcePath` if using raw access
fn parse_raw_source(remainder: &str, raw: RawFileSystem) -> AccessorResult<Option<SourcePath>> {
    if raw != RawFileSystem::Ntfs {
        return Err(AccessorError::RawAccessNotSupported {
            reason: format!("Unsupported raw filesystem: {raw:?}"),
        });
    }

    if remainder.is_empty() {
        return Err(AccessorError::location(
            remainder,
            "raw source requires a source such as ntfs:C:",
        ));
    }

    if is_absolute_host_path(remainder) {
        let drive = remainder
            .chars()
            .next()
            .ok_or_else(|| AccessorError::location(remainder, "ntfs path missing drive letter"))?;
        return Ok(Some(SourcePath::new(PathBuf::from(format!("{drive}:")))));
    }

    let drive = remainder
        .trim_end_matches(':')
        .chars()
        .next()
        .ok_or_else(|| AccessorError::location(remainder, "ntfs source requires a drive letter"))?;

    if !drive.is_ascii_alphabetic() {
        return Err(AccessorError::location(
            remainder,
            "ntfs source drive letter must be alphabetic",
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
        Scheme::Ntfs => {
            if remainder.is_empty() {
                return Err(AccessorError::location(
                    remainder,
                    "ntfs location requires a path",
                ));
            }
            if is_relative_host_path(remainder) {
                return Err(AccessorError::location(
                    remainder,
                    "ntfs locations require an absolute path",
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
        location::{
            loc::Location,
            path::{InnerPath, SourcePath},
            scheme::Scheme,
        },
    };
    use std::path::PathBuf;

    #[test]
    fn test_location() {
        let test = "zip:data.zip!./home/test.txt";
        let result = Location::parse(test).unwrap();
        assert_eq!(result.scheme, Scheme::Zip);
        assert_eq!(
            result.inner_path.display().replace('\\', "/"),
            "home/test.txt"
        );
        assert_eq!(result.source.unwrap().display(), "data.zip");
    }

    #[test]
    fn test_location_ntfs() {
        let test = "ntfs:C:\\home\\test.txt";
        let result = Location::parse(test).unwrap();
        assert_eq!(result.scheme, Scheme::Ntfs);
        assert_eq!(result.inner_path.display(), "C:\\home\\test.txt");
        assert_eq!(result.source.unwrap().display(), "C:");
    }

    #[test]
    fn test_location_tricky() {
        let test = "C:\\home\\ntfs:file!text.txt";
        let result = Location::parse(test).unwrap();
        assert_eq!(result.scheme, Scheme::Host);
        assert_eq!(result.inner_path.display(), "C:\\home\\ntfs:file!text.txt");
        assert!(result.source.is_none());
    }

    #[test]
    fn test_location_colon_in_path() {
        let test =
            "/home/dev/.local/share/gvfs-metadata/sftp:host=192.168.1.147,port=1739-9ea94643.log";
        let result = Location::parse(test).unwrap();
        assert_eq!(result.scheme, Scheme::Host);
        assert_eq!(
            result.inner_path.display(),
            "/home/dev/.local/share/gvfs-metadata/sftp:host=192.168.1.147,port=1739-9ea94643.log"
        );
        assert!(result.source.is_none());
    }

    #[test]
    fn test_location_exclamation_in_path() {
        let test = "/home/dev/.cache/vlc/art/artistalbum/singer/I like Music! /art";
        let result = Location::parse(test).unwrap();
        assert_eq!(result.scheme, Scheme::Host);
        assert_eq!(
            result.inner_path.display(),
            "/home/dev/.cache/vlc/art/artistalbum/singer/I like Music! /art"
        );
        assert!(result.source.is_none());
    }

    #[test]
    fn test_location_zip_exclamation_in_path() {
        let test = "zip:file.zip!./home/dev/.cache/vlc/art/artistalbum/singer/I like Music! /art";
        let result = Location::parse(test).unwrap();
        assert_eq!(result.scheme, Scheme::Zip);
        assert_eq!(
            result.inner_path.as_path(),
            "home/dev/.cache/vlc/art/artistalbum/singer/I like Music! /art"
        );
        assert_eq!(result.source.unwrap().display(), "file.zip");
    }

    #[test]
    fn test_location_exclamation_in_windows_mixed_path() {
        let test = "file.zip!\\home\\dev\\.cache/vlc/art/artistalbum/singer/I like Music! /art";
        let result = Location::parse(test).unwrap();
        assert_eq!(result.scheme, Scheme::Host);
        assert_eq!(
            result.inner_path.as_path(),
            "file.zip!\\home\\dev\\.cache/vlc/art/artistalbum/singer/I like Music! /art"
        );
        assert!(result.source.is_none());
    }

    #[test]
    fn test_location_not_a_zip() {
        let test = "host:zip:/file.zip/Amcache.hve";
        let result = Location::parse(test).unwrap();
        assert_eq!(result.scheme, Scheme::Host);
        assert_eq!(result.inner_path.display(), "zip:/file.zip/Amcache.hve");
        assert!(result.source.is_none());
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

    #[test]
    fn test_location_glob_windows_nested() {
        let (loc, pattern) = Location::split_glob_pattern(r"C:\Users\*\NTUSER*").unwrap();
        assert_eq!(loc.scheme, Scheme::Host);
        assert_eq!(loc.inner_path.display(), r"C:\Users");
        assert_eq!(pattern, r"*\NTUSER*");
    }

    #[test]
    fn test_location_glob_exact_path() {
        let (loc, pattern) = Location::split_glob_pattern("zip:test.zip!var/log/wtmp").unwrap();
        assert_eq!(
            loc.source.unwrap(),
            SourcePath::new(PathBuf::from("test.zip"))
        );
        assert_eq!(loc.scheme, Scheme::Zip);
        assert!(loc.inner_path.display().contains("var"));
        assert_eq!(pattern, "wtmp");
    }

    #[test]
    fn test_location_glob_exact_root_child() {
        let (loc, pattern) = Location::split_glob_pattern("/wtmp").unwrap();
        assert_eq!(loc.scheme, Scheme::Host);
        assert_eq!(loc.inner_path.display(), "/");
        assert_eq!(pattern, "wtmp");
    }

    #[test]
    fn test_location_glob_exact_backslash_root_child() {
        let (loc, pattern) = Location::split_glob_pattern(r"\wtmp").unwrap();
        assert_eq!(loc.scheme, Scheme::Host);
        assert_eq!(loc.inner_path.display(), r"\");
        assert_eq!(pattern, "wtmp");
    }

    #[test]
    fn test_parse_glob_pattern_exact_relative() {
        let (directory, pattern) = Location::parse_glob_pattern("wtmp").unwrap();
        assert!(directory.is_empty());
        assert_eq!(pattern, "wtmp");
    }

    #[test]
    fn test_location_glob_exact_windows_path() {
        let (loc, pattern) =
            Location::split_glob_pattern(r"C:\Windows\System32\config\SAM").unwrap();
        assert_eq!(loc.scheme, Scheme::Host);
        assert_eq!(loc.inner_path.display(), r"C:\Windows\System32\config");
        assert_eq!(pattern, "SAM");
    }

    #[test]
    fn test_glob_root() {
        let (loc, pattern) = Location::split_glob_pattern("/").unwrap();
        assert_eq!(pattern, "*");
        assert_eq!(loc.inner_path.display(), "/");
    }

    #[test]
    fn test_glob_zip_root() {
        let err = Location::split_glob_pattern("zip:test.zip!").unwrap_err();
        assert!(
            matches!(err, AccessorError::Location { input, reason } if input == "" && reason == "glob input cannot be empty")
        );
    }

    #[test]
    fn test_glob_zip_root_path() {
        let (loc, pattern) = Location::split_glob_pattern("zip:test.zip!/").unwrap();
        assert_eq!(pattern, "*");
        assert_eq!(loc.scheme, Scheme::Zip);
        assert_eq!(loc.inner_path, InnerPath::new(PathBuf::from("")));
    }

    #[test]
    fn test_glob_zip_folder() {
        let (loc, pattern) = Location::split_glob_pattern("zip:test.zip!var/log/").unwrap();
        assert_eq!(pattern, "*");
        assert!(loc.inner_path.display().contains("var"));
    }

    #[test]
    fn test_glob_folder() {
        let (loc, pattern) = Location::split_glob_pattern("C:\\Windows\\System32\\").unwrap();
        assert_eq!(pattern, "*");
        assert_eq!(loc.inner_path.display(), "C:\\Windows\\System32");
    }

    #[test]
    fn test_location_host_escape_with_exclamation() {
        let test = "host:/home/dev/.cache/vlc/art/artistalbum/singer/I like Music! /art";
        let result = Location::parse(test).unwrap();
        assert_eq!(result.scheme, Scheme::Host);
        assert_eq!(
            result.inner_path.display(),
            "/home/dev/.cache/vlc/art/artistalbum/singer/I like Music! /art"
        );
        assert!(result.source.is_none());
    }

    #[test]
    fn test_location_relative_zipfile_bang_is_host() {
        let test = "zipfile!foo.txt";
        let result = Location::parse(test).unwrap();
        assert_eq!(result.scheme, Scheme::Host);
        assert_eq!(result.inner_path.display(), "zipfile!foo.txt");
    }

    #[test]
    fn test_location_hostsomething_is_host() {
        let test = "hostsomething:foo";
        let result = Location::parse(test).unwrap();
        assert_eq!(result.scheme, Scheme::Host);
        assert_eq!(result.inner_path.display(), "hostsomething:foo");
    }

    #[test]
    fn test_glob_windows_bang_is_host() {
        let (loc, pattern) = Location::split_glob_pattern("C:\\home\\ntfs:file!text*").unwrap();
        assert_eq!(loc.scheme, Scheme::Host);
        assert_eq!(loc.inner_path.display(), "C:\\home");
        assert_eq!(pattern, "ntfs:file!text*");
    }

    #[test]
    fn test_ntfs_exclamation() {
        let test = "ntfs:C:\\Users\\file!name.txt";
        let result = Location::parse(test).unwrap();
        assert_eq!(result.scheme, Scheme::Ntfs);
        assert_eq!(result.inner_path.display(), "C:\\Users\\file!name.txt");
        assert_eq!(result.source.unwrap().display(), "C:");
    }

    #[test]
    fn test_parse_source_zip_exclamation_in_name() {
        let result = Location::parse_source("zip:/tmp/weird!name.zip").unwrap();
        assert_eq!(result.scheme, Scheme::Zip);
        assert_eq!(result.source.unwrap().display(), "/tmp/weird!name.zip");
        assert!(result.inner_path.is_empty());
    }

    #[test]
    fn test_parse_source_zip_location_is_literal_name() {
        let result = Location::parse_source("zip:file.zip!inner").unwrap();
        assert_eq!(result.source.unwrap().display(), "file.zip!inner");
        assert!(result.inner_path.is_empty());
    }
}
