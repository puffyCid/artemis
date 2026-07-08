use crate::accessor::{
    entry::{
        handle::{DirEntry, DirHandle, EntryKind, EntryMeta, FileHandle, GlobMatch, ItemHandle},
        locator::{DirLocator, FileLocator},
    },
    error::{AccessorError, AccessorResult},
    io::reader::AccessorReader,
    location::path::InnerPath,
};
use glob::Pattern;
use std::{
    fs::{self, File, metadata, read},
    path::{Path, PathBuf},
};

/// Filesystem reader for a live OS
///
/// This uses native Rust Filesystem APIs to navigate the filesystem
pub(crate) struct HostFs;

impl HostFs {
    /// Read the file at provided `InnerPath`
    pub(crate) fn read_file(
        inner: &InnerPath,
        max_read_size: Option<u64>,
    ) -> AccessorResult<Vec<u8>> {
        let path = HostFs::resolve_host_path(inner);
        if !path.exists() {
            return Err(AccessorError::not_found(HostFs::display_path(&path)));
        }

        if !path.is_file() {
            return Err(AccessorError::not_a_file(HostFs::display_path(&path)));
        }
        let metadata = metadata(&path).map_err(|err| AccessorError::io_path(&path, err))?;
        let size = metadata.len();
        if let Some(limit) = max_read_size {
            if size > limit {
                return Err(AccessorError::file_too_large(size, limit));
            }
        }
        read(&path).map_err(|err| AccessorError::io_path(&path, err))
    }

    /// Read the file reference handle
    ///
    /// Since this is using Rust Filesystem APIs we can treat this a normal `read_file` action
    pub(crate) fn read_handle(
        handle: &FileHandle,
        max_read_size: Option<u64>,
    ) -> AccessorResult<Vec<u8>> {
        // When reading files using the the OS APIs, we do not need to read a file via its file reference
        // We can always just read it directly
        match &handle.locator {
            FileLocator::Host { path } => {
                HostFs::read_file(&InnerPath::new(path.clone()), max_read_size)
            }
            _ => Err(AccessorError::invalid_handle(format!(
                "host source cannot read handle for {}",
                handle.display_path()
            ))),
        }
    }

    /// Read the directory at `InnerPath` and return its contents
    pub(crate) fn read_dir(inner: &InnerPath) -> AccessorResult<Vec<DirEntry>> {
        let path = HostFs::resolve_host_path(inner);
        if !path.exists() {
            return Err(AccessorError::not_found(HostFs::display_path(&path)));
        }
        if !path.is_dir() {
            return Err(AccessorError::not_a_directory(HostFs::display_path(&path)));
        }

        let read_dir_value =
            fs::read_dir(&path).map_err(|err| AccessorError::io_path(&path, err))?;
        let mut entries = Vec::new();

        for entry_result in read_dir_value {
            let entry = entry_result.map_err(|err| AccessorError::io_path(&path, err))?;
            let file_type = entry
                .file_type()
                .map_err(|err| AccessorError::io_path(entry.path(), err))?;
            let child_path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();

            let (handle, kind) = if file_type.is_dir() {
                (
                    ItemHandle::Directory(DirHandle::host(&child_path)),
                    EntryKind::Directory,
                )
            } else if file_type.is_file() {
                (
                    ItemHandle::File(FileHandle::host(&child_path)),
                    EntryKind::File,
                )
            } else {
                (
                    ItemHandle::File(FileHandle::host(&child_path)),
                    EntryKind::Unsupported,
                )
            };
            let metadata = entry
                .metadata()
                .map_err(|err| AccessorError::io_path(&child_path, err))?;
            let meta = EntryMeta::new(kind, metadata.len(), HostFs::display_path(&child_path));
            entries.push(DirEntry::new(name, handle, meta));
        }

        //entries.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(entries)
    }

    /// List the directory referenced by `DirHandle`
    pub(crate) fn read_dir_handle(handle: &DirHandle) -> AccessorResult<Vec<DirEntry>> {
        match &handle.locator {
            DirLocator::Host { path } => HostFs::read_dir(&InnerPath::new(path.clone())),
            _ => Err(AccessorError::invalid_handle(format!(
                "host source cannot list directory handle for {}",
                handle.display_path()
            ))),
        }
    }

    /// Apply a glob pattern and return matches
    pub(crate) fn globfs(directory: &InnerPath, pattern: &str) -> AccessorResult<Vec<GlobMatch>> {
        let dir_path = HostFs::resolve_host_path(directory);
        if !dir_path.exists() {
            return Err(AccessorError::not_found(HostFs::display_path(&dir_path)));
        }
        if !dir_path.is_dir() {
            return Err(AccessorError::not_a_directory(HostFs::display_path(
                &dir_path,
            )));
        }
        let glob_pattern = Pattern::new(pattern)
            .map_err(|err| AccessorError::bad_glob(pattern, err.to_string()))?;

        let mut matches = Vec::new();
        for entry in
            fs::read_dir(&dir_path).map_err(|err| AccessorError::io_path(&dir_path, err))?
        {
            let entry = entry.map_err(|err| AccessorError::io_path(&dir_path, err))?;
            let file_type = entry
                .file_type()
                .map_err(|err| AccessorError::io_path(entry.path(), err))?;

            let name = entry.file_name().to_string_lossy().into_owned();
            // Check if file path matches our glob
            if !glob_pattern.matches(&name) {
                continue;
            }

            let child_path = entry.path();
            let metadata = entry
                .metadata()
                .map_err(|err| AccessorError::io_path(&child_path, err))?;

            // Determine our glob entry type
            let (handle, kind) = if file_type.is_dir() {
                (
                    ItemHandle::Directory(DirHandle::host(&child_path)),
                    EntryKind::Directory,
                )
            } else if file_type.is_file() {
                (
                    ItemHandle::File(FileHandle::host(&child_path)),
                    EntryKind::File,
                )
            } else {
                (
                    ItemHandle::File(FileHandle::host(&child_path)),
                    EntryKind::Unsupported,
                )
            };

            // Get very small bit of metadata
            let meta = EntryMeta::new(kind, metadata.len(), HostFs::display_path(&child_path));
            matches.push(GlobMatch::new(handle, meta));
        }

        //matches.sort_by(|left, right| left.handle.display_path().cmp(&right.handle.display_path()));
        Ok(matches)
    }

