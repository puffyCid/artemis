use crate::{
    accessor::{
        entry::{
            handle::{
                DirEntry, DirHandle, EntryMeta, EntryStat, FileHandle, ItemHandle, Timestamp,
            },
            locator::{DirLocator, Ext4EntryRef, FileLocator},
        },
        error::{AccessorError, AccessorResult},
        filesystem::{
            ext4::{
                data::read_file_at,
                directory::{read_directory, read_symlink, resolve_path},
                format::{
                    inode::{Inode, InodeTime},
                    superblock::ext4_error,
                },
                reader::Ext4FileReader,
                volume::Ext4Volume,
            },
            helper::attributes::unix_attributes,
        },
        io::reader::{AccessorReader, ReaderLocation},
        location::path::InnerPath,
    },
    utils::{strings::extract_utf8_string_lossy, time::unixepoch_to_iso_with_nano},
};
use common::files::EntryKind;
use std::{
    io::{Read, Seek},
    sync::Arc,
};

/// Filesystem operations for an opened ext4 filesystem
pub(crate) struct Ext4Fs<R: Read + Seek + Send> {
    /// Reader that points to an ext4 volume
    volume: Arc<Ext4Volume<R>>,
}

impl<R: Read + Seek + Send + 'static> Ext4Fs<R> {
    /// Return an accessor to the ext4 filesystem
    pub(crate) fn new(volume: Ext4Volume<R>) -> Self {
        Self {
            volume: Arc::new(volume),
        }
    }

    /// Read a file into memory
    ///
    /// This will also read the target path that symlink file entries point to.
    /// It does **NOT** follow the symlink
    ///
    /// Will return an error for any file larger than `max_read_size`
    pub(crate) fn read_file(
        &self,
        inner: &InnerPath,
        max_read_size: Option<u64>,
    ) -> AccessorResult<Vec<u8>> {
        let path = filesystem_path(inner);
        let (inode_number, inode) = resolve_path(&self.volume, &path)?;
        self.read_inode_bytes(&path, inode_number, &inode, max_read_size)
    }

    /// Read a file or symlink from a `FileHandle`
    ///
    /// It will not follow the symlink. It just returns the symlink bytes
    pub(crate) fn read_handle(
        &self,
        handle: &FileHandle,
        max_read_size: Option<u64>,
    ) -> AccessorResult<Vec<u8>> {
        let FileLocator::Ext4 {
            label,
            entry,
            display_path,
        } = &handle.locator
        else {
            return Err(AccessorError::invalid_handle(format!(
                "ext4 source cannot read handle for {}",
                handle.display_path()
            )));
        };

        let inode = self.open_inode(label, *entry)?;
        if inode.kind() != EntryKind::File {
            return Err(AccessorError::not_a_file(display_path.clone()));
        }

        self.read_inode_bytes(display_path, entry.inode, &inode, max_read_size)
    }

    /// List files and directories in a provided directory path
    pub(crate) fn read_dir(&self, inner: &InnerPath) -> AccessorResult<Vec<DirEntry>> {
        let path = filesystem_path(inner);
        let (inode_number, inode) = resolve_path(&self.volume, &path)?;

        if inode.kind() != EntryKind::Directory {
            return Err(AccessorError::not_a_directory(path));
        }

        self.list_directory(inode_number, &inode, &path)
    }

    /// List files and directories in a provided `DirHandle`
    pub(crate) fn read_dir_handle(&self, handle: &DirHandle) -> AccessorResult<Vec<DirEntry>> {
        let DirLocator::Ext4 {
            label,
            entry,
            display_path,
        } = &handle.locator
        else {
            return Err(AccessorError::invalid_handle(format!(
                "ext4 source cannot list directory handle for {}",
                handle.display_path()
            )));
        };

        let inode = self.open_inode(label, *entry)?;
        if inode.kind() != EntryKind::Directory {
            return Err(AccessorError::not_a_directory(display_path.clone()));
        }

        self.list_directory(entry.inode, &inode, display_path)
    }

    /// Open a reader for a file to stream it
    ///
    /// All read operations check if the inode generation value has been updated
    ///
    /// If an inode entry gets deleted and reused then the generation value should be different
    pub(crate) fn reader(&self, inner: &InnerPath) -> AccessorResult<AccessorReader> {
        let path = filesystem_path(inner);
        let (inode_number, inode) = resolve_path(&self.volume, &path)?;

        if inode.kind() != EntryKind::File {
            return Err(AccessorError::not_a_file(path));
        }

        let reader = Ext4FileReader::new(Arc::clone(&self.volume), inode_number, inode.generation);
        Ok(AccessorReader::stream(
            reader,
            ReaderLocation::from_display(path),
        ))
    }

    /// Open a reader for file from a `FileHandle`
    pub(crate) fn reader_handle(&self, handle: &FileHandle) -> AccessorResult<AccessorReader> {
        let FileLocator::Ext4 {
            label,
            entry,
            display_path,
        } = &handle.locator
        else {
            return Err(AccessorError::invalid_handle(format!(
                "ext4 source cannot read handle for {}",
                handle.display_path()
            )));
        };

        let inode = self.open_inode(label, *entry)?;
        if inode.kind() != EntryKind::File {
            return Err(AccessorError::not_a_file(display_path.clone()));
        }

        let reader = Ext4FileReader::new(Arc::clone(&self.volume), entry.inode, entry.generation);
        Ok(AccessorReader::stream(
            reader,
            ReaderLocation::from_display(display_path.clone()),
        ))
    }

    /// Metadata for a file entry
    pub(crate) fn stat(&self, inner: &InnerPath) -> AccessorResult<EntryStat> {
        let path = filesystem_path(inner);
        let (inode_number, inode) = resolve_path(&self.volume, &path)?;

        Ok(stat_from_inode(inode_number, &inode, &path))
    }

    /// Stat a `FileHandle`
    pub(crate) fn stat_handle(&self, handle: &FileHandle) -> AccessorResult<EntryStat> {
        let FileLocator::Ext4 {
            label,
            entry,
            display_path,
        } = &handle.locator
        else {
            return Err(AccessorError::invalid_handle(format!(
                "ext4 source cannot stat file handle for {}",
                handle.display_path()
            )));
        };

        let inode = self.open_inode(label, *entry)?;
        Ok(stat_from_inode(entry.inode, &inode, display_path))
    }

    /// Stat a `DirHandle`
    pub(crate) fn stat_dir_handle(&self, handle: &DirHandle) -> AccessorResult<EntryStat> {
        let DirLocator::Ext4 {
            label,
            entry,
            display_path,
        } = &handle.locator
        else {
            return Err(AccessorError::invalid_handle(format!(
                "ext4 source cannot stat file handle for {}",
                handle.display_path()
            )));
        };

        let inode = self.open_inode(label, *entry)?;
        if inode.kind() != EntryKind::Directory {
            return Err(AccessorError::not_a_directory(display_path.clone()));
        }
        Ok(stat_from_inode(entry.inode, &inode, display_path))
    }

    /// Open the `Inode` value from a `Ext4EntryRef`
    fn open_inode(&self, label: &str, entry: Ext4EntryRef) -> AccessorResult<Inode> {
        if label != self.volume.target_path() {
            return Err(AccessorError::invalid_handle(format!(
                "ext4 handle label {label} does not match {}",
                self.volume.target_path()
            )));
        }

        let inode = self.volume.read_inode(entry.inode)?;
        if inode.generation != entry.generation {
            return Err(AccessorError::invalid_handle(format!(
                "inode {} generation is {:#x}, handle has {:#x}",
                entry.inode, inode.generation, entry.generation
            )));
        }

        Ok(inode)
    }

    /// Read bytes associated with `Inode` value
    fn read_inode_bytes(
        &self,
        path: &str,
        inode_number: u64,
        inode: &Inode,
        max_read_size: Option<u64>,
    ) -> AccessorResult<Vec<u8>> {
        if let Some(limit) = max_read_size
            && inode.size > limit
        {
            return Err(AccessorError::file_too_large(inode.size, limit));
        }

        // We only read symlinks to determine the path in points to
        // We never follow them
        if inode.kind() == EntryKind::Symlink {
            return read_symlink(&self.volume, inode_number, inode);
        }

        if inode.kind() != EntryKind::File {
            return Err(AccessorError::not_a_file(path));
        }

        let length = usize::try_from(inode.size).map_err(|err| {
            ext4_error(format!(
                "inode {inode_number} size {} does not fit in memory: {err:?}",
                inode.size
            ))
        })?;

        let mut buf = vec![0_u8; length];
        let bytes = read_file_at(&self.volume, inode_number, inode, 0, &mut buf)?;
        if bytes != buf.len() {
            return Err(ext4_error(format!(
                "inode {inode_number} read {bytes} of {} bytes",
                buf.len()
            )));
        }

        Ok(buf)
    }

    /// List files and directories from `Inode` value
    fn list_directory(
        &self,
        inode_number: u64,
        inode: &Inode,
        parent: &str,
    ) -> AccessorResult<Vec<DirEntry>> {
        let entries = read_directory(&self.volume, inode_number, inode)?;
        let label = self.volume.target_path();
        let mut listed = Vec::with_capacity(entries.len());

        for entry in entries {
            let child_number = entry.inode as u64;
            let child = self.volume.read_inode(child_number)?;

            let name = extract_utf8_string_lossy(&entry.name);
            let display_path = child_path(parent, &name);
            let reference = self.volume.entry_ref(child_number, &child)?;
            let handle = item_handle(child.kind(), label, reference, display_path.clone());
            let stat = stat_from_inode(child_number, &child, &display_path);

            listed.push(DirEntry::new(name, handle, stat.meta, stat.times));
        }

        Ok(listed)
    }
}

