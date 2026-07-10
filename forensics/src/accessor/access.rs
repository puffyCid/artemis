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
/// This accessor supports reading from of variety of sources depending on the `Location Scheme` type. Example: `zip:test.zip!home/test.txt`, `/home/test.txt`, `raw:C::\Users\test.txt`
///
/// Example: live system, raw disk, zip files
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
        read_file_on_source(&self.cache, &source_id, &loc.inner_path)
    }

    /// List a directory
    pub(crate) fn read_dir(&mut self, location: &str) -> AccessorResult<Vec<DirEntry>> {
        let loc = Location::parse(location)?;
        let source_id = build_source(&loc, &self.config, &mut self.cache)?;
        read_dir_on_source(&self.cache, &source_id, &loc.inner_path)
    }

    /// Read bytes for a `FileHandle` from glob/listing
    pub(crate) fn read_file_handle(&mut self, handle: &FileHandle) -> AccessorResult<Vec<u8>> {
        let source_id = source_id_from_file_locator(&handle.locator)?;
        ensure_source(&source_id, &self.config, &mut self.cache)?;
        read_file_handle_on_source(&self.cache, &source_id, handle)
    }

    /// List a directory using a `DirHandle` from glob or directory listing
    pub(crate) fn read_dir_handle(&mut self, handle: &DirHandle) -> AccessorResult<Vec<DirEntry>> {
        let source_id = source_id_from_dir_locator(&handle.locator)?;
        ensure_source(&source_id, &self.config, &mut self.cache)?;
        read_dir_handle_on_source(&self.cache, &source_id, handle)
    }

    /// Glob files and directories in a directory
    ///
    /// Example: `C:\Users\*` or `/var/log/*.log`
    pub(crate) fn globfs(&mut self, input: &str) -> AccessorResult<Vec<GlobMatch>> {
        let (loc, pattern) = Location::split_glob_pattern(input)?;
        let source_id = build_source(&loc, &self.config, &mut self.cache)?;
        glob_on_source(&self.cache, &source_id, &loc.inner_path, &pattern)
    }

    /// Open a seekable reader for a parsed location
    ///
    /// Useful for hard-coded paths like `raw:C:\$MFT`.
    pub(crate) fn open_reader(&mut self, location: &str) -> AccessorResult<AccessorReader> {
        let loc = Location::parse(location)?;
        let source_id = build_source(&loc, &self.config, &mut self.cache)?;
        open_reader_on_source(&self.cache, &source_id, &loc.inner_path)
    }

    /// Open a seekable reader for a `FileHandle`
    pub(crate) fn open_reader_handle(
        &mut self,
        handle: &FileHandle,
    ) -> AccessorResult<AccessorReader> {
        let source_id = source_id_from_file_locator(&handle.locator)?;
        ensure_source(&source_id, &self.config, &mut self.cache)?;
        open_reader_handle_on_source(&self.cache, &source_id, handle)
    }

    /// Open a source for repeated reads
    ///
    /// Examples: `host:`, `raw:C:`, `zip:/path/archive.zip`
    pub(crate) fn open(&mut self, source: &str) -> AccessorResult<SourceHandle> {
        let loc = Location::parse_source(source)?;
        let source_id = build_source(&loc, &self.config, &mut self.cache)?;
        Ok(SourceHandle::new(source_id))
    }

    /// Read a file relative to an opened source
    pub(crate) fn read_file_on(
        &self,
        source: &SourceHandle,
        inner: &str,
    ) -> AccessorResult<Vec<u8>> {
        read_file_on_source(&self.cache, source.id(), &parse_inner_path(inner)?)
    }

    /// List a directory relative to an opened source
    pub(crate) fn read_dir_on(
        &self,
        source: &SourceHandle,
        inner: &str,
    ) -> AccessorResult<Vec<DirEntry>> {
        read_dir_on_source(&self.cache, source.id(), &parse_inner_path(inner)?)
    }

    /// List a directory handle relative to an opened source
    pub(crate) fn read_dir_handle_on(
        &self,
        source: &SourceHandle,
        handle: &DirHandle,
    ) -> AccessorResult<Vec<DirEntry>> {
        validate_dir_handle_for_source(source.id(), &handle.locator)?;
        read_dir_handle_on_source(&self.cache, source.id(), handle)
    }

    /// Read a file handle relative to an opened source
    pub(crate) fn read_file_handle_on(
        &self,
        source: &SourceHandle,
        handle: &FileHandle,
    ) -> AccessorResult<Vec<u8>> {
        validate_file_handle_for_source(source.id(), &handle.locator)?;
        read_file_handle_on_source(&self.cache, source.id(), handle)
    }

    /// Glob relative to an opened source directory
    pub(crate) fn globfs_on(
        &self,
        source: &SourceHandle,
        dir: &str,
        pattern: &str,
    ) -> AccessorResult<Vec<GlobMatch>> {
        glob_on_source(&self.cache, source.id(), &parse_inner_path(dir)?, pattern)
    }

    /// Open a reader relative to an opened source
    pub(crate) fn open_reader_on(
        &self,
        source: &SourceHandle,
        inner: &str,
    ) -> AccessorResult<AccessorReader> {
        open_reader_on_source(&self.cache, source.id(), &parse_inner_path(inner)?)
    }

    /// Open a reader for a file handle relative to an opened source
    pub(crate) fn open_reader_handle_on(
        &self,
        source: &SourceHandle,
        handle: &FileHandle,
    ) -> AccessorResult<AccessorReader> {
        validate_file_handle_for_source(source.id(), &handle.locator)?;
        open_reader_handle_on_source(&self.cache, source.id(), handle)
    }
}

#[cfg(test)]
mod tests {
    use crate::accessor::{access::Accessor, entry::handle::EntryKind};
    use std::{
        fs::{self, File},
        io::Write,
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
            if !sub_dir.is_empty() {
                break;
            }
        }
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
}
