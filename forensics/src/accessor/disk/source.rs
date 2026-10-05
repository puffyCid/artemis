use crate::accessor::{
    config::AccessorConfig,
    disk::{
        format::{DiskFormat, DiskReader},
        identify::{FilesystemKind, IdentifiedPartition, identify_disk},
        inspect::{DiskPartition, inspect_disk},
    },
    entry::{
        handle::{
            DirEntry, DirHandle, EntryMeta, EntryStat, FileHandle, GlobMatch, ItemHandle, Timestamp,
        },
        locator::{DirLocator, DiskEntryRef, FileLocator, disk_display_path, disk_root_display},
    },
    error::{AccessorError, AccessorResult},
    filesystem::ntfs::{data::NtfsFs, volume::NtfsVolume},
    io::{partition::PartitionReader, reader::extension_from_filename},
    location::path::InnerPath,
};
use common::files::EntryKind;
use std::{
    io::{Read, Seek},
    path::PathBuf,
};
use tracing::info;

/// The opened disk image
pub(crate) struct DiskSource {
    /// Format of the disk image
    format: DiskFormat,
    /// Path to the image
    path: PathBuf,
    /// Max file size we read into memory
    max_read_size: Option<u64>,
}

/// Drive letter used when opening NTFS volumes inside disk images
const NTFS_DRIVE: char = 'X';

impl DiskSource {
    /// Open a provided disk image file
    pub(crate) fn open(
        config: &AccessorConfig,
        format: DiskFormat,
        path: impl Into<PathBuf>,
    ) -> AccessorResult<Self> {
        let path = path.into();
        format.open_reader(&path)?;

        Ok(Self {
            format,
            path,
            max_read_size: config.max_read_size,
        })
    }

    /// The disk image container and get a `DiskReader`
    fn open_disk(&self) -> AccessorResult<DiskReader> {
        self.format.open_reader(&self.path)
    }

    /// Read the first supported filesystem that contains the correct filepath (`InnerPath`).
    ///
    /// Example: `raw:/image.raw!/Windows/test.txt` returns the first match.
    /// If multiple partitions are on the image with the same path
    /// we return first one that matches
    ///
    /// User can provide a specific partition via `raw:/image.raw!Partition0:hello\\file.txt`
    pub(crate) fn read_file(&self, inner: &InnerPath) -> AccessorResult<Vec<u8>> {
        let (selected, filesystem_path) = split_selector(inner);
        let partitions = self.identified_partitions()?;

        let targets = supported_targets(&partitions, selected.as_deref())?;
        for partition in targets {
            // The first partition that matches our `InnerPath` is the only one we read
            match self.read_partition_file(partition, &filesystem_path) {
                Ok(result) => {
                    info!(
                        "matched on partition {}. Filesystem: {:?}",
                        partition.partition.id, partition.filesystem
                    );

                    return Ok(result);
                }
                Err(AccessorError::NotFound { .. }) if selected.is_none() => {}
                Err(AccessorError::NotFound { .. }) => {
                    return Err(AccessorError::not_found(inner.display()));
                }
                Err(err) => return Err(err),
            }
        }

        Err(AccessorError::not_found(inner.display()))
    }

    /// Read the first supported filesystem that contains the correct directory (`InnerPath`).
    ///
    /// Example: `raw:/image.raw!/Windows/` returns the first match.
    /// If multiple partitions are on the image with the same path
    /// we return first one that matches
    ///
    /// User can provide a specific partition via `raw:/image.raw!Partition0:hello\\`
    pub(crate) fn read_dir(&self, inner: &InnerPath) -> AccessorResult<Vec<DirEntry>> {
        let (selected, filesystem_path) = split_selector(inner);
        let partitions = self.identified_partitions()?;

        if selected.is_none() && filesystem_path.is_empty() {
            return Ok(self.image_root_entries(&partitions));
        }

        let targets = supported_targets(&partitions, selected.as_deref())?;
        for partition in targets {
            match self.read_partition_dir(partition, &filesystem_path) {
                Ok(results) => {
                    info!(
                        "matched on partition {}. Filesystem: {:?}",
                        partition.partition.id, partition.filesystem
                    );

                    return Ok(results);
                }
                Err(AccessorError::NotFound { .. }) if selected.is_none() => {}
                Err(AccessorError::NotFound { .. }) => {
                    return Err(AccessorError::not_found(inner.display()));
                }
                Err(err) => return Err(err),
            }
        }

        Err(AccessorError::not_found(inner.display()))
    }

