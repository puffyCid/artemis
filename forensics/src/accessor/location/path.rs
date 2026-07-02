use crate::accessor::error::{AccessorError, AccessorResult};
use std::path::{Component, Path, PathBuf};

/// Path to the file or directory to acccess
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct InnerPath(PathBuf);

impl InnerPath {
    /// Create a `InnerPath` structure
    pub(crate) fn new(path: PathBuf) -> Self {
        Self(path)
    }

    /// Initialize an empty `InnerPath`
    pub(crate) fn empty() -> Self {
        Self(PathBuf::new())
    }

    /// Return `InnerPath` as `Path`
    pub(crate) fn as_path(&self) -> &Path {
        &self.0
    }

    /// Return `InnerPath` as String
    pub(crate) fn display(&self) -> String {
        self.0.display().to_string()
    }

    /// Check if `InnerPath` is empty
    pub(crate) fn is_empty(&self) -> bool {
        self.0.as_os_str().is_empty()
    }

    /// Normalize zip-style paths to forward slashes without a leading `./`
    pub(crate) fn normalize_container_path(value: &str) -> AccessorResult<Self> {
        if value.is_empty() {
            return Ok(Self::empty());
        }

        let mut normalized = PathBuf::new();
        for component in Path::new(value).components() {
            match component {
                Component::Prefix(prefix) => {
                    let prefix_value = prefix.as_os_str().to_string_lossy();
                    return Err(AccessorError::location(
                        value,
                        format!("invalid container path prefix: {prefix_value}"),
                    ));
                }
                Component::RootDir => {
                    return Err(AccessorError::location(
                        value,
                        "container paths must be relative to the archive root",
                    ));
                }
                Component::CurDir => {}
                Component::ParentDir => {
                    if !normalized.pop() {
                        return Err(AccessorError::location(
                            value,
                            "container path escapes archive root",
                        ));
                    }
                }
                Component::Normal(part) => normalized.push(part),
            }
        }
        Ok(Self(normalized))
    }
}

/// Path to the optional source container of our data
///
/// Example: A zip file or disk image
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SourcePath(PathBuf);

impl SourcePath {
    /// Create a `SourcePath` read data
    pub(crate) fn new(path: PathBuf) -> Self {
        Self(path)
    }

    /// Return `SourcePath` as `Path`
    pub(crate) fn as_path(&self) -> &Path {
        &self.0
    }

    /// Return `SourcePath` as `PathBuf`
    pub(crate) fn into_path_buf(self) -> PathBuf {
        self.0
    }

    /// Return `SourcePath` as String
    pub(crate) fn display(&self) -> String {
        self.0.display().to_string()
    }
}

/// Determine if our provided input is an abosulate path to the data
///
/// Required for raw access
pub(crate) fn is_absolute_host_path(input: &str) -> bool {
    if input.is_empty() {
        return false;
    }
    let path = Path::new(input);
    if path.has_root() {
        return true;
    }

    let bytes = input.as_bytes();
    bytes.len() >= 2
        && bytes[1] == b':'
        && bytes[0].is_ascii_alphabetic()
        && input.get(2..).is_none_or(|remaining| {
            remaining.is_empty() || remaining.starts_with('\\') || remaining.starts_with('/')
        })
}

/// Determine if our provided input is a relative path to the data
pub(crate) fn is_relative_host_path(input: &str) -> bool {
    if input.is_empty() {
        return false;
    }

    let path = Path::new(input);
    if path.has_root() || is_absolute_host_path(input) {
        return false;
    }

    for component in path.components() {
        if matches!(component, Component::Prefix(_)) {
            return false;
        }
    }

    true
}

/// Determine if our proivded input can be represented as a path on a live system
pub(crate) fn is_host_path(input: &str) -> bool {
    is_absolute_host_path(input) || is_relative_host_path(input)
}
