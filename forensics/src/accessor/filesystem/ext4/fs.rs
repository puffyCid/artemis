use crate::{
    accessor::{
        entry::handle::{EntryMeta, EntryStat, Timestamp},
        error::{AccessorError, AccessorResult},
        filesystem::{
            ext4::{
                data::read_file_at,
                directory::{read_symlink, resolve_path},
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
    utils::time::unixepoch_to_iso_with_nano,
};
use common::files::EntryKind;
use std::{
    io::{Read, Seek},
    path::PathBuf,
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

        if let Some(limit) = max_read_size
            && inode.size > limit
        {
            return Err(AccessorError::file_too_large(inode.size, limit));
        }

        // We only read symlinks to determine the path in points to
        // We never follow them
        if inode.kind() == EntryKind::Symlink {
            return read_symlink(&self.volume, inode_number, &inode);
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
        let bytes = read_file_at(&self.volume, inode_number, &inode, 0, &mut buf)?;
        if bytes != buf.len() {
            return Err(ext4_error(format!(
                "inode {inode_number} read {bytes} of {} bytes",
                buf.len()
            )));
        }

        Ok(buf)
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

    /// Metadata for a file entry
    pub(crate) fn stat(&self, inner: &InnerPath) -> AccessorResult<EntryStat> {
        let path = filesystem_path(inner);
        let (inode_number, inode) = resolve_path(&self.volume, &path)?;

        Ok(stat_from_inode(inode_number, &inode, &path))
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
            .map(iso_time)
            .unwrap_or_else(|| Timestamp::default().created),
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

#[cfg(test)]
mod tests {
    use super::Ext4Fs;
    use crate::{
        accessor::{
            error::AccessorError, filesystem::ext4::volume::Ext4Volume, location::path::InnerPath,
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
}
