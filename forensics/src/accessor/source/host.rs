use crate::accessor::{
    config::AccessorConfig,
    entry::{
        handle::{DirEntry, DirHandle, FileHandle, GlobMatch},
        locator::SourceId,
    },
    error::AccessorResult,
    filesystem::host::HostFs,
    io::reader::AccessorReader,
    location::path::InnerPath,
    source::backend::SourceBackend,
};

/// Source struct for a live OS
pub(crate) struct HostSource {
    max_read_size: Option<u64>,
}

impl HostSource {
    /// Create a `HostSource` structure
    pub(crate) fn new(config: &AccessorConfig) -> Self {
        Self {
            max_read_size: config.max_read_size,
        }
    }
}

impl SourceBackend for HostSource {
    fn source_id(&self) -> SourceId {
        SourceId::Host
    }

    fn read_file(&self, inner: &InnerPath) -> AccessorResult<Vec<u8>> {
        HostFs::read_file(inner, self.max_read_size)
    }

    fn read_dir(&self, inner: &InnerPath) -> AccessorResult<Vec<DirEntry>> {
        HostFs::read_dir(inner)
    }

    fn globfs(&self, directory: &InnerPath, pattern: &str) -> AccessorResult<Vec<GlobMatch>> {
        HostFs::globfs(directory, pattern)
    }

    fn read_file_handle(&self, handle: &FileHandle) -> AccessorResult<Vec<u8>> {
        HostFs::read_handle(handle, self.max_read_size)
    }

    fn open_reader(&self, inner: &InnerPath) -> AccessorResult<AccessorReader> {
        HostFs::reader(inner)
    }

    fn open_reader_handle(&self, handle: &FileHandle) -> AccessorResult<AccessorReader> {
        HostFs::reader_handle(handle)
    }

    fn read_dir_handle(&self, handle: &DirHandle) -> AccessorResult<Vec<DirEntry>> {
        HostFs::read_dir_handle(handle)
    }
}

#[cfg(test)]
mod tests {
    use crate::accessor::config::AccessorConfig;
    use crate::accessor::entry::handle::FileHandle;
    use crate::accessor::entry::locator::{FileLocator, NtfsEntryRef};
    use crate::accessor::error::AccessorError;
    use crate::accessor::location::path::InnerPath;
    use crate::accessor::source::backend::SourceBackend;
    use crate::accessor::source::host::HostSource;
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;

    fn setup(test_name: &str) -> PathBuf {
        let dir = PathBuf::from("./tmp/host_source").join(test_name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_file(dir: &PathBuf, name: &str, contents: &[u8]) -> PathBuf {
        let path = dir.join(name);
        File::create(&path).unwrap().write_all(contents).unwrap();
        path
    }

    fn config_with_limit(limit: u64) -> AccessorConfig {
        AccessorConfig {
            max_read_size: Some(limit),
            ..AccessorConfig::default()
        }
    }

    #[test]
    fn test_source_id_is_host() {
        let source = HostSource::new(&AccessorConfig::default());
        assert_eq!(
            source.source_id(),
            crate::accessor::entry::locator::SourceId::Host
        );
    }

    #[test]
    fn test_read_file_enforces_config_max_read_size() {
        let dir = setup("test_read_file_enforces_config_max_read_size");
        write_file(&dir, "big.bin", &[0u8; 32]);
        let source = HostSource::new(&config_with_limit(16));
        let err = source
            .read_file(&InnerPath::new(dir.join("big.bin")))
            .unwrap_err();
        assert!(matches!(
            err,
            AccessorError::FileTooLarge {
                size: 32,
                limit: 16
            }
        ));
    }

    #[test]
    fn test_read_file_handle_reads_host_locator() {
        let dir = setup("test_read_file_handle_reads_host_locator");
        let path = write_file(&dir, "payload.txt", b"payload");
        let source = HostSource::new(&AccessorConfig::default());
        let handle = FileHandle::host(&path);
        let bytes = source.read_file_handle(&handle).unwrap();
        assert_eq!(bytes, b"payload");
    }

    #[test]
    fn test_read_file_handle_rejects_zip_locator() {
        let source = HostSource::new(&AccessorConfig::default());
        let handle = FileHandle::new(FileLocator::Zip {
            archive: PathBuf::from("./tmp/host_source/archive.zip"),
            entry_index: 0,
            entry: String::from("inner.txt"),
        });
        let err = source.read_file_handle(&handle).unwrap_err();
        assert!(matches!(err, AccessorError::InvalidHandle { .. }));
    }

    #[test]
    fn test_open_reader_skips_max_read_size() {
        let dir = setup("test_open_reader_skips_max_read_size");
        write_file(&dir, "big.bin", &[1, 2, 3, 4, 5, 6, 7, 8]);
        let source = HostSource::new(&config_with_limit(4));
        let mut reader = source
            .open_reader(&InnerPath::new(dir.join("big.bin")))
            .unwrap();

        let mut buf = Vec::new();
        assert_eq!(reader.read_to_end(&mut buf).unwrap(), 8);
    }

    #[test]
    fn test_open_reader_handle_rejects_ntfs_locator() {
        let source = HostSource::new(&AccessorConfig::default());
        let handle = FileHandle::new(FileLocator::Ntfs {
            drive: 'C',
            file_ref: NtfsEntryRef {
                file_record_number: 5,
                sequence_number: 1,
            },
            display_path: String::from(r"C:\$MFT"),
        });
        let err = source.open_reader_handle(&handle).unwrap_err();
        assert!(matches!(err, AccessorError::InvalidHandle { .. }));
    }

    #[test]
    fn test_read_at_via_open_reader() {
        let dir = setup("test_read_at_via_open_reader");
        write_file(&dir, "data.bin", b"abcdefgh");
        let source = HostSource::new(&AccessorConfig::default());
        let mut reader = source
            .open_reader(&InnerPath::new(dir.join("data.bin")))
            .unwrap();
        assert_eq!(reader.read_bytes(2, 3).unwrap(), b"cde");
    }
}
