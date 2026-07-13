use tracing::info;

use crate::accessor::{
    cache::SourceCache,
    config::AccessorConfig,
    entry::handle::{DirEntry, DirHandle, FileHandle, GlobMatch},
    error::AccessorResult,
    io::reader::AccessorReader,
    location::loc::Location,
    source::{
        factory::{
            build_source, ensure_source, glob_on_source, open_reader_handle_on_source,
            open_reader_on_source, parse_inner_path, read_dir_handle_on_source, read_dir_on_source,
            read_file_handle_on_source, read_file_on_source, source_id_from_dir_locator,
            source_id_from_file_locator, validate_dir_handle_for_source,
            validate_file_handle_for_source,
        },
        handle::SourceHandle,
    },
};

/// An access implementation that lets us read files from provided input
///
/// This accessor supports reading from a variety of sources
///
/// Input is parsed into [`Location`] structure which is composed of (scheme, optional source path, and inner path)
///
/// Example: `zip:test.zip!/home/test.txt`
///
/// `Scheme` - `zip`
/// `Source path` - `test.zip`
/// `Inner path` - `/home/test.txt`
///
/// Example `C:\\Users\\test.txt`
///
/// `Scheme` - `host`. Live system
/// `Source path` - None
/// `Inner path` - `C:\\Users\\test.txt`
///
/// Supported schemes are: `zip`, `ntfs`, `host`
pub(crate) struct Accessor {
    /// The configuration for the `Accessor`
    config: AccessorConfig,
    /// Caches opened `Source` metadata (Example: `ZipSource` and zip index)
    /// so repeated reads under the same source avoid re-parsing `Source` metadata.
    cache: SourceCache,
}

impl Accessor {
    /// Create a `Accessor` structure
    pub(crate) fn new(config: AccessorConfig) -> Self {
        Self {
            config,
            cache: SourceCache::new(),
        }
    }

    /// Initialize `Accessor` with default `AccessorConfig`
    pub(crate) fn with_defaults() -> Self {
        Self::new(AccessorConfig::default())
    }

    /// Read an entire file into memory
    pub(crate) fn read_file(&mut self, location: &str) -> AccessorResult<Vec<u8>> {
        let loc = Location::parse(location)?;
        let source_id = build_source(&loc, &self.config, &mut self.cache)?;
        info!(
            "Reading file {location} using source '{}'. Scheme: {}",
            source_id.display(),
            loc.scheme.as_str()
        );

        read_file_on_source(&self.cache, &source_id, &loc.inner_path)
    }

    /// List a directory
    pub(crate) fn read_dir(&mut self, location: &str) -> AccessorResult<Vec<DirEntry>> {
        let loc = Location::parse(location)?;
        let source_id = build_source(&loc, &self.config, &mut self.cache)?;
        info!(
            "Reading directory {location} using source '{}'. Scheme: {}",
            source_id.display(),
            loc.scheme.as_str()
        );

        read_dir_on_source(&self.cache, &source_id, &loc.inner_path)
    }

    /// Read bytes for a `FileHandle` from glob/listing
    pub(crate) fn read_file_handle(&mut self, handle: &FileHandle) -> AccessorResult<Vec<u8>> {
        let source_id = source_id_from_file_locator(&handle.locator)?;
        ensure_source(&source_id, &self.config, &mut self.cache)?;
        info!(
            "Reading file {} via handle using source '{}'",
            handle.display_path(),
            source_id.display(),
        );

        read_file_handle_on_source(&self.cache, &source_id, handle)
    }

    /// List a directory using a `DirHandle` from glob or directory listing
    pub(crate) fn read_dir_handle(&mut self, handle: &DirHandle) -> AccessorResult<Vec<DirEntry>> {
        let source_id = source_id_from_dir_locator(&handle.locator)?;
        ensure_source(&source_id, &self.config, &mut self.cache)?;
        info!(
            "Reading directory {} via handle using source '{}'",
            handle.display_path(),
            source_id.display(),
        );

        read_dir_handle_on_source(&self.cache, &source_id, handle)
    }

    /// Glob files and directories in a directory
    ///
    /// Example: `C:\Users\*` or `/var/log/*.log`
    pub(crate) fn globfs(&mut self, input: &str) -> AccessorResult<Vec<GlobMatch>> {
        let (loc, pattern) = Location::split_glob_pattern(input)?;
        let source_id = build_source(&loc, &self.config, &mut self.cache)?;
        info!(
            "Glob {input} using source '{}'. Scheme: {}",
            source_id.display(),
            loc.scheme.as_str(),
        );

        glob_on_source(&self.cache, &source_id, &loc.inner_path, &pattern)
    }

    /// Open a seekable reader for a parsed location
    ///
    /// Useful for hard-coded paths like `ntfs:C:\$MFT`.
    pub(crate) fn open_reader(&mut self, location: &str) -> AccessorResult<AccessorReader> {
        let loc = Location::parse(location)?;
        let source_id = build_source(&loc, &self.config, &mut self.cache)?;
        info!(
            "Reader for {location} using source '{}'. Scheme: {}",
            source_id.display(),
            loc.scheme.as_str(),
        );

        open_reader_on_source(&self.cache, &source_id, &loc.inner_path)
    }

