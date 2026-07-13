use crate::accessor::{
    config::AccessorConfig,
    entry::{
        handle::{DirEntry, DirHandle, FileHandle, GlobMatch},
        locator::SourceId,
    },
    error::{AccessorError, AccessorResult},
    filesystem::ntfs::{data::NtfsFs, volume::NtfsVolume},
    io::reader::AccessorReader,
    location::path::InnerPath,
    source::backend::SourceBackend,
};
use std::{
    io::{Read, Seek},
    path::PathBuf,
};

/// Use live NTFS filesystem as our source for data access
pub(crate) struct NtfsSource {
    /// Windows drive letter to target
    drive: char,
    /// Max file size to read
    max_read_size: Option<u64>,
    /// Reader for NTFS filesystem
    fs: Box<dyn NtfsFsBackend>,
}

/// NTFS accessor backend for reading files and directories
trait NtfsFsBackend: Send {
    /// Read a file via raw disk access by file path
    fn read_file(&self, inner: &InnerPath, max_read_size: Option<u64>) -> AccessorResult<Vec<u8>>;
    /// Read a file via raw disk access by file reference
    fn read_handle(
        &self,
        handle: &FileHandle,
        max_read_size: Option<u64>,
    ) -> AccessorResult<Vec<u8>>;
    /// List files and directories via raw disk access by file path
    fn read_dir(&self, inner: &InnerPath) -> AccessorResult<Vec<DirEntry>>;
    /// List files and directories via raw disk access by directory reference
    fn read_dir_handle(&self, handle: &DirHandle) -> AccessorResult<Vec<DirEntry>>;
    /// Apply a glob pattern
    fn globfs(&self, directory: &InnerPath, pattern: &str) -> AccessorResult<Vec<GlobMatch>>;
    /// Open a file for streaming via raw disk access by file path
    fn reader(&self, inner: &InnerPath) -> AccessorResult<AccessorReader>;
    /// Open a file for streaming via raw disk access by file reference
    fn reader_handle(&self, handle: &FileHandle) -> AccessorResult<AccessorReader>;
}

impl<T> NtfsFsBackend for NtfsFs<T>
where
    T: Read + Seek + Send + 'static,
{
    fn read_file(&self, inner: &InnerPath, max_read_size: Option<u64>) -> AccessorResult<Vec<u8>> {
        self.read_file(inner, max_read_size)
    }

    fn read_handle(
        &self,
        handle: &FileHandle,
        max_read_size: Option<u64>,
    ) -> AccessorResult<Vec<u8>> {
        self.read_handle(handle, max_read_size)
    }

    fn read_dir(&self, inner: &InnerPath) -> AccessorResult<Vec<DirEntry>> {
        self.read_dir(inner)
    }

    fn read_dir_handle(&self, handle: &DirHandle) -> AccessorResult<Vec<DirEntry>> {
        self.read_dir_handle(handle)
    }

    fn globfs(&self, directory: &InnerPath, pattern: &str) -> AccessorResult<Vec<GlobMatch>> {
        self.globfs(directory, pattern)
    }

    fn reader(&self, inner: &InnerPath) -> AccessorResult<AccessorReader> {
        self.reader(inner)
    }

    fn reader_handle(&self, handle: &FileHandle) -> AccessorResult<AccessorReader> {
        self.reader_handle(handle)
    }
}

impl NtfsSource {
    /// Create a new `NtfsSource` instance
    pub(crate) fn new(config: &AccessorConfig, drive: char) -> AccessorResult<Self> {
        if !drive.is_ascii_alphabetic() {
            return Err(AccessorError::location(
                format!("raw:{drive}:"),
                "raw source drive letter must be alphabetic",
            ));
        }

        Ok(Self {
            drive,
            max_read_size: config.max_read_size,
            fs: open_ntfs_fs(drive)?,
        })
    }

    /// Create a new `NtfsSource` instance via a raw disk image
    pub(crate) fn from_image(config: &AccessorConfig, image_path: PathBuf) -> AccessorResult<Self> {
        let volume = NtfsVolume::open_image(image_path)?;
        Ok(Self {
            drive: 'X',
            max_read_size: config.max_read_size,
            fs: Box::new(NtfsFs::new(volume, 'X')),
        })
    }
}

impl SourceBackend for NtfsSource {
    fn source_id(&self) -> crate::accessor::entry::locator::SourceId {
        SourceId::RawNtfs(self.drive)
    }

    fn read_file(&self, inner: &InnerPath) -> AccessorResult<Vec<u8>> {
        self.fs.read_file(inner, self.max_read_size)
    }

    fn read_dir(&self, inner: &InnerPath) -> AccessorResult<Vec<DirEntry>> {
        self.fs.read_dir(inner)
    }

    fn read_dir_handle(&self, handle: &DirHandle) -> AccessorResult<Vec<DirEntry>> {
        self.fs.read_dir_handle(handle)
    }

    fn globfs(&self, directory: &InnerPath, pattern: &str) -> AccessorResult<Vec<GlobMatch>> {
        self.fs.globfs(directory, pattern)
    }

    fn read_file_handle(&self, handle: &FileHandle) -> AccessorResult<Vec<u8>> {
        self.fs.read_handle(handle, self.max_read_size)
    }

    fn open_reader(&self, inner: &InnerPath) -> AccessorResult<AccessorReader> {
        self.fs.reader(inner)
    }

    fn open_reader_handle(&self, handle: &FileHandle) -> AccessorResult<AccessorReader> {
        self.fs.reader_handle(handle)
    }
}

/// Open the raw NTFS disk on Windows system. Will not work on non-Windows platforms
fn open_ntfs_fs(drive: char) -> AccessorResult<Box<dyn NtfsFsBackend>> {
    let volume = NtfsVolume::open_live_drive(drive)?;

    Ok(Box::new(NtfsFs::new(volume, drive)))
}

#[cfg(test)]
mod tests {
    use crate::accessor::{
        config::AccessorConfig,
        location::path::InnerPath,
        source::{backend::SourceBackend, ntfs::NtfsSource},
    };
    use std::path::PathBuf;

    #[test]
    fn test_ntfs_image() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/filesystems/ntfs/test.raw");
        let config = AccessorConfig::default();

        let accessor = NtfsSource::from_image(&config, path).unwrap();

        let matches = accessor
            .globfs(&InnerPath::new(PathBuf::from("hello")), "*.txt")
            .unwrap();

        assert_eq!(matches.len(), 1);
    }

    #[test]
    #[cfg(windows)]
    fn test_ntfs_read_root_dirs() {
        use crate::accessor::entry::handle::EntryKind;

        let config = AccessorConfig::default();
        let source = NtfsSource::new(&config, 'C').unwrap();

        let matches = source.globfs(&InnerPath::new(PathBuf::new()), "*").unwrap();
        for entry in matches {
            if entry.meta.kind != EntryKind::Directory {
                continue;
            }

            let entries = source
                .read_dir(&InnerPath::new(PathBuf::from(
                    entry.handle.as_directory().unwrap().display_path(),
                )))
                .unwrap();

            if entry.meta.display_path == "C:\\Users" {
                assert!(!entries.is_empty());
            }
        }
    }
}
