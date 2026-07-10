//! Source lifecycle and dispatch helpers for the accessor
//!
//! This module sits between `Accessor` and the backend `Source` enum. The module helps with:
//!
//! - Maps parsed [`Location`] values and entry handles to a [`SourceId`]
//! - Opens backends once and stores them in [`SourceCache`]
//! - Dispatches read/list/glob/reader calls to the cached [`Source`]
//!
//! # One-shot vs two-step access
//!
//! **One-shot** (`read_file("zip:arc.zip!foo")`):
//! `build_source` → resolve `SourceId` → `ensure_source` → operate using `Location.inner_path`
//!
//! **Two-step** (`open("zip:arc.zip")` then `read_file_on(..., "foo")`):
//! `ensure_source` at open time; later calls use `parse_inner_path` + `read_*_on_source`
//!
//! # What the cache stores
//!
//! One [`Source`] per [`SourceId`] (example: `ZipSource` and zip index metadata)
//! Entry reads might still reopen the underlying archive; the cache tries to avoid rebuilding
//! source metadata on every call
//!
//! # Handle validation
//!
//! Handles from `globfs` / `read_dir` carry a locator (`Host`, `Zip`, `Ntfs`)
//! `read_*_handle_on` paths validate that the handle matches the open `SourceId`
//! before dispatching

use crate::accessor::{
    cache::SourceCache,
    config::AccessorConfig,
    entry::{
        handle::{DirEntry, DirHandle, FileHandle, GlobMatch},
        locator::{DirLocator, FileLocator, SourceId},
    },
    error::{AccessorError, AccessorResult},
    io::reader::AccessorReader,
    location::{location::Location, path::InnerPath, scheme::Scheme},
    source::{dispatch::Source, host::HostSource, ntfs::NtfsSource, zip::ZipSource},
};
use std::path::PathBuf;

/// Resolve a parsed location to a [`SourceId`] and ensure the backend is cached
///
/// Used by one-shot APIs (`read_file`, `read_dir`, `globfs`, `open_reader`)
/// Returns the `SourceId` so callers can read `location.inner_path` against it
pub(crate) fn build_source(
    location: &Location,
    config: &AccessorConfig,
    cache: &mut SourceCache,
) -> AccessorResult<SourceId> {
    let source_id = source_id_from_location(location)?;
    ensure_source(&source_id, config, cache)?;

    Ok(source_id)
}

/// Open a backend for `source_id` if it is not already in the cache
pub(crate) fn ensure_source(
    source_id: &SourceId,
    config: &AccessorConfig,
    cache: &mut SourceCache,
) -> AccessorResult<()> {
    if cache.get(source_id).is_some() {
        return Ok(());
    }

    let source = match source_id {
        SourceId::Host => Source::Host(HostSource::new(config)),
        SourceId::RawNtfs(drive) => Source::RawNtfs(NtfsSource::new(config, *drive)?),
        SourceId::Zip(path) => Source::Zip(ZipSource::new(config, path.clone())?),
    };

    cache.insert(source_id.clone(), source);
    Ok(())
}

/// Derive the cache key from a parsed [`Location`] scheme and source fields
///
/// - `Host` → [`SourceId::Host`]
/// - `Zip` → [`SourceId::Zip`]
/// - `Raw` → [`SourceId::RawNtfs`]
pub(crate) fn source_id_from_location(location: &Location) -> AccessorResult<SourceId> {
    match location.scheme {
        Scheme::Host => Ok(SourceId::Host),
        Scheme::Raw => {
            let source = location
                .source
                .as_ref()
                .ok_or_else(|| AccessorError::location("", "raw location missing drive source"))?;
            let drive =
                source.display().chars().next().ok_or_else(|| {
                    AccessorError::location("", "raw source missing drive letter")
                })?;
            Ok(SourceId::RawNtfs(drive))
        }
        Scheme::Zip => {
            let source = location
                .source
                .as_ref()
                .ok_or_else(|| AccessorError::location("", "zip location missing archive path"))?;
            Ok(SourceId::Zip(source.as_path().to_path_buf()))
        }
    }
}

/// Derive [`SourceId`] from a handle produced by listing or glob
///
/// Allows `read_file_handle` or `read_dir_handle` to open the correct cached source
/// without walking the filesystem again
pub(crate) fn source_id_from_file_locator(locator: &FileLocator) -> AccessorResult<SourceId> {
    match locator {
        FileLocator::Host { .. } => Ok(SourceId::Host),
        FileLocator::Ntfs { drive, .. } => Ok(SourceId::RawNtfs(*drive)),
        FileLocator::Zip { archive, .. } => Ok(SourceId::Zip(archive.clone())),
    }
}

/// Look up an opened source or return [`AccessorError::SourceNotOpen`]
///
/// Two-step callers must call `open` first
fn source_from_cache<'a>(
    cache: &'a SourceCache,
    source_id: &SourceId,
) -> AccessorResult<&'a Source> {
    cache
        .get(source_id)
        .ok_or_else(|| AccessorError::SourceNotOpen {
            source_id: source_id.display(),
        })
}