/// Return `EntryStat` from a parsed `Inode` value
fn stat_from_inode(inode_number: u64, inode: &Inode, path: &str) -> EntryStat {
    let mut meta = EntryMeta::new(inode.kind(), inode.size, path);
    meta.inode = inode_number;
    meta.uid = inode.uid;
    meta.gid = inode.gid;
    meta.attributes = unix_attributes(inode.mode as u32);

    EntryStat {
        meta,
        times: timestamps(inode),
    }
}

/// Return timestamps from a parsed `Inode`
fn timestamps(inode: &Inode) -> Timestamp {
    Timestamp {
        created: inode
            .created
            .map_or_else(|| Timestamp::default().created, iso_time),
        modified: iso_time(inode.modified),
        accessed: iso_time(inode.accessed),
        changed: iso_time(inode.changed),
    }
}

/// Convert ISO8601 with nanosecond precision
fn iso_time(time: InodeTime) -> String {
    unixepoch_to_iso_with_nano(time.seconds, i64::from(time.nanoseconds))
}

/// Return the filesystem target from an `InnerPath` value
fn filesystem_path(inner: &InnerPath) -> String {
    if inner.is_empty() {
        String::from("/")
    } else {
        inner.display()
    }
}

/// Construct an `ItemHandle` from `Ext4EntryRef`
fn item_handle(
    kind: EntryKind,
    label: &str,
    entry: Ext4EntryRef,
    display_path: String,
) -> ItemHandle {
    match kind {
        EntryKind::File => ItemHandle::File(FileHandle::new(FileLocator::Ext4 {
            label: label.to_string(),
            entry,
            display_path: display_path.clone(),
        })),
        EntryKind::Directory => ItemHandle::Directory(DirHandle::new(DirLocator::Ext4 {
            label: label.to_string(),
            entry,
            display_path: display_path.clone(),
        })),
        _ => ItemHandle::Unsupported(FileHandle::new(FileLocator::Ext4 {
            label: label.to_string(),
            entry,
            display_path: display_path.clone(),
        })),
    }
}