    pub(crate) fn stat(&self, inner: &InnerPath) -> AccessorResult<EntryStat> {
        let (selected, filesystem_path) = split_selector(inner);
        let partitions = self.identified_partitions()?;

        if selected.is_none() && filesystem_path.is_empty() {
            return Ok(self.image_root_stat());
        }

        let targets = supported_targets(&partitions, selected.as_deref())?;
        for partition in targets {
            match self.stat_partition_file(partition, &filesystem_path) {
                Ok(results) => {
                    info!(
                        "matched on partition {}. Filesystem: {:?}",
                        partition.partition.id, partition.filesystem
                    );

                    return Ok(results);
                }
                Err(AccessorError::NotFound { .. }) if selected.is_none() => {}
                Err(AccessorError::NotFound { .. }) => {
                    return Err(AccessorError::not_found(inner.display()));
                }
                Err(err) => return Err(err),
            }
        }

        Err(AccessorError::not_found(inner.display()))
    }

    pub(crate) fn stat_handle(&self, handle: &FileHandle) -> AccessorResult<EntryStat> {
        let FileLocator::Disk {
            image,
            format,
            partition_id,
            filesystem_path,
            entry,
        } = &handle.locator
        else {
            return Err(AccessorError::invalid_handle(format!(
                "disk source cannot stat file handle for {}",
                handle.display_path()
            )));
        };

        self.ensure_same_image(image, *format)?;

        let partition = self.partition(partition_id)?;
        self.stat_referenced_entry(&partition, filesystem_path, entry)
    }

    pub(crate) fn stat_dir_handle(&self, handle: &DirHandle) -> AccessorResult<EntryStat> {
        let DirLocator::Disk {
            image,
            format,
            partition_id,
            filesystem_path,
            entry,
        } = &handle.locator
        else {
            return Err(AccessorError::invalid_handle(format!(
                "disk source cannot stat directory handle for {}",
                handle.display_path()
            )));
        };

        self.ensure_same_image(image, *format)?;

        let partition = self.partition(partition_id)?;
        self.stat_referenced_entry(&partition, filesystem_path, entry)
    }

    pub(crate) fn globfs(
        &self,
        directory: &InnerPath,
        pattern: &str,
    ) -> AccessorResult<Vec<GlobMatch>> {
        let (selected, filesystem_path) = split_selector(directory);
        let partitions = self.identified_partitions()?;
        let targets = supported_targets(&partitions, selected.as_deref())?;

        let mut matches = Vec::new();

        for partition in targets {
            match self.glob_partition(&partition, &filesystem_path, pattern) {
                Ok(found) => matches.extend(found),
                Err(AccessorError::NotFound { .. }) if selected.is_none() => {}
                Err(AccessorError::NotFound { .. }) => {
                    return Err(AccessorError::not_found(directory.display()));
                }
                Err(err) => return Err(err),
            }
        }

        Ok(matches)
    }

    /// Find all partitions from provided disk image
    fn identified_partitions(&self) -> AccessorResult<Vec<IdentifiedPartition>> {
        let mut reader = self.open_disk()?;
        let layout = inspect_disk(&mut reader)?;

        identify_disk(&mut reader, &layout)
    }

    /// Read the filesystem on the provided partition
    fn read_partition_file(
        &self,
        partition: &IdentifiedPartition,
        inner: &InnerPath,
    ) -> AccessorResult<Vec<u8>> {
        match partition.filesystem {
            FilesystemKind::Ntfs => {
                let filesystem = open_ntfs(self.open_disk()?, &partition.partition)?;
                filesystem.read_file(inner, self.max_read_size)
            }
            FilesystemKind::Ext4 | FilesystemKind::Bitlocker | FilesystemKind::Unknown => {
                Err(unsupported(partition))
            }
        }
    }