    /// Open a `AccessorReader` to the provided `InnerPath`
    ///
    /// Can be used to stream large files
    pub(crate) fn reader(inner: &InnerPath) -> AccessorResult<AccessorReader> {
        let path = HostFs::resolve_host_path(inner);
        if !path.exists() {
            return Err(AccessorError::not_found(path.display().to_string()));
        }
        if !path.is_file() {
            return Err(AccessorError::not_a_file(path.display().to_string()));
        }

        let file = File::open(&path).map_err(|err| AccessorError::io_path(path, err))?;
        Ok(AccessorReader::Host(file))
    }

    /// Open a `AccessorReader` to the provided `FileHandle`
    ///
    /// Can be used to stream large files. Since this is using Rust Filesystem APIs we can treat this a normal `reader` action
    pub(crate) fn reader_handle(handle: &FileHandle) -> AccessorResult<AccessorReader> {
        // When reading files using the the OS APIs, we do not need to read a file via its file reference
        // We can always just read it directly
        match &handle.locator {
            FileLocator::Host { path } => HostFs::reader(&InnerPath::new(path.clone())),
            _ => Err(AccessorError::invalid_handle(format!(
                "host source cannot open reader handle for {}",
                handle.display_path()
            ))),
        }
    }

    /// Return `PathBuf` from `InnerPath`
    fn resolve_host_path(inner: &InnerPath) -> PathBuf {
        if inner.is_empty() {
            PathBuf::from(".")
        } else {
            inner.as_path().to_path_buf()
        }
    }

    /// Return target path as a String
    fn display_path(path: &Path) -> String {
        path.display().to_string()
    }
}

#[cfg(test)]
mod tests {
    use crate::accessor::{
        entry::handle::FileHandle, error::AccessorError, filesystem::host::HostFs,
        location::path::InnerPath,
    };
    use std::{
        fs::{self, File},
        io::Write,
        path::PathBuf,
    };

    fn setup(test_name: &str) -> PathBuf {
        let dir = PathBuf::from("./tmp/hostfs").join(test_name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn inner(dir: &PathBuf, part: &str) -> InnerPath {
        InnerPath::new(dir.join(part))
    }

    fn write_file(dir: &PathBuf, name: &str, contents: &[u8]) {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        File::create(path).unwrap().write_all(contents).unwrap();
    }

    #[test]
    fn test_accessor_hostfs() {
        let dir = setup("test_accessor_hostfs");
        write_file(&dir, "test.txt", b"my first test");
        let bytes = HostFs::read_file(&inner(&dir, "test.txt"), None).unwrap();
        assert_eq!(bytes, b"my first test");
    }

    #[test]
    fn test_read_file_returns_contents() {
        let dir = setup("test_read_file_returns_contents");
        write_file(&dir, "hello.txt", b"hello world");
        let bytes = HostFs::read_file(&inner(&dir, "hello.txt"), None).unwrap();
        assert_eq!(bytes, b"hello world");
    }

    #[test]
    fn test_read_file_handle() {
        let dir = setup("test_read_file_handle");
        write_file(&dir, "hello.txt", b"hello world");
        let inner = inner(&dir, "hello.txt");
        let handle = FileHandle::host(inner.as_path().to_path_buf());
        let bytes = HostFs::read_handle(&handle, Some(200)).unwrap();
        assert_eq!(bytes, b"hello world");
    }

    #[test]
    fn test_read_file_not_found() {
        let dir = setup("test_read_file_not_found");
        let err = HostFs::read_file(&inner(&dir, "missing.txt"), None).unwrap_err();
        assert!(matches!(err, AccessorError::NotFound { .. }));
    }

    #[test]
    fn test_open_reader_skips_max_read_size() {
        let dir = setup("test_open_reader_skips_max_read_size");
        write_file(&dir, "big.bin", &[1, 2, 3, 4]);
        let mut reader = HostFs::reader(&inner(&dir, "big.bin")).unwrap();
        let mut buf = Vec::new();
        let size = reader.read_to_end(&mut buf).unwrap();
        assert_eq!(buf, vec![1, 2, 3, 4]);
        assert_eq!(size, 4)
    }

    #[test]
    fn test_hostfs_reader_handle() {
        let dir = setup("test_hostfs_reader_handle");
        write_file(&dir, "big.bin", &[1, 2, 3, 4]);
        let inner = inner(&dir, "big.bin");
        let handle = FileHandle::host(inner.as_path().to_path_buf());

        let mut reader = HostFs::reader_handle(&handle).unwrap();
        let buf = reader.read_bytes(0, 2).unwrap();
        assert_eq!(buf, vec![1, 2,]);
    }

    #[test]
    fn test_hostfs_read_dir() {
        let test = PathBuf::from(".");
        let dir = HostFs::read_dir(&inner(&test, "")).unwrap();
        assert!(!dir.is_empty());
    }

    #[test]
    fn test_hostfs_glob() {
        let _ = setup("test");
        let test = PathBuf::from("./tmp");
        let dir = HostFs::globfs(&inner(&test, ""), "*").unwrap();
        assert!(!dir.is_empty());
    }
}
