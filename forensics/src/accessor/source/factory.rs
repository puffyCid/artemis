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
    source::{dispatch::Source, host::HostSource, zip::ZipSource},
};
use std::path::PathBuf;

pub(crate) fn build_source(
    location: &Location,
    config: &AccessorConfig,
    cache: &mut SourceCache,
) -> AccessorResult<SourceId> {
    let source_id = source_id_from_location(location)?;
    ensure_source(&source_id, config, cache)?;

    Ok(source_id)
}

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
        SourceId::RawNtfs(drive) => {
            return Err(AccessorError::Filesystem {
                reason: format!("raw:{drive}: source is not implemented yet"),
            });
        }
        SourceId::Zip(path) => Source::Zip(ZipSource::new(config, path.clone())?),
    };

    cache.insert(source_id.clone(), source);
    Ok(())
}

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

pub(crate) fn source_id_from_file_locator(locator: &FileLocator) -> AccessorResult<SourceId> {
    match locator {
        FileLocator::Host { .. } => Ok(SourceId::Host),
        FileLocator::Ntfs { drive, .. } => Ok(SourceId::RawNtfs(*drive)),
        FileLocator::Zip { archive, .. } => Ok(SourceId::Zip(archive.clone())),
    }
}

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

pub(crate) fn read_file_on_source(
    cache: &SourceCache,
    source_id: &SourceId,
    inner: &InnerPath,
) -> AccessorResult<Vec<u8>> {
    source_from_cache(cache, source_id)?.read_file(inner)
}

pub(crate) fn read_dir_on_source(
    cache: &SourceCache,
    source_id: &SourceId,
    inner: &InnerPath,
) -> AccessorResult<Vec<DirEntry>> {
    source_from_cache(cache, source_id)?.read_dir(inner)
}
pub(crate) fn glob_on_source(
    cache: &SourceCache,
    source_id: &SourceId,
    dir: &InnerPath,
    pattern: &str,
) -> AccessorResult<Vec<GlobMatch>> {
    source_from_cache(cache, source_id)?.globfs(dir, pattern)
}
pub(crate) fn read_file_handle_on_source(
    cache: &SourceCache,
    source_id: &SourceId,
    handle: &FileHandle,
) -> AccessorResult<Vec<u8>> {
    source_from_cache(cache, source_id)?.read_file_handle(handle)
}
pub(crate) fn open_reader_handle_on_source(
    cache: &SourceCache,
    source_id: &SourceId,
    handle: &FileHandle,
) -> AccessorResult<AccessorReader> {
    source_from_cache(cache, source_id)?.open_reader_handle(handle)
}

pub(crate) fn open_reader_on_source(
    cache: &SourceCache,
    source_id: &SourceId,
    inner: &InnerPath,
) -> AccessorResult<AccessorReader> {
    source_from_cache(cache, source_id)?.open_reader(inner)
}

pub(crate) fn parse_inner_path(inner: &str) -> AccessorResult<InnerPath> {
    let inner = inner.trim();
    if inner.is_empty() {
        return Ok(InnerPath::empty());
    }
    Ok(InnerPath::new(PathBuf::from(inner)))
}

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

pub(crate) fn source_id_from_dir_locator(locator: &DirLocator) -> AccessorResult<SourceId> {
    match locator {
        DirLocator::Host { .. } => Ok(SourceId::Host),
        DirLocator::Ntfs { drive, .. } => Ok(SourceId::RawNtfs(*drive)),
        DirLocator::Zip { archive, .. } => Ok(SourceId::Zip(archive.clone())),
    }
}

pub(crate) fn read_dir_handle_on_source(
    cache: &SourceCache,
    source_id: &SourceId,
    handle: &DirHandle,
) -> AccessorResult<Vec<DirEntry>> {
    source_from_cache(cache, source_id)?.read_dir_handle(handle)
}

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