    /// Open a seekable reader for a `FileHandle`
    pub(crate) fn open_reader_handle(
        &mut self,
        handle: &FileHandle,
    ) -> AccessorResult<AccessorReader> {
        let source_id = source_id_from_file_locator(&handle.locator)?;
        ensure_source(&source_id, &self.config, &mut self.cache)?;
        info!(
            "Reader for {} file handle using source '{}'",
            handle.display_path(),
            source_id.display(),
        );

        open_reader_handle_on_source(&self.cache, &source_id, handle)
    }

    /// Open a source for repeated reads
    ///
    /// Examples: `host:`, `ntfs:C:`, `zip:/path/archive.zip`
    pub(crate) fn open_source(&mut self, source: &str) -> AccessorResult<SourceHandle> {
        let loc = Location::parse_source(source)?;
        let source_id = build_source(&loc, &self.config, &mut self.cache)?;
        info!(
            "Opened input {source} using source '{}'. Scheme: {}",
            source_id.display(),
            loc.scheme.as_str(),
        );

        Ok(SourceHandle::new(source_id))
    }

    /// Read a file relative to an opened source
    pub(crate) fn source_read_file(
        &self,
        source: &SourceHandle,
        inner: &str,
    ) -> AccessorResult<Vec<u8>> {
        info!("Reading file {inner} with source {}", source.display());
        read_file_on_source(&self.cache, source.id(), &parse_inner_path(inner)?)
    }

    /// List a directory relative to an opened source
    pub(crate) fn source_read_dir(
        &self,
        source: &SourceHandle,
        inner: &str,
    ) -> AccessorResult<Vec<DirEntry>> {
        info!("Reading directory {inner} with source {}", source.display());
        read_dir_on_source(&self.cache, source.id(), &parse_inner_path(inner)?)
    }

    /// List a directory handle relative to an opened source
    pub(crate) fn source_read_dir_handle(
        &self,
        source: &SourceHandle,
        handle: &DirHandle,
    ) -> AccessorResult<Vec<DirEntry>> {
        info!(
            "Reading directory handle {} with source {}",
            handle.display_path(),
            source.display()
        );

        validate_dir_handle_for_source(source.id(), &handle.locator)?;
        read_dir_handle_on_source(&self.cache, source.id(), handle)
    }

    /// Read a file handle relative to an opened source
    pub(crate) fn source_read_file_handle(
        &self,
        source: &SourceHandle,
        handle: &FileHandle,
    ) -> AccessorResult<Vec<u8>> {
        info!(
            "Reading file handle {} with source {}",
            handle.display_path(),
            source.display()
        );

        validate_file_handle_for_source(source.id(), &handle.locator)?;
        read_file_handle_on_source(&self.cache, source.id(), handle)
    }

    /// Glob relative to an opened source directory
    pub(crate) fn source_globfs(
        &self,
        source: &SourceHandle,
        input: &str,
    ) -> AccessorResult<Vec<GlobMatch>> {
        info!(
            "Globbing with pattern {input} with source {}",
            source.display()
        );
        let (directory, pattern) = Location::parse_glob_pattern(input)?;

        glob_on_source(
            &self.cache,
            source.id(),
            &parse_inner_path(&directory)?,
            &pattern,
        )
    }

    /// Open a reader relative to an opened source
    pub(crate) fn source_open_reader(
        &self,
        source: &SourceHandle,
        inner: &str,
    ) -> AccessorResult<AccessorReader> {
        info!("Reader for file {inner} with source {}", source.display());
        open_reader_on_source(&self.cache, source.id(), &parse_inner_path(inner)?)
    }

    /// Open a reader for a file handle relative to an opened source
    pub(crate) fn source_open_reader_handle(
        &self,
        source: &SourceHandle,
        handle: &FileHandle,
    ) -> AccessorResult<AccessorReader> {
        info!(
            "Reader for file handle {} with source {}",
            handle.display_path(),
            source.display()
        );

        validate_file_handle_for_source(source.id(), &handle.locator)?;
        open_reader_handle_on_source(&self.cache, source.id(), handle)
    }
}

#[cfg(test)]
mod tests {
    use crate::accessor::{access::Accessor, entry::handle::EntryKind};
    use std::{
        fs::{self, File},
        io::{Read, Write},
        path::PathBuf,
    };

