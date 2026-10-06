use crate::{
    accessor::{
        config::AccessorConfig,
        disk::{
            format::{DiskFormat, DiskReader, disk_display_path, disk_root_display},
            identify::{FilesystemKind, IdentifiedPartition, identify_disk},
            inspect::{DiskPartition, inspect_disk},
        },
        entry::{
            handle::{
                DirEntry, DirHandle, EntryMeta, EntryStat, FileHandle, GlobMatch, ItemHandle,
                Timestamp,
            },
            locator::{DirLocator, DiskEntryRef, FileLocator},
        },
        error::{AccessorError, AccessorResult},
        filesystem::ntfs::{
            data::NtfsFs,
            volume::NtfsVolume,
            walk::{LabeledPath, PathLabel},
        },
        io::{
            partition::PartitionReader,
            reader::{AccessorReader, ReaderLocation, extension_from_filename},
        },
        location::path::InnerPath,
    },
    output::manager::OutputManager,
    structs::artifacts::os::files::FileOptions,
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
        // We will only read file on supported partitions
        let partitions = self.identified_partitions()?;

        let targets = supported_targets(&partitions, selected.as_deref())?;
        for partition in targets {
            // The first partition that matches our `InnerPath` is the only one we read
            match self.read_partition_file(partition, &filesystem_path) {
                Ok(result) => {
                    info!(
                        "Matched on partition {}. Filesystem: {:?}",
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

    /// Read a `FileHandle` on a disk image
    ///
    /// Typically obtained after globbing the filesystem or using `read_dir` or `read_dir_handle`
    pub(crate) fn read_file_handle(&self, handle: &FileHandle) -> AccessorResult<Vec<u8>> {
        let FileLocator::Disk {
            image,
            format,
            partition_id,
            filesystem_path,
            entry,
        } = &handle.locator
        else {
            return Err(AccessorError::invalid_handle(format!(
                "disk source cannot read file handle for {}",
                handle.display_path()
            )));
        };

        self.ensure_same_image(image, *format)?;
        let partition = self.partition(partition_id)?;
        self.read_referenced_file(&partition, filesystem_path, entry)
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
        // We will only read directory on supported partitions
        let partitions = self.identified_partitions()?;

        if selected.is_none() && filesystem_path.is_empty() {
            return Ok(self.image_root_entries(&partitions));
        }

        let targets = supported_targets(&partitions, selected.as_deref())?;
        for partition in targets {
            match self.read_partition_dir(partition, &filesystem_path) {
                Ok(results) => {
                    info!(
                        "Matched on partition {}. Filesystem: {:?}",
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

    /// Read a `DirHandle` on a disk image
    ///
    /// Typically obtained after globbing the filesystem or using `read_dir` or `read_dir_handle`
    pub(crate) fn read_dir_handle(&self, handle: &DirHandle) -> AccessorResult<Vec<DirEntry>> {
        let DirLocator::Disk {
            image,
            format,
            partition_id,
            filesystem_path,
            entry,
        } = &handle.locator
        else {
            return Err(AccessorError::invalid_handle(format!(
                "disk source cannot list directory handle for {}",
                handle.display_path()
            )));
        };

        self.ensure_same_image(image, *format)?;
        let partition = self.partition(partition_id)?;
        self.read_reference_dir(&partition, filesystem_path, entry)
    }

    /// Stat the first supported filesystem that contains the correct filepath (`InnerPath`).
    ///
    /// Example: `raw:/image.raw!/Windows/test.txt` returns the first match.
    /// If multiple partitions are on the image with the same path
    /// we return first one that matches
    ///
    /// User can provide a specific partition via `raw:/image.raw!Partition0:hello\\file.txt`
    pub(crate) fn stat(&self, inner: &InnerPath) -> AccessorResult<EntryStat> {
        let (selected, filesystem_path) = split_selector(inner);
        // We will only stat on supported partitions
        let partitions = self.identified_partitions()?;

        if selected.is_none() && filesystem_path.is_empty() {
            return Ok(self.image_root_stat());
        }

        let targets = supported_targets(&partitions, selected.as_deref())?;
        for partition in targets {
            match self.stat_partition_file(partition, &filesystem_path) {
                Ok(results) => {
                    info!(
                        "Matched on partition {}. Filesystem: {:?}",
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

    /// Stat a `FileHandle` on a disk image
    ///
    /// Typically obtained after globbing the filesystem or using `read_dir` or `read_dir_handle`
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

    /// Stat a `DirHandle` on a disk image
    ///
    /// Typically obtained after globbing the filesystem or using `read_dir` or `read_dir_handle`
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

    /// Glob all partitions contains the correct filepath (`InnerPath`) and pattern.
    ///
    /// Unlike other functions `globfs` will run on all partitions (unless a single partition is select)
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
            match self.glob_partition(partition, &filesystem_path, pattern) {
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

    /// Return an `AccessorReader` on the first supported filesystem that contains the correct filepath (`InnerPath`).
    ///
    /// Example: `raw:/image.raw!/Windows/test.txt` returns the first match.
    /// If multiple partitions are on the image with the same path
    /// we return first one that matches
    ///
    /// User can provide a specific partition via `raw:/image.raw!Partition0:hello\\file.txt`
    pub(crate) fn open_reader(&self, inner: &InnerPath) -> AccessorResult<AccessorReader> {
        let (selected, filesystem_path) = split_selector(inner);
        let partitions = self.identified_partitions()?;
        let targets = supported_targets(&partitions, selected.as_deref())?;

        for partition in targets {
            match self.open_partition_reader(partition, &filesystem_path) {
                Ok(reader) => {
                    info!(
                        "Matched on partition {}. Filesystem: {:?}",
                        partition.partition.id, partition.filesystem
                    );
                    return Ok(reader);
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

    /// Return an `AccessorReader` on the a `FileHandle` on a disk image
    ///
    /// Typically obtained after globbing the filesystem or using `read_dir` or `read_dir_handle`
    pub(crate) fn open_reader_handle(&self, handle: &FileHandle) -> AccessorResult<AccessorReader> {
        let FileLocator::Disk {
            image,
            format,
            partition_id,
            filesystem_path,
            entry,
        } = &handle.locator
        else {
            return Err(AccessorError::invalid_handle(format!(
                "disk source cannot open reader handle for {}",
                handle.display_path()
            )));
        };

        self.ensure_same_image(image, *format)?;
        let partition = self.partition(partition_id)?;

        self.open_referenced_reader(&partition, filesystem_path, entry)
    }

    /// Recursively walk the all supported partitions and filesystems on a disk image
    ///
    /// Similar to `globfs` this function runs against all partitions unless the user selects
    /// a specific partition to walk
    ///
    /// If the user provides a start path but no partition then the first match for that
    /// start path is the path we use
    pub(crate) fn walk(
        &self,
        inner: &InnerPath,
        options: &FileOptions,
        manager: &mut OutputManager,
        rule: &str,
        evidence: &str,
    ) -> AccessorResult<()> {
        let (selected, filesystem_path) = split_selector(inner);
        let partitions = self.identified_partitions()?;
        let targets = supported_targets(&partitions, selected.as_deref())?;

        let walk_all = selected.is_none() && is_filesystem_root(&filesystem_path);

        for partition in targets {
            match self.walk_partition(
                partition,
                &filesystem_path,
                options,
                manager,
                rule,
                evidence,
            ) {
                Ok(()) => {
                    info!(
                        "Matched on partition {}. Filesystem: {:?}",
                        partition.partition.id, partition.filesystem
                    );

                    // If we not walking all partitions
                    // Then we we are done now
                    if !walk_all {
                        return Ok(());
                    }
                }
                Err(AccessorError::NotFound { .. }) if walk_all || selected.is_none() => {}
                Err(AccessorError::NotFound { .. }) => {
                    return Err(AccessorError::not_found(inner.display()));
                }
                Err(err) => return Err(err),
            }
        }

        // If we walked all partitions then return now
        if walk_all {
            return Ok(());
        }

        Err(AccessorError::not_found(inner.display()))
    }

    /// Start walking the partition
    /// if we support parsing the filesystem
    fn walk_partition(
        &self,
        partition: &IdentifiedPartition,
        inner: &InnerPath,
        options: &FileOptions,
        manager: &mut OutputManager,
        rule: &str,
        evidence: &str,
    ) -> AccessorResult<()> {
        match partition.filesystem {
            FilesystemKind::Ntfs => {
                // Open the NTFS filesystem
                let filesystem = open_ntfs(self.open_disk()?, &partition.partition)?;
                let image = self.path.clone();
                let format = self.format;
                let partition_id = partition.partition.id.clone();

                // Create `PathLabel` to leverage when walking the NTFS filesystem
                let paths = PathLabel::custom(move |ntfs_display| {
                    let filesystem_path = ntfs_filesystem_path(ntfs_display);
                    LabeledPath {
                        directory: parent_path(&filesystem_path),
                        display_path: disk_display_path(
                            &image,
                            format,
                            &partition_id,
                            &filesystem_path,
                        ),
                        drive: partition_id.clone(),
                        full_path: filesystem_path,
                    }
                });

                // Use our NTFS accessor to walk the filesystem
                filesystem.walk_labeled(inner, options, manager, rule, evidence, paths)
            }
            FilesystemKind::Ext4 | FilesystemKind::Bitlocker | FilesystemKind::Unknown => {
                Err(unsupported(partition))
            }
        }
    }

    /// Find all partitions from provided disk image
    fn identified_partitions(&self) -> AccessorResult<Vec<IdentifiedPartition>> {
        let mut reader = self.open_disk()?;
        let layout = inspect_disk(&mut reader)?;

        identify_disk(&mut reader, &layout)
    }

    /// Read the filesystem file on the provided partition
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

    /// Read the filesystem directory on the provided partition
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

    /// Stat the filesystem file on the provided partition
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
                    self.format,
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

    /// Glob the filesystem on the provided partition
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

    /// Take each `DirEntry` value and append our disk info (`PartitionID`) so we know which
    /// partition is associated with the `ItemHandle` (`FileHandle` and `DirHandle`)
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
            disk_display_path(&self.path, self.format, partition_id, &filesystem_path);

        Ok(DirEntry::new(
            entry.name,
            handle,
            rewrite_meta(entry.meta, filesystem_path, display_path),
            entry.times,
        ))
    }

    /// Take each `GlobMatch` value and append our disk info (`PartitionID`) so we know which
    /// partition is associated with the `ItemHandle` (`FileHandle` and `DirHandle`)
    fn map_glob(&self, partition_id: &str, item: GlobMatch) -> AccessorResult<GlobMatch> {
        let entry = self.map_dir_entry(
            partition_id,
            DirEntry::new(String::new(), item.handle, item.meta, Timestamp::default()),
        )?;

        Ok(GlobMatch::new(entry.handle, entry.meta))
    }

    /// Return a Partition root `EntryStat` with default `Timestamp`
    fn image_root_stat(&self) -> EntryStat {
        let display_path = disk_root_display(&self.path, self.format);
        let filename = self
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_string();

        // Construct a mock/default `EntryStat` value when targeting the root of partitions
        EntryStat {
            meta: path_meta(EntryKind::Directory, 0, &filename, &display_path),
            times: Timestamp::default(),
        }
    }

    /// Return list of Partitions if a user wants us to read the root of the disk image
    fn image_root_entries(&self, partitions: &[IdentifiedPartition]) -> Vec<DirEntry> {
        partitions
            .iter()
            .map(|partition| {
                let id = partition.partition.id.clone();
                let display_path = disk_display_path(&self.path, self.format, &id, "");

                // Construct an `EntryMeta` value if we are listing partitions
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

                // No timestamps for the partitions
                DirEntry::new(id, handle, meta, Timestamp::default())
            })
            .collect()
    }

    /// Construct a `FileHandle` that can work on disk images
    ///
    /// We need to update the `FileHandle` returned from filesystem parser
    /// to include details on the Partition
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

    /// Construct a `DirHandle` that can work on disk images
    ///
    /// We need to update the `DirHandle` returned from filesystem parser
    /// to include details on the Partition
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

    /// Make sure callers always reference the same disk image
    fn ensure_same_image(&self, image: &PathBuf, format: DiskFormat) -> AccessorResult<()> {
        if image == &self.path && format == self.format {
            return Ok(());
        }

        Err(AccessorError::invalid_handle(
            "disk handle belongs to a different image",
        ))
    }

    /// Validate the partition provided to us can be found
    /// on the disk image
    fn partition(&self, partition_id: &str) -> AccessorResult<IdentifiedPartition> {
        let partitions = self.identified_partitions()?;

        let partition = partitions
            .into_iter()
            .find(|partition| partition.partition.id == partition_id)
            .ok_or_else(|| AccessorError::not_found(partition_id))?;

        require_supported(&partition)?;
        Ok(partition)
    }

    /// Stat the provided `FileHandle` or `DirHandle`
    ///
    /// This reuses the `stat_handle` code from filesystem accessor
    fn stat_referenced_entry(
        &self,
        partition: &IdentifiedPartition,
        filesystem_path: &str,
        entry: &DiskEntryRef,
    ) -> AccessorResult<EntryStat> {
        // Return the stat value for the root filesystem path
        if matches!(entry, DiskEntryRef::PartitionRoot) {
            return self.stat_partition_file(partition, &InnerPath::empty());
        }

        let stat = match (partition.filesystem, entry) {
            (FilesystemKind::Ntfs, DiskEntryRef::Ntfs(file_ref)) => {
                let filesystem = open_ntfs(self.open_disk()?, &partition.partition)?;

                // Construct the NTFS `FileHandle` via the `file_ref` value
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
            _ => {
                return Err(AccessorError::invalid_handle(format!(
                    "{} entry reference does not match {:?} filesystem",
                    partition.partition.id, partition.filesystem
                )));
            }
        };

        let display_path = disk_display_path(
            &self.path,
            self.format,
            &partition.partition.id,
            filesystem_path,
        );

        Ok(EntryStat {
            meta: rewrite_meta(stat.meta, filesystem_path.to_string(), display_path),
            times: stat.times,
        })
    }

    /// Read the provided `FileHandle`
    ///
    /// This reuses the `read_handle` code from filesystem accessor
    fn read_referenced_file(
        &self,
        partition: &IdentifiedPartition,
        filesystem_path: &str,
        entry: &DiskEntryRef,
    ) -> AccessorResult<Vec<u8>> {
        // Cannot read the literal partition
        if matches!(entry, DiskEntryRef::PartitionRoot) {
            return Err(AccessorError::not_a_file(disk_display_path(
                &self.path,
                self.format,
                &partition.partition.id,
                filesystem_path,
            )));
        }

        match (partition.filesystem, entry) {
            (FilesystemKind::Ntfs, DiskEntryRef::Ntfs(file_ref)) => {
                let filesystem = open_ntfs(self.open_disk()?, &partition.partition)?;

                // Construct the NTFS `FileHandle` via the `file_ref` value
                let handle = FileHandle::new(FileLocator::Ntfs {
                    drive: NTFS_DRIVE,
                    file_ref: file_ref.clone(),
                    display_path: filesystem_path.to_string(),
                });

                filesystem.read_handle(&handle, self.max_read_size)
            }
            (FilesystemKind::Ext4 | FilesystemKind::Bitlocker | FilesystemKind::Unknown, _) => {
                Err(unsupported(partition))
            }
            _ => Err(AccessorError::invalid_handle(format!(
                "{} entry reference does not match {:?} filesystem",
                partition.partition.id, partition.filesystem
            ))),
        }
    }

    /// Read the provided `DirHandle`
    ///
    /// This reuses the `read_dir_handle` code from filesystem accessor
    fn read_reference_dir(
        &self,
        partition: &IdentifiedPartition,
        filesystem_path: &str,
        entry: &DiskEntryRef,
    ) -> AccessorResult<Vec<DirEntry>> {
        if matches!(entry, DiskEntryRef::PartitionRoot) {
            return self.read_partition_dir(partition, &InnerPath::empty());
        }

        match (partition.filesystem, entry) {
            (FilesystemKind::Ntfs, DiskEntryRef::Ntfs(dir_ref)) => {
                let filesystem = open_ntfs(self.open_disk()?, &partition.partition)?;

                // Construct the NTFS `DirHandle` via the `file_ref` value
                let handle = DirHandle::new(DirLocator::Ntfs {
                    drive: NTFS_DRIVE,
                    dir_ref: dir_ref.clone(),
                    display_path: filesystem_path.to_string(),
                });

                filesystem
                    .read_dir_handle(&handle)?
                    .into_iter()
                    .map(|child| self.map_dir_entry(&partition.partition.id, child))
                    .collect()
            }
            (FilesystemKind::Ext4 | FilesystemKind::Bitlocker | FilesystemKind::Unknown, _) => {
                Err(unsupported(partition))
            }
            _ => Err(AccessorError::invalid_handle(format!(
                "{} entry reference does not match {:?} filesystem",
                partition.partition.id, partition.filesystem
            ))),
        }
    }

    /// Return an `AccessorReader` for the provided file path
    fn open_partition_reader(
        &self,
        partition: &IdentifiedPartition,
        inner: &InnerPath,
    ) -> AccessorResult<AccessorReader> {
        match partition.filesystem {
            FilesystemKind::Ntfs => {
                let filesystem = open_ntfs(self.open_disk()?, &partition.partition)?;
                let reader = filesystem.reader(inner)?;

                Ok(self.with_disk_location(reader, &partition.partition.id, &inner.display()))
            }
            FilesystemKind::Ext4 | FilesystemKind::Bitlocker | FilesystemKind::Unknown => {
                Err(unsupported(partition))
            }
        }
    }

    /// Return an `AccessorReader` for the provided `FileHandle`
    ///
    /// This reuses the `reader_handle` code from filesystem accessor
    fn open_referenced_reader(
        &self,
        partition: &IdentifiedPartition,
        filesystem_path: &str,
        entry: &DiskEntryRef,
    ) -> AccessorResult<AccessorReader> {
        // We cannot return a reader for the literal partition
        if matches!(entry, DiskEntryRef::PartitionRoot) {
            return Err(AccessorError::not_a_file(disk_display_path(
                &self.path,
                self.format,
                &partition.partition.id,
                filesystem_path,
            )));
        }

        match (partition.filesystem, entry) {
            (FilesystemKind::Ntfs, DiskEntryRef::Ntfs(file_ref)) => {
                let filesystem = open_ntfs(self.open_disk()?, &partition.partition)?;

                // Construct the NTFS `FileHandle` via the `file_ref` value
                let handle = FileHandle::new(FileLocator::Ntfs {
                    drive: NTFS_DRIVE,
                    file_ref: file_ref.clone(),
                    display_path: filesystem_path.to_string(),
                });

                let reader = filesystem.reader_handle(&handle)?;

                Ok(self.with_disk_location(reader, &partition.partition.id, filesystem_path))
            }
            (FilesystemKind::Ext4 | FilesystemKind::Bitlocker | FilesystemKind::Unknown, _) => {
                Err(unsupported(partition))
            }
            _ => Err(AccessorError::invalid_handle(format!(
                "{} entry reference does not match {:?} filesystem",
                partition.partition.id, partition.filesystem
            ))),
        }
    }

    /// Filesystem readers operate a the filesystem level
    ///
    /// We need to update the filesystem reader location to ensure it also handles
    /// the disk image and partition
    ///
    /// Useful for logging and debugging. This will show the file our `AccessorReader` uses
    fn with_disk_location(
        &self,
        mut reader: AccessorReader,
        partition_id: &str,
        filesystem_path: &str,
    ) -> AccessorReader {
        reader.location = ReaderLocation::from_display(disk_display_path(
            &self.path,
            self.format,
            partition_id,
            filesystem_path,
        ));

        reader
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

/// Return parent directory for a path
fn parent_path(path: &str) -> String {
    let trimmed = path.trim_end_matches(['\\', '/']);

    match trimmed.rfind(['\\', '/']) {
        Some(index) => trimmed[..index].to_string(),
        None => String::new(),
    }
}

/// Try to determine we are at root directory
fn is_filesystem_root(inner: &InnerPath) -> bool {
    inner.display().trim_matches(['\\', '/']).is_empty()
}

/// Update `EntryMeta` data to represent truthful paths
///
/// Mostly applies only to NTFS accessor. Since it expects
/// a drive letter.
///
/// However since we are reading disk images
/// we do not have driver letter
fn rewrite_meta(meta: EntryMeta, full_path: String, display_path: String) -> EntryMeta {
    path_meta_from(meta, &full_path, &display_path)
}

/// Ensure paths returned represent accurate paths
///
/// For example on NTFS the disk accessor uses the mock driver letter `X` for the NTFS accessor
///
/// When returning paths from accessing the disk, we make sure that is removed
///
/// This is only applied when reading directories, stat, or globbing
fn path_meta_from(mut meta: EntryMeta, full_path: &str, display_path: &str) -> EntryMeta {
    let filename = full_path
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(full_path)
        .to_string();

    // Use truthful paths when we are accessing files on disk
    meta.full_path = full_path.to_string();
    meta.display_path = display_path.to_string();
    meta.directory = parent_path(full_path);
    meta.extension = extension_from_filename(&filename);
    meta.filename = filename;

    meta
}

/// Construct a `EntryMeta` value for provided path
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
    use crate::{
        accessor::{
            config::AccessorConfig,
            disk::format::DiskFormat,
            entry::{
                handle::{DirHandle, FileHandle, ItemHandle},
                locator::{DirLocator, DiskEntryRef, FileLocator},
            },
            error::AccessorError,
            location::path::InnerPath,
        },
        output::manager::OutputManager,
        structs::{
            artifacts::os::files::FileOptions,
            toml::{OutputConfig, OutputDestination, OutputFormat},
        },
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

    #[test]
    fn test_read_file_handle() {
        let image = test_image();
        let source = source(&image);
        let entries = source.read_dir(&path("hello")).unwrap();
        let handle = entries
            .iter()
            .find(|entry| entry.name == "hello world.txt")
            .unwrap()
            .handle
            .as_file()
            .unwrap();

        let bytes = source.read_file_handle(handle).unwrap();
        assert_eq!(bytes, b"hello world\n");
    }

    #[test]
    fn test_read_dir_handle_partition_root() {
        let image = test_image();
        let source = source(&image);
        let root = source.read_dir(&path("")).unwrap();

        let handle = root[0].handle.as_directory().unwrap();
        let entries = source.read_dir_handle(handle).unwrap();
        let by_path = source.read_dir(&path("Partition0:")).unwrap();

        assert_eq!(entries.len(), by_path.len());
        assert!(entries.iter().any(|entry| entry.name == "hello"));
        assert!(
            entries
                .iter()
                .all(|entry| !entry.meta.display_path.contains("X:"))
        );
    }

    #[test]
    fn test_read_dir_handle_nested_directory() {
        let image = test_image();
        let source = source(&image);

        let root = source.read_dir(&path("Partition0:")).unwrap();
        let hello = root
            .iter()
            .find(|entry| entry.name == "hello")
            .unwrap()
            .handle
            .as_directory()
            .unwrap();

        let entries = source.read_dir_handle(hello).unwrap();
        let file = entries
            .iter()
            .find(|entry| entry.name == "hello world.txt")
            .unwrap();

        assert_eq!(file.meta.full_path, "hello\\hello world.txt");
        assert!(
            file.meta
                .display_path
                .contains("Partition0:hello\\hello world.txt")
        );

        assert!(!file.meta.display_path.contains("X:"));
        let bytes = source
            .read_file_handle(file.handle.as_file().unwrap())
            .unwrap();
        assert_eq!(bytes, b"hello world\n");
    }

    #[test]
    fn test_read_file_handle_rejects_partition_root() {
        let image = test_image();
        let source = source(&image);
        let handle = FileHandle::new(FileLocator::Disk {
            image: image.clone(),
            format: DiskFormat::Raw,
            partition_id: String::from("Partition0"),
            filesystem_path: String::new(),
            entry: DiskEntryRef::PartitionRoot,
        });

        let err = source.read_file_handle(&handle).unwrap_err();
        assert!(matches!(err, AccessorError::NotAFile { .. }));
    }

    #[test]
    fn test_open_reader() {
        let image = test_image();
        let source = source(&image);
        let mut reader = source.open_reader(&path("hello\\hello world.txt")).unwrap();
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).unwrap();

        assert_eq!(bytes, b"hello world\n");
        assert!(
            reader
                .location
                .display_path()
                .contains("Partition0:hello\\hello world.txt")
        );
        assert!(!reader.location.display_path().contains("X:"));
    }

    #[test]
    fn test_open_reader_handle() {
        let image = test_image();
        let source = source(&image);
        let entries = source.read_dir(&path("hello")).unwrap();
        let handle = entries
            .iter()
            .find(|entry| entry.name == "hello world.txt")
            .unwrap()
            .handle
            .as_file()
            .unwrap();

        let mut reader = source.open_reader_handle(handle).unwrap();
        let mut bytes = Vec::new();

        reader.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"hello world\n");
        assert_eq!(reader.location.display_path(), handle.display_path());
        assert!(!reader.location.display_path().contains("X:"));
    }

    #[test]
    fn test_open_reader_handle_rejects_partition_root() {
        let image = test_image();
        let source = source(&image);
        let handle = FileHandle::new(FileLocator::Disk {
            image: image.clone(),
            format: DiskFormat::Raw,
            partition_id: String::from("Partition0"),
            filesystem_path: String::new(),
            entry: DiskEntryRef::PartitionRoot,
        });

        let err = source.open_reader_handle(&handle).unwrap_err();
        assert!(matches!(err, AccessorError::NotAFile { .. }));
    }

    #[test]
    fn test_stat_file_handle() {
        let image = test_image();
        let source = source(&image);
        let entries = source.read_dir(&path("hello")).unwrap();

        let handle = entries
            .iter()
            .find(|entry| entry.name == "hello world.txt")
            .unwrap()
            .handle
            .as_file()
            .unwrap();

        let stat = source.stat_handle(handle).unwrap();
        let by_path = source.stat(&path("hello\\hello world.txt")).unwrap();

        assert_eq!(stat.meta.full_path, by_path.meta.full_path);
        assert_eq!(stat.meta.display_path, by_path.meta.display_path);
        assert_eq!(stat.meta.size, by_path.meta.size);

        assert_eq!(stat.times.modified, by_path.times.modified);
        assert!(stat.meta.display_path.contains("Partition0:"));
        assert!(!stat.meta.display_path.contains("X:"));
    }

    #[test]
    fn test_stat_dir_handle() {
        let image = test_image();
        let source = source(&image);
        let entries = source.read_dir(&path("Partition0:")).unwrap();

        let handle = entries
            .iter()
            .find(|entry| entry.name == "hello")
            .unwrap()
            .handle
            .as_directory()
            .unwrap();

        let stat = source.stat_dir_handle(handle).unwrap();
        let by_path = source.stat(&path("hello")).unwrap();

        assert_eq!(stat.meta.full_path, "hello");
        assert_eq!(stat.meta.display_path, by_path.meta.display_path);
        assert_eq!(stat.times.created, by_path.times.created);
        assert!(!stat.meta.display_path.contains("X:"));
    }

    #[test]
    fn test_stat_dir_handle_partition_root() {
        let image = test_image();
        let source = source(&image);
        let entries = source.read_dir(&path("")).unwrap();

        let handle = entries[0].handle.as_directory().unwrap();
        let stat = source.stat_dir_handle(handle).unwrap();
        let by_path = source.stat(&path("Partition0:")).unwrap();

        assert_eq!(stat.meta.display_path, by_path.meta.display_path);
        assert_eq!(stat.meta.kind, by_path.meta.kind);
        assert_eq!(stat.times.modified, by_path.times.modified);

        assert!(stat.meta.display_path.contains("!Partition0"));
        assert!(!stat.meta.display_path.contains("X:"));
    }

    #[test]
    fn test_stat_image_root_display_path() {
        let stat = source(&test_image()).stat(&path("")).unwrap();

        assert!(stat.meta.display_path.ends_with('!'));
        assert!(!stat.meta.display_path.contains("Partition0"));
        assert!(!stat.meta.display_path.contains("X:"));
    }

    #[test]
    fn test_stat_handle_rejects_other_image() {
        let image = test_image();
        let source = source(&image);
        let entries = source.read_dir(&path("hello")).unwrap();

        let FileLocator::Disk {
            partition_id,
            filesystem_path,
            entry,
            ..
        } = &entries
            .iter()
            .find(|entry| entry.name == "hello world.txt")
            .unwrap()
            .handle
            .as_file()
            .unwrap()
            .locator
        else {
            panic!("expected a disk file handle");
        };

        let foreign = FileHandle::new(FileLocator::Disk {
            image: PathBuf::from("other.raw"),
            format: DiskFormat::Raw,
            partition_id: partition_id.clone(),
            filesystem_path: filesystem_path.clone(),
            entry: entry.clone(),
        });

        let err = source.stat_handle(&foreign).unwrap_err();
        assert!(matches!(err, AccessorError::InvalidHandle { .. }));
    }

    #[test]
    fn test_walk_disk_paths() {
        let image = test_image();
        let source = source(&image);
        let options = FileOptions {
            start_path: String::new(),
            depth: Some(2),
            source: format!("raw:{}", image.display()),
            ..Default::default()
        };

        let config = OutputConfig {
            name: String::from("disk_walk"),
            endpoint_id: String::from("test"),
            directory: PathBuf::from("./tmp"),
            destination: OutputDestination::Local,
            format: OutputFormat::Jsonl,
            compress: false,
            ..Default::default()
        };
        let mut manager = OutputManager::new(config).unwrap();

        source
            .walk(&path(""), &options, &mut manager, "", "raw:test")
            .unwrap();
        let output_dir = PathBuf::from("./tmp/disk_walk");

        let mut jsonl = String::new();
        for entry in std::fs::read_dir(&output_dir).unwrap() {
            let file = entry.unwrap().path();
            let name = file.file_name().unwrap().to_string_lossy();
            if name.starts_with("files_raw_") && name.ends_with(".jsonl") {
                jsonl.push_str(&std::fs::read_to_string(&file).unwrap());
            }
        }

        assert!(jsonl.contains("hello world.txt"));
        assert!(jsonl.contains("hello\\\\hello world.txt"));
        assert!(jsonl.contains("\"drive\":\"X:\""));
        assert!(jsonl.contains("X:"));
        assert!(!jsonl.contains("ntfs:"));
    }
}