    fn read_partition_dir(
        &self,
        partition: &IdentifiedPartition,
        inner: &InnerPath,
    ) -> AccessorResult<Vec<DirEntry>> {
        match partition.filesystem {
            FilesystemKind::Ntfs => {
                let filesystem = open_ntfs(self.open_disk()?, &partition.partition)?;
                let entries = filesystem.read_dir(inner)?;
                entries
                    .into_iter()
                    .map(|entry| self.map_dir_entry(&partition.partition.id, entry))
                    .collect()
            }
            FilesystemKind::Ext4 | FilesystemKind::Bitlocker | FilesystemKind::Unknown => {
                Err(unsupported(partition))
            }
        }
    }

    fn stat_partition_file(
        &self,
        partition: &IdentifiedPartition,
        inner: &InnerPath,
    ) -> AccessorResult<EntryStat> {
        match partition.filesystem {
            FilesystemKind::Ntfs => {
                let filesystem = open_ntfs(self.open_disk()?, &partition.partition)?;
                let stat = filesystem.stat(inner)?;
                let filesystem_path = ntfs_filesystem_path(&stat.meta.full_path);

                let display_path = disk_display_path(
                    &self.path,
                    &self.format,
                    &partition.partition.id,
                    &filesystem_path,
                );

                Ok(EntryStat {
                    meta: rewrite_meta(stat.meta, filesystem_path, display_path),
                    times: stat.times,
                })
            }
            FilesystemKind::Ext4 | FilesystemKind::Bitlocker | FilesystemKind::Unknown => {
                Err(unsupported(partition))
            }
        }
    }

    fn glob_partition(
        &self,
        partition: &IdentifiedPartition,
        inner: &InnerPath,
        pattern: &str,
    ) -> AccessorResult<Vec<GlobMatch>> {
        match partition.filesystem {
            FilesystemKind::Ntfs => {
                let filesystem = open_ntfs(self.open_disk()?, &partition.partition)?;
                let found = filesystem.globfs(inner, pattern)?;

                found
                    .into_iter()
                    .map(|item| self.map_glob(&partition.partition.id, item))
                    .collect()
            }
            FilesystemKind::Ext4 | FilesystemKind::Bitlocker | FilesystemKind::Unknown => {
                Err(unsupported(partition))
            }
        }
    }

    fn map_dir_entry(&self, partition_id: &str, entry: DirEntry) -> AccessorResult<DirEntry> {
        let (handle, filesystem_path) = match entry.handle {
            ItemHandle::File(handle) => {
                let (handle, filesystem_path) = self.disk_file_handle(partition_id, handle)?;
                (ItemHandle::File(handle), filesystem_path)
            }
            ItemHandle::Unsupported(handle) => {
                let (handle, filesystem_path) = self.disk_file_handle(partition_id, handle)?;
                (ItemHandle::Unsupported(handle), filesystem_path)
            }
            ItemHandle::Directory(handle) => {
                let (handle, filesystem_path) = self.disk_dir_handle(partition_id, handle)?;
                (ItemHandle::Directory(handle), filesystem_path)
            }
        };

        let display_path =
            disk_display_path(&self.path, &self.format, partition_id, &filesystem_path);

        Ok(DirEntry::new(
            entry.name,
            handle,
            rewrite_meta(entry.meta, filesystem_path, display_path),
            entry.times,
        ))
    }

    fn map_glob(&self, partition_id: &str, item: GlobMatch) -> AccessorResult<GlobMatch> {
        let entry = self.map_dir_entry(
            partition_id,
            DirEntry::new(String::new(), item.handle, item.meta, Timestamp::default()),
        )?;

        Ok(GlobMatch::new(entry.handle, entry.meta))
    }

    fn image_root_stat(&self) -> EntryStat {
        let display_path = disk_root_display(&self.path, &self.format);
        let filename = self
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_string();

        EntryStat {
            meta: path_meta(EntryKind::Directory, 0, &filename, &display_path),
            times: Timestamp::default(),
        }
    }

