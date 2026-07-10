use crate::accessor::{
    entry::handle::{DirEntry, DirHandle, FileHandle, GlobMatch},
    error::AccessorResult,
    io::reader::AccessorReader,
    location::path::InnerPath,
    source::{backend::SourceBackend, host::HostSource, ntfs::NtfsSource, zip::ZipSource},
};

/// Supported sources that we support reading data from
///
/// Host - Live system
/// Zip - Zip file
/// NTFS - raw disk access
pub(crate) enum Source {
    /// Use the live system as the source
    Host(HostSource),
    /// Use a zip file as the source
    Zip(ZipSource),
    /// Use raw NTFS Windows drive as the source
    RawNtfs(NtfsSource),
}

impl Source {
    /// Read a file at provided `InnerPath`
    pub(crate) fn read_file(&self, inner: &InnerPath) -> AccessorResult<Vec<u8>> {
        match self {
            Self::Host(source) => source.read_file(inner),
            Self::Zip(source) => source.read_file(inner),
            Self::RawNtfs(source) => source.read_file(inner),
        }
    }

    /// Read the directory at `InnerPath` and return its contents
    pub(crate) fn read_dir(&self, inner: &InnerPath) -> AccessorResult<Vec<DirEntry>> {
        match self {
            Self::Host(source) => source.read_dir(inner),
            Self::Zip(source) => source.read_dir(inner),
            Self::RawNtfs(source) => source.read_dir(inner),
        }
    }

    pub(crate) fn read_dir_handle(&self, handle: &DirHandle) -> AccessorResult<Vec<DirEntry>> {
        match self {
            Self::Host(source) => source.read_dir_handle(handle),
            Self::Zip(source) => source.read_dir_handle(handle),
            Self::RawNtfs(source) => source.read_dir_handle(handle),
        }
    }

    /// Apply a glob pattern and return matches
    pub(crate) fn globfs(&self, dir: &InnerPath, pattern: &str) -> AccessorResult<Vec<GlobMatch>> {
        match self {
            Self::Host(source) => source.globfs(dir, pattern),
            Self::Zip(source) => source.globfs(dir, pattern),
            Self::RawNtfs(source) => source.globfs(dir, pattern),
        }
    }

    /// Read the file reference handle
    pub(crate) fn read_file_handle(&self, handle: &FileHandle) -> AccessorResult<Vec<u8>> {
        match self {
            Self::Host(source) => source.read_file_handle(handle),
            Self::Zip(source) => source.read_file_handle(handle),
            Self::RawNtfs(source) => source.read_file_handle(handle),
        }
    }

    /// Open a `AccessorReader` to the provided `FileHandle`
    pub(crate) fn open_reader_handle(&self, handle: &FileHandle) -> AccessorResult<AccessorReader> {
        match self {
            Self::Host(source) => source.open_reader_handle(handle),
            Self::Zip(source) => source.open_reader_handle(handle),
            Self::RawNtfs(source) => source.open_reader_handle(handle),
        }
    }

    /// Open a `AccessorReader` to the provided `InnerPath`
    pub(crate) fn open_reader(&self, inner: &InnerPath) -> AccessorResult<AccessorReader> {
        match self {
            Self::Host(source) => source.open_reader(inner),
            Self::Zip(source) => source.open_reader(inner),
            Self::RawNtfs(source) => source.open_reader(inner),
        }
    }
}
