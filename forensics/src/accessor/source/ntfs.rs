use crate::{
    accessor::{
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
    },
    artifacts::os::systeminfo::info::{PlatformType, get_platform_enum},
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

impl<R> NtfsFsBackend for NtfsFs<R>
where
    R: Read + Seek + Send + 'static,
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
    if get_platform_enum() != PlatformType::Windows {
        return Err(AccessorError::Ntfs {
            path: None,
            reason: String::from("Cannot read live NTFS on non-Windows platform"),
        });
    }
    let volume = NtfsVolume::open_live_drive(drive)?;

    return Ok(Box::new(NtfsFs::new(volume, drive)));
}