    fn image_root_entries(&self, partitions: &[IdentifiedPartition]) -> Vec<DirEntry> {
        partitions
            .iter()
            .map(|partition| {
                let id = partition.partition.id.clone();
                let display_path = disk_display_path(&self.path, &self.format, &id, "");

                let meta = path_meta(
                    EntryKind::Directory,
                    partition.partition.byte_length,
                    &id,
                    &display_path,
                );

                let handle = ItemHandle::Directory(DirHandle::new(DirLocator::Disk {
                    image: self.path.clone(),
                    format: self.format,
                    partition_id: id.clone(),
                    filesystem_path: String::new(),
                    entry: DiskEntryRef::PartitionRoot,
                }));
                DirEntry::new(id, handle, meta, Timestamp::default())
            })
            .collect()
    }

    fn disk_file_handle(
        &self,
        partition_id: &str,
        handle: FileHandle,
    ) -> AccessorResult<(FileHandle, String)> {
        match handle.locator {
            FileLocator::Ntfs {
                file_ref,
                display_path,
                ..
            } => {
                let filesystem_path = ntfs_filesystem_path(&display_path);
                let locator = FileLocator::Disk {
                    image: self.path.clone(),
                    format: self.format,
                    partition_id: partition_id.to_string(),
                    filesystem_path: filesystem_path.clone(),
                    entry: DiskEntryRef::Ntfs(file_ref),
                };
                Ok((FileHandle::new(locator), filesystem_path))
            }
            _ => Err(AccessorError::invalid_handle(
                "Filesystem listing returned an unexpected file handle",
            )),
        }
    }

    fn disk_dir_handle(
        &self,
        partition_id: &str,
        handle: DirHandle,
    ) -> AccessorResult<(DirHandle, String)> {
        match handle.locator {
            DirLocator::Ntfs {
                dir_ref,
                display_path,
                ..
            } => {
                let filesystem_path = ntfs_filesystem_path(&display_path);
                let locator = DirLocator::Disk {
                    image: self.path.clone(),
                    format: self.format,
                    partition_id: partition_id.to_string(),
                    filesystem_path: filesystem_path.clone(),
                    entry: DiskEntryRef::Ntfs(dir_ref),
                };
                Ok((DirHandle::new(locator), filesystem_path))
            }
            _ => Err(AccessorError::invalid_handle(
                "Filesystem listing returned an unexpected directory handle",
            )),
        }
    }

    fn ensure_same_image(&self, image: &PathBuf, format: DiskFormat) -> AccessorResult<()> {
        if image == &self.path && format == self.format {
            return Ok(());
        }
        Err(AccessorError::invalid_handle(
            "disk handle belongs to a different image",
        ))
    }

    fn partition(&self, partition_id: &str) -> AccessorResult<IdentifiedPartition> {
        let partitions = self.identified_partitions()?;
        let partition = partitions
            .into_iter()
            .find(|partition| partition.partition.id == partition_id)
            .ok_or_else(|| AccessorError::not_found(partition_id))?;

        require_supported(&partition)?;
        Ok(partition)
    }

    fn stat_referenced_entry(
        &self,
        partition: &IdentifiedPartition,
        filesystem_path: &str,
        entry: &DiskEntryRef,
    ) -> AccessorResult<EntryStat> {
        if matches!(entry, DiskEntryRef::PartitionRoot) {
            return self.stat_partition_file(partition, &InnerPath::empty());
        }

        let stat = match (partition.filesystem, entry) {
            (FilesystemKind::Ntfs, DiskEntryRef::Ntfs(file_ref)) => {
                let filesystem = open_ntfs(self.open_disk()?, &partition.partition)?;
                let handle = FileHandle::new(FileLocator::Ntfs {
                    drive: NTFS_DRIVE,
                    file_ref: file_ref.clone(),
                    display_path: filesystem_path.to_string(),
                });
                filesystem.stat_handle(&handle)?
            }
            (FilesystemKind::Ext4 | FilesystemKind::Bitlocker | FilesystemKind::Unknown, _) => {
                return Err(unsupported(partition));
            }
            #[allow(unreachable_patterns)]
            _ => {
                return Err(AccessorError::invalid_handle(format!(
                    "{} entry reference does not match {:?} filesystem",
                    partition.partition.id, partition.filesystem
                )));
            }
        };

        let display_path = disk_display_path(
            &self.path,
            &self.format,
            &partition.partition.id,
            filesystem_path,
        );

        Ok(EntryStat {
            meta: rewrite_meta(stat.meta, filesystem_path.to_string(), display_path),
            times: stat.times,
        })
    }
}

