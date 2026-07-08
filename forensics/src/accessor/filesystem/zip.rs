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
use std::{collections::BTreeMap, fs::File, io::Read, path::PathBuf, sync::Mutex};
use zip::ZipArchive;

/// A record representing a zip content entry
#[derive(Debug, Clone)]
struct ZipEntryRecord {
    /// Index in central index table
    index: usize,
    /// Path of the file in the zip
    path: String,
    /// If the path is a directory
    is_dir: bool,
    /// Decompressed size
    size: u64,
}

/// Represents our zip archive file
#[derive(Debug, Clone)]
pub(crate) struct ZipIndex {
    /// Path the zip file
    archive_path: PathBuf,
    /// Number of entries in the zip
    entries: Vec<ZipEntryRecord>,
    /// File paths in the zip file used for navigating the zip filesystem
    file_paths: BTreeMap<String, usize>,
}

impl ZipIndex {
    /// Open the zip file for reading
    pub(crate) fn open(archive_path: PathBuf) -> AccessorResult<Self> {
        if !archive_path.exists() {
            return Err(AccessorError::not_found(format!(
                "zip:{}",
                archive_path.display()
            )));
        }

        if !archive_path.is_file() {
            return Err(AccessorError::not_a_file(format!(
                "zip:{}",
                archive_path.display()
            )));
        }

        let file =
            File::open(&archive_path).map_err(|err| AccessorError::io_path(&archive_path, err))?;
        let mut archive = ZipArchive::new(file)
            .map_err(|err| AccessorError::zip(archive_path.clone(), err.to_string()))?;

        // Count of files in the zip file
        let mut entries = Vec::with_capacity(archive.len());
        let mut file_paths = BTreeMap::new();

        // Grab the index of each entry in the zip
        for index in 0..archive.len() {
            let entry = archive
                .by_index(index)
                .map_err(|err| AccessorError::zip(archive_path.clone(), err.to_string()))?;

            let path = ZipFs::normalize_zip_path(entry.name());
            let is_dir = entry.is_dir();
            let size = entry.size();

            if !is_dir && !path.is_empty() {
                // If a zip file contains duplicate path. Something could be wrong with the zip file, ex: corruption
                if file_paths.contains_key(&path) {
                    return Err(AccessorError::zip(
                        archive_path.clone(),
                        format!("duplicate zip entry path: {path}"),
                    ));
                }
                file_paths.insert(path.clone(), entries.len());
            }

            entries.push(ZipEntryRecord {
                index,
                path,
                is_dir,
                size,
            });
        }

        Ok(Self {
            archive_path,
            entries,
            file_paths,
        })
    }

    /// Return the `ZipEntryRecord` associated with the file we want to read
    fn record_at(&self, path: &str) -> Option<&ZipEntryRecord> {
        self.file_paths
            .get(path)
            .and_then(|position| self.entries.get(*position))
    }

    /// Return the Directory index for path
    fn dir_entry_index(&self, prefix: &str) -> u32 {
        self.entries
            .iter()
            .find(|entry| entry.is_dir && entry.path == prefix)
            .map(|entry| entry.index as u32)
            .unwrap_or(0)
    }
}

/// A filesystem like accessor that can be used to read files from a zip file
pub(crate) struct ZipFs {
    /// Index of the file we want to access
    index: ZipIndex,
    archive: Mutex<ZipArchive<File>>,
}

impl ZipFs {
    /// Create a new `ZipFs` instance
    pub(crate) fn new(archive_path: PathBuf) -> AccessorResult<Self> {
        // Open archive twice:
        // First to extract file path metadata and indexes
        // Second to access `ZipArchive` reader for reading content
        let index = ZipIndex::open(archive_path.clone())?;
        let file =
            File::open(&archive_path).map_err(|err| AccessorError::io_path(&archive_path, err))?;
        let archive = ZipArchive::new(file)
            .map_err(|err| AccessorError::zip(archive_path.clone(), err.to_string()))?;
        Ok(Self {
            index,
            archive: Mutex::new(archive),
        })
    }

