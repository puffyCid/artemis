use crate::accessor::{
    config::AccessorConfig,
    entry::{
        handle::{DirEntry, DirHandle, FileHandle, GlobMatch},
        locator::SourceId,
    },
    error::AccessorResult,
    filesystem::zip::{ZipFs, ZipIndex},
    io::reader::AccessorReader,
    location::path::InnerPath,
    source::backend::SourceBackend,
};
use std::path::PathBuf;

/// Use a zip file as our source for data access
pub(crate) struct ZipSource {
    /// Path to the zip file
    archive_path: PathBuf,
    /// Max file size to read
    max_read_size: Option<u64>,
    /// Parsed zip metadata
    index: ZipIndex,
}

impl ZipSource {
    /// Create a new `ZipSource` instance
    pub(crate) fn new(config: &AccessorConfig, archive_path: PathBuf) -> AccessorResult<Self> {
        let index = ZipIndex::open(archive_path.clone())?;
        Ok(Self {
            archive_path,
            max_read_size: config.max_read_size,
            index,
        })
    }

    /// Return a `ZipFs` structure
    fn zipfs(&self) -> ZipFs {
        ZipFs::new(self.index.clone())
    }
}

impl SourceBackend for ZipSource {
    fn source_id(&self) -> SourceId {
        SourceId::Zip(self.archive_path.clone())
    }

    fn read_file(&self, inner: &InnerPath) -> AccessorResult<Vec<u8>> {
        self.zipfs().read_file(inner, self.max_read_size)
    }

    fn read_dir(&self, inner: &InnerPath) -> AccessorResult<Vec<DirEntry>> {
        self.zipfs().read_dir(inner)
    }

    fn read_dir_handle(&self, handle: &DirHandle) -> AccessorResult<Vec<DirEntry>> {
        self.zipfs().read_dir_handle(handle)
    }

    fn globfs(&self, directory: &InnerPath, pattern: &str) -> AccessorResult<Vec<GlobMatch>> {
        self.zipfs().globfs(directory, pattern)
    }

    fn read_file_handle(&self, handle: &FileHandle) -> AccessorResult<Vec<u8>> {
        self.zipfs().read_handle(handle, self.max_read_size)
    }

    fn open_reader(&self, inner: &InnerPath) -> AccessorResult<AccessorReader> {
        self.zipfs().reader(inner)
    }

    fn open_reader_handle(&self, handle: &FileHandle) -> AccessorResult<AccessorReader> {
        self.zipfs().reader_handle(handle)
    }
}

#[cfg(test)]
mod tests {
    use crate::accessor::config::AccessorConfig;
    use crate::accessor::entry::handle::{DirHandle, FileHandle};
    use crate::accessor::entry::locator::{DirLocator, FileLocator};
    use crate::accessor::error::AccessorError;
    use crate::accessor::location::path::InnerPath;
    use crate::accessor::source::backend::SourceBackend;
    use crate::accessor::source::zip::ZipSource;
    use std::{
        fs::{self, File},
        io::Write,
        path::PathBuf,
    };
    use zip::{ZipWriter, write::SimpleFileOptions};

    fn setup(test_name: &str) -> PathBuf {
        let dir = PathBuf::from("./tmp/zip_source").join(test_name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_zip(path: &PathBuf, entries: &[(&str, &[u8])]) {
        let file = File::create(path).unwrap();
        let mut writer = ZipWriter::new(file);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for (name, contents) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(contents).unwrap();
        }
        writer.finish().unwrap();
    }

    #[test]
    fn test_zip_source_read_file() {
        let dir = setup("test_zip_source_read_file");
        let archive = dir.join("archive.zip");
        write_zip(&archive, &[("inner.txt", b"zip source payload")]);
        let source = ZipSource::new(&AccessorConfig::default(), archive).unwrap();

        let bytes = source
            .read_file(&InnerPath::new(PathBuf::from("inner.txt")))
            .unwrap();
        assert_eq!(bytes, b"zip source payload");
    }

    #[test]
    fn test_zip_source_read_file_handle_rejects_host_locator() {
        let dir = setup("test_zip_source_read_file_handle_rejects_host_locator");
        let archive = dir.join("archive.zip");
        write_zip(&archive, &[("inner.txt", b"zip source payload")]);
        let source = ZipSource::new(&AccessorConfig::default(), archive).unwrap();

        let handle = FileHandle::host(dir.join("inner.txt"));
        let err = source.read_file_handle(&handle).unwrap_err();
        assert!(matches!(err, AccessorError::InvalidHandle { .. }));
    }

    #[test]
    fn test_zip_source_read_dir_handle() {
        let dir = setup("test_zip_source_read_dir_handle");
        let archive = dir.join("archive.zip");
        write_zip(
            &archive,
            &[
                ("home/test.txt", b"payload"),
                ("home/nested/other.txt", b"other"),
            ],
        );

        let source = ZipSource::new(&AccessorConfig::default(), archive.clone()).unwrap();
        let handle = DirHandle::new(DirLocator::Zip {
            archive,
            entry_index: 0,
            prefix: String::from("home"),
        });

        let entries = source.read_dir_handle(&handle).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_zip_source_enforces_max_read_size() {
        let dir = setup("test_zip_source_enforces_max_read_size");
        let archive = dir.join("archive.zip");
        write_zip(&archive, &[("big.bin", &[0u8; 32])]);

        let source = ZipSource::new(
            &AccessorConfig {
                max_read_size: Some(16),
                ..AccessorConfig::default()
            },
            archive,
        )
        .unwrap();

        let err = source
            .read_file(&InnerPath::new(PathBuf::from("big.bin")))
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
    fn test_zip_source_read_file_handle() {
        let dir = setup("test_zip_source_read_file_handle");
        let archive = dir.join("archive.zip");
        write_zip(&archive, &[("inner.txt", b"handle payload")]);

        let source = ZipSource::new(&AccessorConfig::default(), archive.clone()).unwrap();
        let handle = FileHandle::new(FileLocator::Zip {
            archive,
            entry_index: 0,
            entry: String::from("inner.txt"),
        });

        let bytes = source.read_file_handle(&handle).unwrap();
        assert_eq!(bytes, b"handle payload");
    }

    #[test]
    fn test_zipfs_read_document() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/archives/document.odt");

        let source = ZipSource::new(&AccessorConfig::default(), test_location.clone()).unwrap();
        let bytes = source
            .read_file(&InnerPath::new(PathBuf::from("meta.xml")))
            .unwrap();
        assert_eq!(bytes.len(), 974);
    }
}