/// When auto-choosing the partition
/// make sure we only try supported filesystems
fn supported_targets<'a>(
    partitions: &'a [IdentifiedPartition],
    selected: Option<&str>,
) -> AccessorResult<Vec<&'a IdentifiedPartition>> {
    // If user provided an explicit partition. We try that first
    if let Some(id) = selected {
        let partition = partitions
            .iter()
            .find(|partition| partition.partition.id == id)
            .ok_or_else(|| AccessorError::not_found(id))?;

        return Ok(vec![require_supported(partition)?]);
    }

    // Identify all support partition filesystems
    let supported = partitions
        .iter()
        .filter(|partition| is_supported(partition.filesystem))
        .collect::<Vec<_>>();

    if supported.is_empty() {
        return Err(AccessorError::volume("Image has no supported filesystem"));
    }

    Ok(supported)
}

/// Open the NTFS filesystem
fn open_ntfs<R: Read + Seek + Send + 'static>(
    reader: R,
    partition: &DiskPartition,
) -> AccessorResult<NtfsFs<PartitionReader<R>>> {
    let volume = NtfsVolume::open_partition(
        reader,
        partition.byte_offset,
        partition.byte_length,
        partition.id.clone(),
    )?;

    Ok(NtfsFs::new(volume, NTFS_DRIVE))
}

/// Check to make sure we support the selected partition
fn require_supported(partition: &IdentifiedPartition) -> AccessorResult<&IdentifiedPartition> {
    if is_supported(partition.filesystem) {
        return Ok(partition);
    }

    Err(unsupported(partition))
}

/// Any unsupported partition we try to access is an `AccessorError`
fn unsupported(partition: &IdentifiedPartition) -> AccessorError {
    AccessorError::volume(format!(
        "{} is {:?}, which is not supported",
        partition.partition.id, partition.filesystem
    ))
}

/// Filesystems we current support on partitions
fn is_supported(kind: FilesystemKind) -> bool {
    match kind {
        FilesystemKind::Ntfs => true,
        FilesystemKind::Bitlocker | FilesystemKind::Ext4 | FilesystemKind::Unknown => false,
    }
}

/// Split the partition label if provided
///
/// We only accept partition with label `Partition`
fn split_selector(inner: &InnerPath) -> (Option<String>, InnerPath) {
    let display = inner.display();
    let Some(remaining) = display.strip_prefix("Partition") else {
        return (None, inner.clone());
    };

    let Some((index, path)) = remaining.split_once(':') else {
        return (None, inner.clone());
    };

    if index.is_empty() || !index.bytes().all(|byte| byte.is_ascii_digit()) {
        return (None, inner.clone());
    }

    let filesystem = path.trim_start_matches(['\\', '/']);
    (
        Some(format!("Partition{index}")),
        InnerPath::new(PathBuf::from(filesystem)),
    )
}

fn parent_path(path: &str) -> String {
    let trimmed = path.trim_end_matches(['\\', '/']);

    match trimmed.rfind(['\\', '/']) {
        Some(index) => trimmed[..index].to_string(),
        None => String::new(),
    }
}

fn rewrite_meta(meta: EntryMeta, full_path: String, display_path: String) -> EntryMeta {
    path_meta_from(meta, &full_path, &display_path)
}

fn path_meta_from(mut meta: EntryMeta, full_path: &str, display_path: &str) -> EntryMeta {
    let filename = full_path
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(full_path)
        .to_string();

    meta.full_path = full_path.to_string();
    meta.display_path = display_path.to_string();
    meta.directory = parent_path(full_path);
    meta.extension = extension_from_filename(&filename);
    meta.filename = filename;

    meta
}

fn path_meta(kind: EntryKind, size: u64, full_path: &str, display_path: &str) -> EntryMeta {
    path_meta_from(
        EntryMeta::new(kind, size, display_path),
        full_path,
        display_path,
    )
}