    /// Read the file inside the zip file into memory
    pub(crate) fn read_file(
        &self,
        inner: &InnerPath,
        max_read_size: Option<u64>,
    ) -> AccessorResult<Vec<u8>> {
        let path = Self::inner_to_prefix(inner);
        // Get the zip entry from the parsed zip index
        let record = self
            .index
            .record_at(&path)
            .ok_or_else(|| AccessorError::not_found(self.display_entry_path(&path)))?;

        if record.is_dir {
            return Err(AccessorError::not_a_file(self.display_entry_path(&path)));
        }

        if let Some(limit) = max_read_size {
            if record.size > limit {
                return Err(AccessorError::file_too_large(record.size, limit));
            }
        }
        self.read_entry_bytes(record.index)
    }

    /// Read a file by its index value directly
    ///
    /// Can be obtained from `read_dir` or `globfs`
    pub(crate) fn read_handle(
        &self,
        handle: &FileHandle,
        max_read_size: Option<u64>,
    ) -> AccessorResult<Vec<u8>> {
        match &handle.locator {
            FileLocator::Zip {
                archive,
                entry_index,
                entry,
            } => {
                if archive != &self.index.archive_path {
                    return Err(AccessorError::invalid_handle(format!(
                        "zip source cannot read handle for {}",
                        handle.display_path()
                    )));
                }

                // Find the file by its index value
                let record = self
                    .index
                    .entries
                    .iter()
                    .find(|record| record.index == *entry_index as usize)
                    .ok_or_else(|| AccessorError::not_found(handle.display_path()))?;

                if record.path != *entry {
                    return Err(AccessorError::invalid_handle(format!(
                        "zip entry path mismatch for {}",
                        handle.display_path()
                    )));
                }

                if record.is_dir {
                    return Err(AccessorError::not_a_file(handle.display_path()));
                }

                if let Some(limit) = max_read_size {
                    if record.size > limit {
                        return Err(AccessorError::file_too_large(record.size, limit));
                    }
                }

                self.read_entry_bytes(record.index)
            }
            _ => Err(AccessorError::invalid_handle(format!(
                "zip source cannot read handle for {}",
                handle.display_path()
            ))),
        }
    }

    /// Read a directory in a zip file and return its contents
    pub(crate) fn read_dir(&self, inner: &InnerPath) -> AccessorResult<Vec<DirEntry>> {
        self.read_dir_inner(inner)
    }

    /// Read a directory by its index value directly
    ///
    /// Can be obtained from `read_dir` or `globfs`
    pub(crate) fn read_dir_handle(&self, handle: &DirHandle) -> AccessorResult<Vec<DirEntry>> {
        match &handle.locator {
            DirLocator::Zip {
                archive, prefix, ..
            } => {
                if archive != &self.index.archive_path {
                    return Err(AccessorError::invalid_handle(format!(
                        "zip source cannot list directory handle for {}",
                        handle.display_path()
                    )));
                }

                self.read_dir_inner(&InnerPath::new(PathBuf::from(prefix.clone())))
            }
            _ => Err(AccessorError::invalid_handle(format!(
                "zip source cannot list directory handle for {}",
                handle.display_path()
            ))),
        }
    }

