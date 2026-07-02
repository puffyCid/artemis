use crate::accessor::{
    entry::handle::{DirEntry, FileHandle, GlobMatch},
    error::AccessorResult,
    io::reader::AccessorReader,
    location::path::InnerPath,
    source::{backend::SourceBackend, host::HostSource},
};

/// Supported sources that we support reading data from
///
/// Host - Live system
/// Zip - Zip file
/// NTFS - raw disk access
pub(crate) enum Source {
    /// Use the live system as the source
    Host(HostSource),
}

impl Source {
    /// Read a file at provided `InnerPath`
    pub(crate) fn read_file(&self, inner: &InnerPath) -> AccessorResult<Vec<u8>> {
        match self {
            Self::Host(source) => source.read_file(inner),
        }
    }

    /// Read the directory at `InnerPath` and return its contents
    pub(crate) fn read_dir(&self, inner: &InnerPath) -> AccessorResult<Vec<DirEntry>> {
        match self {
            Self::Host(source) => source.read_dir(inner),
        }
    }

    /// Apply a glob pattern and return matches
    pub(crate) fn glob(&self, dir: &InnerPath, pattern: &str) -> AccessorResult<Vec<GlobMatch>> {
        match self {
            Self::Host(source) => source.glob(dir, pattern),
        }
    }

    /// Read the file reference handle
    pub(crate) fn read_file_handle(&self, handle: &FileHandle) -> AccessorResult<Vec<u8>> {
        match self {
            Self::Host(source) => source.read_file_handle(handle),
        }
    }

    /// Open a `AccessorReader` to the provided `FileHandle`
    pub(crate) fn open_reader_handle(&self, handle: &FileHandle) -> AccessorResult<AccessorReader> {
        match self {
            Self::Host(source) => source.open_reader_handle(handle),
        }
    }

    /// Open a `AccessorReader` to the provided `InnerPath`
    pub(crate) fn open_reader(&self, inner: &InnerPath) -> AccessorResult<AccessorReader> {
        match self {
            Self::Host(source) => source.open_reader(inner),
        }
    }
}
