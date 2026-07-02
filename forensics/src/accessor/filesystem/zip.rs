use std::{collections::BTreeMap, fs::File, io::Read, path::PathBuf};

use glob::Pattern;
use zip::ZipArchive;

use crate::accessor::{
    entry::{
        handle::{DirEntry, DirHandle, EntryKind, EntryMeta, FileHandle, GlobMatch, ItemHandle},
        locator::{DirLocator, FileLocator},
    },
    error::{AccessorError, AccessorResult},
    io::reader::AccessorReader,
    location::path::InnerPath,
};

#[derive(Debug, Clone)]
struct ZipEntryRecord {
    index: usize,
    path: String,
    is_dir: bool,
    size: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct ZipIndex {
    archive_path: PathBuf,
    entries: Vec<ZipEntryRecord>,
    file_paths: BTreeMap<String, usize>,
}

impl ZipIndex {
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

        let mut entries = Vec::with_capacity(archive.len());
        let mut file_paths = BTreeMap::new();

        for index in 0..archive.len() {
            let entry = archive
                .by_index(index)
                .map_err(|err| AccessorError::zip(archive_path.clone(), err.to_string()))?;
            let path = ZipFs::normalize_zip_path(entry.name());
            let is_dir = entry.is_dir();
            let size = entry.size();

            if !is_dir && !path.is_empty() {
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

    fn record_at(&self, path: &str) -> Option<&ZipEntryRecord> {
        self.file_paths
            .get(path)
            .and_then(|position| self.entries.get(*position))
    }

    fn dir_entry_index(&self, prefix: &str) -> u32 {
        self.entries
            .iter()
            .find(|entry| entry.is_dir && entry.path == prefix)
            .map(|entry| entry.index as u32)
            .unwrap_or(0)
    }
}

pub(crate) struct ZipFs {
    index: ZipIndex,
}

impl ZipFs {
    pub(crate) fn new(index: ZipIndex) -> Self {
        Self { index }
    }

    pub(crate) fn normalize_zip_path(path: &str) -> String {
        path.replace('\\', "/")
            .trim_start_matches('/')
            .trim_end_matches('/')
            .to_string()
    }

    pub(crate) fn read_file(
        &self,
        inner: &InnerPath,
        max_read_size: Option<u64>,
    ) -> AccessorResult<Vec<u8>> {
        let path = Self::inner_to_prefix(inner);
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

    pub(crate) fn read_dir(&self, inner: &InnerPath) -> AccessorResult<Vec<DirEntry>> {
        self.read_dir_inner(inner)
    }

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

    pub(crate) fn globfs(
        &self,
        directory: &InnerPath,
        pattern: &str,
    ) -> AccessorResult<Vec<GlobMatch>> {
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

    pub(crate) fn reader(&self, inner: &InnerPath) -> AccessorResult<AccessorReader> {
        let path = Self::inner_to_prefix(inner);
        let record = self
            .index
            .record_at(&path)
            .ok_or_else(|| AccessorError::not_found(self.display_entry_path(&path)))?;
        if record.is_dir {
            return Err(AccessorError::not_a_file(self.display_entry_path(&path)));
        }
        let bytes = self.read_entry_bytes(record.index)?;
        Ok(AccessorReader::memory(bytes))
    }

    pub(crate) fn reader_handle(&self, handle: &FileHandle) -> AccessorResult<AccessorReader> {
        match &handle.locator {
            FileLocator::Zip { archive, .. } if archive == &self.index.archive_path => {
                let bytes = self.read_handle(handle, None)?;
                Ok(AccessorReader::memory(bytes))
            }
            _ => Err(AccessorError::invalid_handle(format!(
                "zip source cannot open reader handle for {}",
                handle.display_path()
            ))),
        }
    }

    fn inner_to_prefix(inner: &InnerPath) -> String {
        if inner.is_empty() {
            String::new()
        } else {
            Self::normalize_zip_path(&inner.display())
        }
    }

    fn display_entry_path(&self, entry_path: &str) -> String {
        if entry_path.is_empty() {
            format!("zip:{}", self.index.archive_path.display())
        } else {
            format!("zip:{}!{entry_path}", self.index.archive_path.display())
        }
    }

    fn read_entry_bytes(&self, index: usize) -> AccessorResult<Vec<u8>> {
        let file = File::open(&self.index.archive_path)
            .map_err(|err| AccessorError::io_path(&self.index.archive_path, err))?;
        let mut archive = ZipArchive::new(file)
            .map_err(|err| AccessorError::zip(self.index.archive_path.clone(), err.to_string()))?;

        let mut entry = archive
            .by_index(index)
            .map_err(|err| AccessorError::zip(self.index.archive_path.clone(), err.to_string()))?;
        let mut buf = Vec::new();
        entry
            .read_to_end(&mut buf)
            .map_err(|err| AccessorError::zip(self.index.archive_path.clone(), err.to_string()))?;
        Ok(buf)
    }

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

    fn list_children(&self, prefix: &str) -> AccessorResult<BTreeMap<String, ZipChild>> {
        let prefix = Self::normalize_zip_path(prefix);
        let prefix_with_slash = if prefix.is_empty() {
            String::new()
        } else {
            format!("{prefix}/")
        };

        let mut children = BTreeMap::<String, ZipChild>::new();

        for record in &self.index.entries {
            let entry_path = &record.path;
            if entry_path.is_empty() {
                continue;
            }
            let remainder = if prefix.is_empty() {
                entry_path.as_str()
            } else if *entry_path == prefix {
                continue;
            } else if let Some(rest) = entry_path.strip_prefix(&prefix_with_slash) {
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

#[derive(Debug, Clone)]
enum ZipChild {
    File {
        record: ZipEntryRecord,
        name: String,
    },
    Directory {
        prefix: String,
        name: String,
    },
}

impl ZipChild {
    fn file(record: ZipEntryRecord, name: String) -> Self {
        Self::File { record, name }
    }
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
        entry::handle::{EntryKind, FileHandle},
        error::AccessorError,
        filesystem::zip::{ZipFs, ZipIndex},
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
        let index = ZipIndex::open(archive.clone()).unwrap();
        let zipfs = ZipFs::new(index);
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
        let index = ZipIndex::open(archive).unwrap();
        let zipfs = ZipFs::new(index);
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
        let index = ZipIndex::open(archive).unwrap();
        let zipfs = ZipFs::new(index);
        let matches = zipfs.globfs(&inner("home"), "*.txt").unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].meta.kind, EntryKind::File);
    }
    #[test]
    fn test_zipfs_read_file_not_found() {
        let dir = setup("test_zipfs_read_file_not_found");
        let archive = dir.join("archive.zip");
        write_zip(&archive, &[("home/test.txt", b"zip payload")]);
        let index = ZipIndex::open(archive).unwrap();
        let zipfs = ZipFs::new(index);
        let err = zipfs.read_file(&inner("missing.txt"), None).unwrap_err();
        assert!(matches!(err, AccessorError::NotFound { .. }));
    }
    #[test]
    fn test_zipfs_reader_handle() {
        let dir = setup("test_zipfs_reader_handle");
        let archive = dir.join("archive.zip");
        write_zip(&archive, &[("home/test.txt", b"abcdef")]);
        let index = ZipIndex::open(archive).unwrap();
        let zipfs = ZipFs::new(index);
        let handle = FileHandle::new(crate::accessor::entry::locator::FileLocator::Zip {
            archive: dir.join("archive.zip"),
            entry_index: 0,
            entry: String::from("home/test.txt"),
        });
        let mut reader = zipfs.reader_handle(&handle).unwrap();
        assert_eq!(reader.read_bytes(2, 2).unwrap(), b"cd");
    }
}