    /// Apply a glob pattern and return matches
    pub(crate) fn globfs(
        &self,
        directory: &InnerPath,
        pattern: &str,
    ) -> AccessorResult<Vec<GlobMatch>> {
        // Convert the inner path to our zip file path
        // zip:test.zip!/home/*.txt -> pattern is 'home/*.txt'
        // Prefix is home/
        // If the prefix is empty then root directory is the default
        let prefix = Self::inner_to_prefix(directory);
        if !prefix.is_empty()
            && !self.index.file_paths.contains_key(&prefix)
            && !self
                .index
                .entries
                .iter()
                .any(|entry| entry.is_dir && entry.path == prefix)
            && self.list_children(&prefix)?.is_empty()
        {
            return Err(AccessorError::not_a_directory(
                self.display_entry_path(&prefix),
            ));
        }

        let glob_pattern = Pattern::new(pattern)
            .map_err(|err| AccessorError::bad_glob(pattern, err.to_string()))?;
        let children = self.list_children(&prefix)?;
        let mut matches = Vec::new();

        for (name, child) in children {
            if !glob_pattern.matches(&name) {
                continue;
            }
            let (handle, kind, size, display_path) = match child {
                ZipChild::File { record, .. } => (
                    ItemHandle::File(FileHandle::new(FileLocator::Zip {
                        archive: self.index.archive_path.clone(),
                        entry_index: record.index as u32,
                        entry: record.path.clone(),
                    })),
                    EntryKind::File,
                    record.size,
                    self.display_entry_path(&record.path),
                ),
                ZipChild::Directory { prefix, .. } => (
                    ItemHandle::Directory(DirHandle::new(DirLocator::Zip {
                        archive: self.index.archive_path.clone(),
                        entry_index: self.index.dir_entry_index(&prefix),
                        prefix: prefix.clone(),
                    })),
                    EntryKind::Directory,
                    0,
                    self.display_entry_path(&prefix),
                ),
            };

            matches.push(GlobMatch::new(
                handle,
                EntryMeta::new(kind, size, display_path),
            ));
        }

        Ok(matches)
    }

    /// Open a `AccessorReader` to the provided `InnerPath`
    ///
    /// Due to the zip file crate limitations this reader reads the file into memory
    pub(crate) fn reader(
        &self,
        inner: &InnerPath,
        max_read_size: Option<u64>,
    ) -> AccessorResult<AccessorReader> {
        let bytes = self.read_file(inner, max_read_size)?;
        Ok(AccessorReader::memory(bytes))
    }

    /// Open a `AccessorReader` to the provided `FileHandle`
    ///
    /// Due to the zip file crate limitations this reader reads the file into memory
    pub(crate) fn reader_handle(
        &self,
        handle: &FileHandle,
        max_read_size: Option<u64>,
    ) -> AccessorResult<AccessorReader> {
        let bytes = self.read_handle(handle, max_read_size)?;
        Ok(AccessorReader::memory(bytes))
    }

    /// Helper to convert `InnerPath` to String for zip content file access
    fn inner_to_prefix(inner: &InnerPath) -> String {
        if inner.is_empty() {
            String::new()
        } else {
            Self::normalize_zip_path(&inner.display())
        }
    }

    /// Normalize paths to represent zip content file paths
    fn normalize_zip_path(path: &str) -> String {
        path.replace('\\', "/")
            .trim_start_matches('/')
            .trim_end_matches('/')
            .to_string()
    }

    /// Return target path as a String
    fn display_entry_path(&self, entry_path: &str) -> String {
        if entry_path.is_empty() {
            format!("zip:{}", self.index.archive_path.display())
        } else {
            format!("zip:{}!{entry_path}", self.index.archive_path.display())
        }
    }

    /// Read the zip content file. Currently who file is decompressed into memory
    fn read_entry_bytes(&self, index: usize) -> AccessorResult<Vec<u8>> {
        // Access the `ZipArchive` file reader
        let mut archive = self.archive.lock().map_err(|_| {
            AccessorError::zip(
                self.index.archive_path.clone(),
                "zip archive lock poisoned".to_string(),
            )
        })?;

        let mut entry = archive
            .by_index(index)
            .map_err(|err| AccessorError::zip(self.index.archive_path.clone(), err.to_string()))?;

        let mut buf = Vec::new();
        entry
            .read_to_end(&mut buf)
            .map_err(|err| AccessorError::zip(self.index.archive_path.clone(), err.to_string()))?;
        Ok(buf)
    }