/// Read a file relative to an already-open source using an inner path
pub(crate) fn read_file_on_source(
    cache: &SourceCache,
    source_id: &SourceId,
    inner: &InnerPath,
) -> AccessorResult<Vec<u8>> {
    source_from_cache(cache, source_id)?.read_file(inner)
}

/// List a directory relative to an already-open source
pub(crate) fn read_dir_on_source(
    cache: &SourceCache,
    source_id: &SourceId,
    inner: &InnerPath,
) -> AccessorResult<Vec<DirEntry>> {
    source_from_cache(cache, source_id)?.read_dir(inner)
}

/// Glob immediate children under `directory` within an already-open source
pub(crate) fn glob_on_source(
    cache: &SourceCache,
    source_id: &SourceId,
    directory: &InnerPath,
    pattern: &str,
) -> AccessorResult<Vec<GlobMatch>> {
    source_from_cache(cache, source_id)?.globfs(directory, pattern)
}

/// Read a file using a handle, without reparsing a location string
pub(crate) fn read_file_handle_on_source(
    cache: &SourceCache,
    source_id: &SourceId,
    handle: &FileHandle,
) -> AccessorResult<Vec<u8>> {
    source_from_cache(cache, source_id)?.read_file_handle(handle)
}

/// Open a seekable reader using a handle, without reparsing a location string
pub(crate) fn open_reader_handle_on_source(
    cache: &SourceCache,
    source_id: &SourceId,
    handle: &FileHandle,
) -> AccessorResult<AccessorReader> {
    source_from_cache(cache, source_id)?.open_reader_handle(handle)
}

/// Open a seekable reader for an inner path on an already-open source
pub(crate) fn open_reader_on_source(
    cache: &SourceCache,
    source_id: &SourceId,
    inner: &InnerPath,
) -> AccessorResult<AccessorReader> {
    source_from_cache(cache, source_id)?.open_reader(inner)
}

/// Parse a relative inner path for two-step APIs (`read_file_on`, `globfs_on`, etc.)
///
/// Used after `open("host:")` or `open("zip:/path/archive.zip")`
/// Empty string means the source root (archive root for zip, `.` semantics for host)
///
/// Does not accept scheme prefixes or `!` container syntax — only paths within the source
pub(crate) fn parse_inner_path(inner: &str) -> AccessorResult<InnerPath> {
    let inner = inner.trim();
    if inner.is_empty() {
        return Ok(InnerPath::empty());
    }
    Ok(InnerPath::new(PathBuf::from(inner)))
}

/// Ensure a handle's locator matches the currently open source
///
/// Prevents reading a zip handle while `host:` is open, or a handle from one
/// archive while another zip source is open. Used by file handle two-step APIs only
pub(crate) fn validate_file_handle_for_source(
    source_id: &SourceId,
    locator: &FileLocator,
) -> AccessorResult<()> {
    match (source_id, locator) {
        (SourceId::Host, FileLocator::Host { .. }) => Ok(()),
        (
            SourceId::RawNtfs(drive),
            FileLocator::Ntfs {
                drive: handle_drive,
                ..
            },
        ) if drive == handle_drive => Ok(()),
        (
            SourceId::Zip(archive),
            FileLocator::Zip {
                archive: handle_archive,
                ..
            },
        ) if archive == handle_archive => Ok(()),
        _ => Err(AccessorError::invalid_handle(format!(
            "file handle does not belong to open source {}",
            source_id.display()
        ))),
    }
}

/// Return the `SourceId` from a `DirLocator`
pub(crate) fn source_id_from_dir_locator(locator: &DirLocator) -> AccessorResult<SourceId> {
    match locator {
        DirLocator::Host { .. } => Ok(SourceId::Host),
        DirLocator::Ntfs { drive, .. } => Ok(SourceId::RawNtfs(*drive)),
        DirLocator::Zip { archive, .. } => Ok(SourceId::Zip(archive.clone())),
    }
}

/// Read a directory using a handle, without reparsing a location string
pub(crate) fn read_dir_handle_on_source(
    cache: &SourceCache,
    source_id: &SourceId,
    handle: &DirHandle,
) -> AccessorResult<Vec<DirEntry>> {
    source_from_cache(cache, source_id)?.read_dir_handle(handle)
}

/// Ensure a handle's locator matches the currently open source
///
/// Prevents reading a zip handle while `host:` is open, or a handle from one
/// archive while another zip source is open. Used by directory handle two-step APIs only
pub(crate) fn validate_dir_handle_for_source(
    source_id: &SourceId,
    locator: &DirLocator,
) -> AccessorResult<()> {
    match (source_id, locator) {
        (SourceId::Host, DirLocator::Host { .. }) => Ok(()),
        (
            SourceId::RawNtfs(drive),
            DirLocator::Ntfs {
                drive: handle_drive,
                ..
            },
        ) if drive == handle_drive => Ok(()),
        (
            SourceId::Zip(archive),
            DirLocator::Zip {
                archive: handle_archive,
                ..
            },
        ) if archive == handle_archive => Ok(()),
        _ => Err(AccessorError::invalid_handle(format!(
            "directory handle does not belong to open source {}",
            source_id.display()
        ))),
    }
}