    fn setup(test_name: &str) -> PathBuf {
        let dir = PathBuf::from("./tmp/accessor_host").join(test_name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_file(dir: &PathBuf, name: &str, contents: &[u8]) {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        File::create(path).unwrap().write_all(contents).unwrap();
    }

    #[test]
    fn test_host_accessor() {
        let dir = setup("test_host_accessor");
        write_file(&dir, "test.txt", b"my first test");

        let mut access = Accessor::with_defaults();
        let bytes = access
            .read_file("./tmp/accessor_host/test_host_accessor/test.txt")
            .unwrap();
        assert_eq!(bytes, b"my first test");
    }

    #[test]
    fn test_host_accessor_read_glob() {
        let mut access = Accessor::with_defaults();
        let results = access.globfs(".*").unwrap();

        for entry in results {
            if entry.meta.kind != EntryKind::Directory {
                continue;
            }
            let sub_dir = access
                .read_dir_handle(entry.handle.as_directory().unwrap())
                .unwrap();

            assert!(!sub_dir.is_empty())
        }
    }

    #[test]
    fn test_host_accessor_source_reader_handle() {
        let mut access = Accessor::with_defaults();
        let host_source = access.open_source("host:").unwrap();
        let results = access.source_globfs(&host_source, ".*").unwrap();

        for entry in results {
            if entry.meta.kind != EntryKind::File {
                continue;
            }
            let mut file_reader = access
                .source_open_reader_handle(&host_source, &entry.handle.as_file().unwrap())
                .unwrap();

            let mut buf = [0u8; 10];
            let bytes = file_reader.read(&mut buf).unwrap();
            assert_eq!(bytes, 10);
        }
    }

    #[test]
    fn test_host_accessor_source_read_file_handle() {
        let mut access = Accessor::with_defaults();
        let host_source = access.open_source("host:").unwrap();
        let results = access.source_globfs(&host_source, ".*").unwrap();

        for entry in results {
            if entry.meta.kind != EntryKind::File {
                continue;
            }
            let bytes = access
                .source_read_file_handle(&host_source, &entry.handle.as_file().unwrap())
                .unwrap();

            assert!(!bytes.is_empty());
        }
    }

    #[test]
    fn test_read_dir() {
        let test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut access = Accessor::with_defaults();
        let results = access
            .read_dir(&test_location.display().to_string())
            .unwrap();
        assert!(!results.is_empty())
    }

    #[test]
    fn test_host_accessor_read_zip() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/archives/document.odt");

        let mut access = Accessor::with_defaults();
        let results = access
            .globfs(&format!("zip:{}!*", test_location.display().to_string()))
            .unwrap();
        assert_eq!(results.len(), 9);

        for entry in results {
            if entry.meta.kind != EntryKind::File {
                continue;
            }

            if entry
                .meta
                .display_path
                .contains("document.odt!manifest.rdf")
            {
                let bytes = access
                    .read_file_handle(&entry.handle.as_file().unwrap())
                    .unwrap();
                assert_eq!(bytes.len(), 899);
            }
        }
    }

    #[test]
    fn test_host_accessor_zip_reader() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/archives/document.odt");

        let mut access = Accessor::with_defaults();
        let mut file_reader = access
            .open_reader(&format!("host:{}", test_location.display().to_string()))
            .unwrap();
        let mut buf = [0u8; 10];

        let bytes = file_reader.read(&mut buf).unwrap();
        assert_eq!(bytes, 10);
    }

    #[test]
    #[cfg(windows)]
    fn test_raw_accessor_read_zip() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/archives/document.odt");

        let mut access = Accessor::with_defaults();
        let source = access
            .open_source(&format!("ntfs:{}", test_location.display().to_string()))
            .unwrap();

        let bytes = access
            .source_read_file(&source, &test_location.display().to_string())
            .unwrap();
        assert_eq!(bytes.len(), 10493);
    }

    #[test]
    #[cfg(windows)]
    fn test_raw_accessor_live() {
        let mut access = Accessor::with_defaults();
        let source = access.open_source(&"ntfs:C").unwrap();

        let files = access.source_globfs(&source, "*").unwrap();
        assert!(!files.is_empty());

        for file in files {
            if file.meta.display_path == "C:\\$MFT" {
                assert!(file.meta.size > 100);
            }
        }
    }

    #[test]
    #[cfg(windows)]
    fn test_raw_accessor_live_read_dir_handle() {
        let mut access = Accessor::with_defaults();
        let source = access.open_source(&"ntfs:C").unwrap();

        let files = access.source_globfs(&source, "*").unwrap();
        assert!(!files.is_empty());

        for file in files {
            if file.meta.kind != EntryKind::Directory {
                continue;
            }

            let results = access
                .source_read_dir_handle(&source, &file.handle.as_directory().unwrap())
                .unwrap();

            let path_results = access
                .source_read_dir(&source, &file.meta.display_path)
                .unwrap();
            assert_eq!(path_results, results);
            if file.meta.display_path == "C:\\Users" {
                assert!(!results.is_empty());
                assert!(!path_results.is_empty());
            }
        }
    }

    #[test]
    #[cfg(windows)]
    fn test_raw_accessor_mft_reader() {
        use std::io::Read;

        let mut access = Accessor::with_defaults();
        let source = access.open_source(&"ntfs:C").unwrap();

        let mut reader = access.source_open_reader(&source, "$MFT").unwrap();
        let mut buf = [0u8; 1024];
        let bytes = reader.read(&mut buf).unwrap();

        assert_eq!(bytes, 1024);
        assert!(buf.starts_with(b"FILE0"));
    }

    #[test]
    #[cfg(windows)]
    fn test_windows_globfs() {
        let mut access = Accessor::with_defaults();
        let entries = access.globfs("C:\\Users\\*\\NTUSER*").unwrap();
        assert!(!entries.is_empty());
    }
}