    /// Read the zip content directory and return entries
    fn read_dir_inner(&self, inner: &InnerPath) -> AccessorResult<Vec<DirEntry>> {
        let prefix = Self::inner_to_prefix(inner);
        if !prefix.is_empty()
            && !self.index.file_paths.contains_key(&prefix)
            && !self
                .index
                .entries
                .iter()
                .any(|entry| entry.is_dir && entry.path == prefix)
            && self.list_children(&prefix)?.is_empty()
        {
            return Err(AccessorError::not_found(self.display_entry_path(&prefix)));
        }

        let children = self.list_children(&prefix)?;
        let mut entries = Vec::with_capacity(children.len());

        for (name, child) in children {
            let (handle, kind, size, display_path) = match child {
                // Symbolic links are treated as a file
                ZipChild::File { record, .. } => (
                    ItemHandle::File(FileHandle::new(FileLocator::Zip {
                        archive: self.index.archive_path.clone(),
                        entry_index: record.index as u32,
                        entry: record.path.clone(),
                    })),
                    EntryKind::File,
                    record.size,
                    self.display_entry_path(&record.path),
                ),
                ZipChild::Directory { prefix, .. } => (
                    ItemHandle::Directory(DirHandle::new(DirLocator::Zip {
                        archive: self.index.archive_path.clone(),
                        entry_index: self.index.dir_entry_index(&prefix),
                        prefix: prefix.clone(),
                    })),
                    EntryKind::Directory,
                    0,
                    self.display_entry_path(&prefix),
                ),
            };
            entries.push(DirEntry::new(
                name,
                handle,
                EntryMeta::new(kind, size, display_path),
            ));
        }

        Ok(entries)
    }

    /// Identify children of our prefix/directory
    ///
    /// Prefix is 'home/' find all entries that have the prefix: 'home/'
    fn list_children(&self, prefix: &str) -> AccessorResult<BTreeMap<String, ZipChild>> {
        // Normalize a prefix path to proper zip path
        let prefix = Self::normalize_zip_path(prefix);
        let prefix_with_slash = if prefix.is_empty() {
            // Empty path means we treat as root path
            String::new()
        } else {
            format!("{prefix}/")
        };

        let mut children = BTreeMap::<String, ZipChild>::new();

        // Go through our parsed zip index records
        for record in &self.index.entries {
            let entry_path = &record.path;
            if entry_path.is_empty() {
                continue;
            }

            let remainder = if prefix.is_empty() {
                entry_path.as_str()
            } else if *entry_path == prefix {
                // Ignore the prefix directory. Example, if the directory prefix is 'home/' we should skip the entry 'home/'
                // If we do not have this then the `strip_prefix` function below would match and we would track it
                continue;
            } else if let Some(rest) = entry_path.strip_prefix(&prefix_with_slash) {
                // If we found an zip entry that matches the prefix, we keep it
                rest
            } else {
                continue;
            };

            if remainder.is_empty() {
                continue;
            }

            if let Some(slash) = remainder.find('/') {
                let name = remainder[..slash].to_string();
                children
                    .entry(name.clone())
                    .or_insert_with(|| ZipChild::directory(name, &prefix, &self.index));
                continue;
            }

            if record.is_dir {
                children.insert(
                    remainder.to_string(),
                    ZipChild::directory(remainder.to_string(), &prefix, &self.index),
                );
            } else {
                children.insert(
                    remainder.to_string(),
                    ZipChild::file(record.clone(), remainder.to_string()),
                );
            }
        }
        Ok(children)
    }
}

/// Structure to represent zip entries
#[derive(Debug, Clone)]
enum ZipChild {
    /// Zip content is a file
    File {
        record: ZipEntryRecord,
        name: String,
    },
    /// Zip content is a directory
    Directory { prefix: String, name: String },
}

impl ZipChild {
    /// Return a `ZipChild` file. Symbolic links are treated as files
    fn file(record: ZipEntryRecord, name: String) -> Self {
        Self::File { record, name }
    }

    /// Return a `ZipChild` directory
    fn directory(name: String, parent_prefix: &str, index: &ZipIndex) -> Self {
        let prefix = if parent_prefix.is_empty() {
            name.clone()
        } else {
            format!("{parent_prefix}/{name}")
        };
        let _ = index.dir_entry_index(&prefix);
        Self::Directory { prefix, name }
    }
}

#[cfg(test)]
mod tests {
    use crate::accessor::{
        entry::{
            handle::{EntryKind, FileHandle},
            locator::FileLocator,
        },
        error::AccessorError,
        filesystem::zip::ZipFs,
        location::path::InnerPath,
    };
    use std::{
        fs::{self, File},
        io::Write,
        path::PathBuf,
    };
    use zip::{ZipWriter, write::SimpleFileOptions};

