use crate::accessor::{
    entry::{
        handle::{DirEntry, DirHandle, FileHandle, GlobMatch},
        locator::SourceId,
    },
    error::AccessorResult,
    io::reader::AccessorReader,
    location::path::InnerPath,
};

/// Common interface for an opened accessor source backend
///
/// Each implementation handles one `SourceId` variant and validates that entry
/// handles belong to that backend before reading
pub(crate) trait SourceBackend {
    /// Return the `SourceId` for the `SourceBackened`
    fn source_id(&self) -> SourceId;

    /// Read provide file. This reads the entire file into memory
    fn read_file(&self, inner: &InnerPath) -> AccessorResult<Vec<u8>>;

    /// Read a directory and return directory contents
    fn read_dir(&self, inner: &InnerPath) -> AccessorResult<Vec<DirEntry>>;

    fn read_dir_handle(&self, handle: &DirHandle) -> AccessorResult<Vec<DirEntry>>;

    /// Apply a glob like pattern and return matches
    fn globfs(&self, directory: &InnerPath, pattern: &str) -> AccessorResult<Vec<GlobMatch>>;

    /// Read a file using provided `FileHandle`. Can be used to read a file directly instead of walking the filesystem
    fn read_file_handle(&self, handle: &FileHandle) -> AccessorResult<Vec<u8>>;

    /// Create a `AccessorReader` for provided file. Allows `Read+Seek` for provided file
    fn open_reader(&self, inner: &InnerPath) -> AccessorResult<AccessorReader>;

    /// Create a `AccessorReader` for provided `FileHandle`. Allows `Read+Seek` for provided file
    fn open_reader_handle(&self, handle: &FileHandle) -> AccessorResult<AccessorReader>;
}