/// Convert `ntfs:X:\hello\file.txt` or `X:\hello\file.txt` to `hello\file.txt`.
fn ntfs_filesystem_path(display_path: &str) -> String {
    let path = display_path.strip_prefix("ntfs:").unwrap_or(display_path);
    let drive = format!("{NTFS_DRIVE}:");
    let path = path.strip_prefix(&drive).unwrap_or(path);
    path.trim_start_matches(['\\', '/']).to_string()
}

#[cfg(test)]
mod tests {
    use super::DiskSource;
    use crate::accessor::{
        config::AccessorConfig,
        disk::format::DiskFormat,
        entry::{
            handle::{DirHandle, ItemHandle},
            locator::{DirLocator, DiskEntryRef},
        },
        error::AccessorError,
        location::path::InnerPath,
    };
    use std::path::PathBuf;

    fn test_image() -> PathBuf {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/filesystems/ntfs/test.raw");
        path
    }

    fn source(path: &PathBuf) -> DiskSource {
        DiskSource::open(&AccessorConfig::default(), DiskFormat::Raw, path).unwrap()
    }

    fn path(value: &str) -> InnerPath {
        InnerPath::new(PathBuf::from(value))
    }

    #[test]
    fn test_read_file_logical_ntfs() {
        let bytes = source(&test_image())
            .read_file(&path("hello\\hello world.txt"))
            .unwrap();

        assert_eq!(bytes, b"hello world\n");
    }

    #[test]
    fn test_read_file_explicit_partition() {
        let bytes = source(&test_image())
            .read_file(&path("Partition0:hello\\hello world.txt"))
            .unwrap();

        assert_eq!(bytes, b"hello world\n");
    }

    #[test]
    fn test_read_file_partition_name_without_colon() {
        let err = source(&test_image())
            .read_file(&path("Partition0hello\\hello world.txt"))
            .unwrap_err();

        assert!(matches!(err, AccessorError::NotFound { .. }));
    }

    #[test]
    fn test_read_file_missing_partition() {
        let err = source(&test_image())
            .read_file(&path("Partition3:hello\\hello world.txt"))
            .unwrap_err();

        assert!(matches!(
            err,
            AccessorError::NotFound { path } if path == "Partition3"
        ));
    }

    #[test]
    fn test_read_file_directory_is_not_a_file() {
        let err = source(&test_image()).read_file(&path("hello")).unwrap_err();
        match err {
            AccessorError::NotAFile { path } => assert!(!path.contains("X:")),
            other => panic!("expected NotAFile, got {other:?}"),
        }
    }

    #[test]
    fn test_read_dir_image_root() {
        let entries = source(&test_image()).read_dir(&path("")).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "Partition0");

        assert!(entries[0].meta.display_path.contains("!Partition0"));
        assert!(!entries[0].meta.display_path.contains("X:"));
        assert!(matches!(
            &entries[0].handle,
            ItemHandle::Directory(DirHandle {
                locator: DirLocator::Disk {
                    entry: DiskEntryRef::PartitionRoot,
                    ..
                }
            })
        ));
    }
    #[test]
    fn test_read_dir_ntfs_root() {
        let entries = source(&test_image())
            .read_dir(&path("Partition0:"))
            .unwrap();

        assert!(entries.iter().any(|entry| entry.name == "hello"));
        assert!(
            entries
                .iter()
                .all(|entry| !entry.meta.display_path.contains("X:"))
        );
    }

    #[test]
    fn test_stat_file() {
        let stat = source(&test_image())
            .stat(&path("hello\\hello world.txt"))
            .unwrap();

        assert_eq!(stat.meta.full_path, "hello\\hello world.txt");
        assert!(
            stat.meta
                .display_path
                .contains("Partition0:hello\\hello world.txt")
        );
        assert!(!stat.meta.display_path.contains("X:"));
        assert_eq!(stat.meta.size, 12);
    }

    #[test]
    fn test_glob_all_ntfs_partitions() {
        let found = source(&test_image())
            .globfs(&path("hello"), "*.txt")
            .unwrap();

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].meta.full_path, "hello\\hello world.txt");
        assert!(found[0].meta.display_path.contains("Partition0:"));
        assert!(!found[0].meta.display_path.contains("X:"));
    }
}