/// Return the full path for when listing files and directories in a directory
fn child_path(parent: &str, name: &str) -> String {
    if parent == "/" {
        format!("/{name}")
    } else {
        format!("{parent}/{name}")
    }
}

#[cfg(test)]
mod tests {
    use super::Ext4Fs;
    use crate::{
        accessor::{
            entry::{
                handle::FileHandle,
                locator::{Ext4EntryRef, FileLocator},
            },
            error::AccessorError,
            filesystem::ext4::volume::Ext4Volume,
            location::path::InnerPath,
        },
        utils::time::unixepoch_to_iso_with_nano,
    };
    use common::files::{Attributes, EntryKind};
    use std::{fs::File, io::Read, path::PathBuf};

    fn image() -> Ext4Fs<File> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/images/ext4/test.img");
        let volume = Ext4Volume::open(File::open(path).unwrap(), "test.img").unwrap();
        Ext4Fs::new(volume)
    }

    fn inner(path: &str) -> InnerPath {
        InnerPath::new(PathBuf::from(path))
    }

    #[test]
    fn test_stat_reads_root_and_hello_txt() {
        let fs = image();

        let root = fs.stat(&inner("/")).unwrap();
        assert_eq!(root.meta.kind, EntryKind::Directory);
        assert_eq!(root.meta.inode, 2);
        assert_eq!(root.meta.size, 1024);

        assert_eq!(root.meta.uid, 0);
        assert_eq!(root.meta.gid, 0);
        assert_eq!(
            root.meta.attributes,
            vec![
                Attributes::UserRead,
                Attributes::UserWrite,
                Attributes::UserExecute,
                Attributes::GroupRead,
                Attributes::GroupExecute,
                Attributes::OtherRead,
                Attributes::OtherExecute,
            ]
        );

        let hello = fs.stat(&inner("test/hello.txt")).unwrap();
        assert_eq!(hello.meta.kind, EntryKind::File);
        assert_eq!(hello.meta.inode, 14);
        assert_eq!(hello.meta.size, 13);

        assert_eq!(hello.meta.filename, "hello.txt");
        assert_eq!(hello.meta.full_path, "test/hello.txt");
        assert_eq!(
            hello.meta.attributes,
            vec![
                Attributes::UserRead,
                Attributes::UserWrite,
                Attributes::GroupRead,
                Attributes::OtherRead,
            ]
        );
        assert_eq!(
            hello.times.modified,
            unixepoch_to_iso_with_nano(1_762_998_664, 257_148_486)
        );
        assert_eq!(
            hello.times.changed,
            unixepoch_to_iso_with_nano(1_762_998_664, 257_148_486)
        );

        assert_eq!(
            hello.times.accessed,
            unixepoch_to_iso_with_nano(1_762_998_657, 225_939_267)
        );
        assert_eq!(
            hello.times.created,
            unixepoch_to_iso_with_nano(1_762_998_652, 463_781_721)
        );
        assert!(hello.times.accessed.starts_with("2025-11-13T01:50:57."));
    }

    #[test]
    fn test_read_file_reads_hello_txt_and_afile() {
        let fs = image();

        assert_eq!(
            fs.read_file(&inner("test/hello.txt"), None).unwrap(),
            b"Hello world!\n"
        );
        assert_eq!(
            fs.read_file(&inner("/afile"), Some(13)).unwrap(),
            b"I like Rust!\n"
        );
    }

    #[test]
    fn test_read_file_rejects_a_directory_a_missing_path_and_a_low_limit() {
        let fs = image();

        assert!(matches!(
            fs.read_file(&inner("test"), None),
            Err(AccessorError::NotAFile { .. })
        ));
        assert!(matches!(
            fs.stat(&inner("missing")),
            Err(AccessorError::NotFound { .. })
        ));
        assert!(matches!(
            fs.read_file(&inner("afile"), Some(12)),
            Err(AccessorError::FileTooLarge {
                size: 13,
                limit: 12
            })
        ));
    }

    #[test]
    fn test_reader_seeks_within_hello_txt() {
        let fs = image();
        let mut reader = fs.reader(&inner("test/hello.txt")).unwrap();

        assert_eq!(reader.location.filename(), "hello.txt");

        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).unwrap();
        assert_eq!(buf, b"Hello world!\n");

        reader.seek_from_start(6).unwrap();
        let mut rest = [0_u8; 7];
        reader.read_exact(&mut rest).unwrap();

        assert_eq!(&rest, b"world!\n");
    }

    #[test]
    fn test_read_dir_lists_root_and_reads_afile_by_handle() {
        let fs = image();
        let root = fs.read_dir(&inner("/")).unwrap();
        assert_eq!(
            root.iter()
                .map(|entry| entry.name.as_str())
                .collect::<Vec<_>>(),
            ["lost+found", "test", "afile"]
        );

        let afile = root.iter().find(|entry| entry.name == "afile").unwrap();
        let handle = afile.handle.as_file().unwrap();
        assert_eq!(fs.read_handle(handle, None).unwrap(), b"I like Rust!\n");
        assert_eq!(fs.stat_handle(handle).unwrap().meta.inode, 18);

        let test_dir = root
            .iter()
            .find(|entry| entry.name == "test")
            .unwrap()
            .handle
            .as_directory()
            .unwrap();

        let children = fs.read_dir_handle(test_dir).unwrap();
        assert!(children.iter().any(|entry| entry.name == "hello.txt"));
        assert_eq!(
            fs.stat_dir_handle(test_dir).unwrap().meta.kind,
            EntryKind::Directory
        );
    }

    #[test]
    fn test_read_dir_rejects_a_file_and_a_stale_handle() {
        let fs = image();
        assert!(matches!(
            fs.read_dir(&inner("afile")),
            Err(AccessorError::NotADirectory { .. })
        ));

        let root = fs.read_dir(&inner("/")).unwrap();
        let handle = root
            .iter()
            .find(|entry| entry.name == "afile")
            .unwrap()
            .handle
            .as_file()
            .unwrap();

        let FileLocator::Ext4 {
            label,
            entry,
            display_path,
        } = &handle.locator
        else {
            panic!("expected an ext4 file handle");
        };

        let stale = FileHandle::new(FileLocator::Ext4 {
            label: label.clone(),
            entry: Ext4EntryRef {
                inode: entry.inode,
                generation: 0,
            },
            display_path: display_path.clone(),
        });
        assert!(matches!(
            fs.read_handle(&stale, None),
            Err(AccessorError::InvalidHandle { .. })
        ));

        let foreign = FileHandle::new(FileLocator::Ext4 {
            label: String::from("other"),
            entry: *entry,
            display_path: display_path.clone(),
        });
        assert!(matches!(
            fs.stat_handle(&foreign),
            Err(AccessorError::InvalidHandle { .. })
        ));
    }
}