    fn setup(test_name: &str) -> PathBuf {
        let dir = PathBuf::from("./tmp/zipfs").join(test_name);
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

    fn inner(part: &str) -> InnerPath {
        if part.is_empty() {
            InnerPath::empty()
        } else {
            InnerPath::new(PathBuf::from(part))
        }
    }

    #[test]
    fn test_zipfs_read_file() {
        let dir = setup("test_zipfs_read_file");
        let archive = dir.join("archive.zip");
        write_zip(
            &archive,
            &[
                ("home/test.txt", b"zip payload"),
                ("home/nested/other.txt", b"other"),
            ],
        );

        let zipfs = ZipFs::new(archive).unwrap();
        let bytes = zipfs.read_file(&inner("home/test.txt"), None).unwrap();
        assert_eq!(bytes, b"zip payload");
    }

    #[test]
    fn test_zipfs_read_dir() {
        let dir = setup("test_zipfs_read_dir");
        let archive = dir.join("archive.zip");
        write_zip(
            &archive,
            &[
                ("home/test.txt", b"zip payload"),
                ("home/nested/other.txt", b"other"),
                ("readme.txt", b"root"),
            ],
        );

        let zipfs = ZipFs::new(archive).unwrap();
        let entries = zipfs.read_dir_inner(&inner("home")).unwrap();

        assert_eq!(entries.len(), 2);
        assert!(entries.iter().any(|entry| entry.name == "test.txt"));
        assert!(entries.iter().any(|entry| entry.name == "nested"));
    }

    #[test]
    fn test_zipfs_globfs() {
        let dir = setup("test_zipfs_globfs");
        let archive = dir.join("archive.zip");
        write_zip(
            &archive,
            &[
                ("home/test.txt", b"zip payload"),
                ("home/other.log", b"log"),
            ],
        );

        let zipfs = ZipFs::new(archive).unwrap();
        let matches = zipfs.globfs(&inner("home"), "*.txt").unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].meta.kind, EntryKind::File);
    }

    #[test]
    fn test_zipfs_read_file_not_found() {
        let dir = setup("test_zipfs_read_file_not_found");
        let archive = dir.join("archive.zip");
        write_zip(&archive, &[("home/test.txt", b"zip payload")]);

        let zipfs = ZipFs::new(archive).unwrap();
        let err = zipfs.read_file(&inner("missing.txt"), None).unwrap_err();
        assert!(matches!(err, AccessorError::NotFound { .. }));
    }

    #[test]
    fn test_zipfs_reader_handle() {
        let dir = setup("test_zipfs_reader_handle");
        let archive = dir.join("archive.zip");
        write_zip(&archive, &[("home/test.txt", b"abcdef")]);

        let zipfs = ZipFs::new(archive).unwrap();
        let handle = FileHandle::new(FileLocator::Zip {
            archive: dir.join("archive.zip"),
            entry_index: 0,
            entry: String::from("home/test.txt"),
        });

        let mut reader = zipfs.reader_handle(&handle, None).unwrap();
        assert_eq!(reader.read_bytes(2, 2).unwrap(), b"cd");
    }

    #[test]
    fn test_zipfs_read_document() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/archives/document.odt");

        let zipfs = ZipFs::new(test_location).unwrap();

        let entries = zipfs.read_dir(&inner("")).unwrap();
        assert_eq!(entries.len(), 9);
        for entry in entries {
            if !entry.is_directory() || entry.name == "Configurations2" {
                continue;
            }
            let values = zipfs
                .read_dir_handle(entry.handle.as_directory().unwrap())
                .unwrap();
            assert!(!values.is_empty());
        }
    }

    #[test]
    fn test_zipfs_read_document_too_large() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/archives/document.odt");

        let zipfs = ZipFs::new(test_location).unwrap();
        let err = zipfs
            .read_file(&inner("content.xml"), Some(10))
            .unwrap_err();
        assert!(matches!(
            err,
            AccessorError::FileTooLarge {
                size: 4229,
                limit: 10
            }
        ));
    }
}
