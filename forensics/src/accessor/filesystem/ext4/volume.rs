use crate::accessor::{
    error::{AccessorError, AccessorResult},
    filesystem::ext4::format::{
        layout::Ext4Layout,
        superblock::{SuperBlock, ext4_error, parse_superblock},
    },
};
use std::{
    io::{Read, Seek, SeekFrom},
    sync::{Mutex, MutexGuard},
};

/// Open EXT4 filesystem
#[derive(Debug)]
pub(crate) struct Ext4Volume<R: Read + Seek + Send> {
    /// Path to ext4 filesystem
    pub(crate) target_path: String,
    /// Parsed `SuperBlock` structure
    pub(crate) superblock: SuperBlock,
    /// Parsed ext4 filesystem layout
    pub(crate) layout: Ext4Layout,
    /// Seed for the checksum metadata
    pub(crate) checksum_seed: Option<u32>,
    /// Reader for the ext4 filesystem
    pub(crate) reader: Mutex<R>,
}

impl<R: Read + Seek + Send> Ext4Volume<R> {
    /// Use the provided reader to read and parse the `SuperBlock` bytes
    pub(crate) fn open(mut reader: R, label: impl Into<String>) -> AccessorResult<Self> {
        let target_path = label.into();

        let superblock_offset = 1024;
        reader
            .seek(SeekFrom::Start(superblock_offset))
            .map_err(|err| {
                ext4_path_error(
                    &target_path,
                    format!("failed to seek to the superblock: {err}"),
                )
            })?;

        let mut data = [0_u8; 1024];
        reader.read_exact(&mut data).map_err(|err| {
            ext4_path_error(
                &target_path,
                format!("failed to read the superblock: {err}"),
            )
        })?;

        let superblock = parse_superblock(&data)?;
        superblock.validate_superblock(&data)?;

        let layout = Ext4Layout::from_superblock(&superblock)?;
        reject_unsupported(&superblock)?;
        let checksum_seed = superblock.checksum_seed();

        Ok(Self {
            target_path,
            superblock,
            layout,
            checksum_seed,
            reader: Mutex::new(reader),
        })
    }

    /// Return the ext4 filesystem target
    pub(super) fn target_path(&self) -> &str {
        &self.target_path
    }

    /// Return the `SuperBlock` structure
    pub(super) fn superblock(&self) -> &SuperBlock {
        &self.superblock
    }

    /// Return the `Ext4Layout` structure
    pub(super) fn layout(&self) -> &Ext4Layout {
        &self.layout
    }

    /// Return the checksum seed value
    pub(super) fn checksum_seed(&self) -> Option<u32> {
        self.checksum_seed
    }

    /// Return the ext4 filesystem reader
    pub(super) fn with_reader<F, T>(&self, operation: F) -> AccessorResult<T>
    where
        F: FnOnce(&mut R) -> AccessorResult<T>,
    {
        let mut reader = self.lock_reader()?;
        operation(&mut reader)
    }

    /// Prevent multiple ext4 readers
    fn lock_reader(&self) -> AccessorResult<MutexGuard<'_, R>> {
        self.reader.lock().map_err(|err| {
            ext4_path_error(
                &self.target_path(),
                format!("ext4 volume reader lock poisoned: {err:?}"),
            )
        })
    }
}

/// Check for unsupported ext4 flags
fn reject_unsupported(superblock: &SuperBlock) -> AccessorResult<()> {
    if superblock.revision < 1 || !superblock.has_incompatible(SuperBlock::EXTENTS) {
        return Err(ext4_error("extent-based EXT4 is required"));
    }

    let unknown = superblock.unknown_incompatible();
    if unknown != 0 {
        return Err(ext4_error(format!(
            "unsupported incompatible features {unknown:#x}"
        )));
    }

    // Current unsupported flags
    /*
     * <https://docs.kernel.org/filesystems/ext4/super.html>
     * Meta block groups (META_BG) <https://origin.kernel.org/doc/html/latest/filesystems/ext4/blockgroup.html#meta-block-groups>
     * Encryption
     * Inline data <https://docs.kernel.org/filesystems/ext4/inlinedata.html>
     * Extended Attributes Inode <https://origin.kernel.org/doc/html/latest/filesystems/ext4/eainode.html>
     * Case insensitive filenames
     * Compression - Legacy flag
     * Directory data - Never implemented?
     * Journal device - The ext4 journal is a separate attached device (ex: a SSD)
     */
    let incompatible = [
        (SuperBlock::META_BG, "meta_bg"),
        (SuperBlock::ENCRYPT, "encrypt"),
        (SuperBlock::INLINE_DATA, "inline_data"),
        (SuperBlock::EA_INODE, "ea_inode"),
        (SuperBlock::CASEFOLD, "casefold"),
        (SuperBlock::COMPRESSION, "compression"),
        (SuperBlock::DIR_DATA, "dir_data"),
        (SuperBlock::JOURNAL_DEV, "journal_dev"),
    ];

    for (bit, name) in incompatible {
        if superblock.has_incompatible(bit) {
            return Err(ext4_error(format!(
                "unsupported incompatible feature {name}"
            )));
        }
    }

    if superblock.has_read_only(SuperBlock::BIGALLOC) {
        return Err(ext4_error("unsupported read-only feature bigalloc"));
    }

    Ok(())
}

/// Map errors to `AccessorError`
fn ext4_path_error(label: &str, reason: impl Into<String>) -> AccessorError {
    AccessorError::ext4(Some(label.to_string()), reason)
}

#[cfg(test)]
mod tests {
    use super::Ext4Volume;
    use crate::accessor::{
        error::AccessorError,
        filesystem::ext4::format::{checksum::crc32c_check, superblock::SuperBlock},
    };
    use std::{fs::read, io::Cursor, path::PathBuf};

    fn fixture() -> Vec<u8> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/filesystems/ext4/debian/bookworm/firstblock.raw");
        read(path).unwrap()
    }

    fn image_from(superblock: &[u8]) -> Cursor<Vec<u8>> {
        let mut image = vec![0_u8; 1024];
        image.extend_from_slice(superblock);
        Cursor::new(image)
    }

    fn restore_checksum(superblock: &mut [u8]) {
        let sum = crc32c_check(0xffff_ffff, &superblock[..0x3fc]);
        superblock[0x3fc..0x400].copy_from_slice(&sum.to_le_bytes());
    }

    #[test]
    fn test_open_debian12() {
        let volume = Ext4Volume::open(image_from(&fixture()), "bookworm").unwrap();

        assert_eq!(volume.target_path(), "bookworm");
        assert_eq!(volume.layout().block_size, 4096);
        assert_eq!(volume.layout().group_count, 187);
        assert_eq!(volume.checksum_seed(), Some(3_502_656_522));

        assert!(volume.superblock().has_incompatible(SuperBlock::RECOVER));
        assert_eq!(volume.superblock().checksum, 1082325001);
    }

    #[test]
    fn test_open_keeps_the_reader_at_the_filesystem_start() {
        let volume = Ext4Volume::open(image_from(&fixture()), "bookworm").unwrap();

        let magic = volume
            .with_reader(|reader| {
                use std::io::{Read, Seek, SeekFrom};
                reader.seek(SeekFrom::Start(1024 + 0x38))?;

                let mut magic = [0_u8; 2];

                reader.read_exact(&mut magic)?;
                Ok(u16::from_le_bytes(magic))
            })
            .unwrap();

        assert_eq!(magic, 0xef53);
    }

    #[test]
    fn test_open_rejects_a_short_read() {
        let result = Ext4Volume::open(Cursor::new(vec![0_u8; 64]), "bookworm");
        let err = result.unwrap_err();

        assert!(err.to_string().contains("bookworm"));
        assert!(err.to_string().contains("failed to read the superblock"));
    }

    #[test]
    fn test_open_rejects_bad_checksum() {
        let mut superblock = fixture();
        superblock[0x3fc] ^= 0xff;
        let result = Ext4Volume::open(image_from(&superblock), "bookworm");
        let err = result.unwrap_err();
        assert!(
            matches!(err, AccessorError::Ext4 {  reason,.. } if reason == "superblock checksum mismatch: found 0x4082f809, stored 0x4082f8f6")
        );
    }

    #[test]
    fn test_open_rejects_revision_0() {
        let mut superblock = fixture();
        superblock[0x4c..0x50].copy_from_slice(&0_u32.to_le_bytes());

        let err = Ext4Volume::open(image_from(&superblock), "bookworm").unwrap_err();
        assert!(
            matches!(err, AccessorError::Ext4 { reason,.. } if reason == "extent-based EXT4 is required")
        );
    }

    #[test]
    fn test_open_rejects_missing_extents() {
        let mut superblock = fixture();
        let features =
            u32::from_le_bytes(superblock[0x60..0x64].try_into().unwrap()) & !SuperBlock::EXTENTS;
        superblock[0x60..0x64].copy_from_slice(&features.to_le_bytes());
        restore_checksum(&mut superblock);

        let err = Ext4Volume::open(image_from(&superblock), "bookworm").unwrap_err();
        assert!(
            matches!(err, AccessorError::Ext4 { reason,.. } if reason == "extent-based EXT4 is required")
        )
    }

    #[test]
    fn test_open_rejects_unknown_incompatible_feature() {
        let mut superblock = fixture();
        let features = u32::from_le_bytes(superblock[0x60..0x64].try_into().unwrap()) | (1 << 31);
        superblock[0x60..0x64].copy_from_slice(&features.to_le_bytes());
        restore_checksum(&mut superblock);

        let err = Ext4Volume::open(image_from(&superblock), "bookworm").unwrap_err();
        assert!(
            matches!(err, AccessorError::Ext4 { reason,.. } if reason == "unsupported incompatible features 0x80000000")
        )
    }

    #[test]
    fn test_open_rejects_meta_bg() {
        let mut superblock = fixture();
        let features =
            u32::from_le_bytes(superblock[0x60..0x64].try_into().unwrap()) | SuperBlock::META_BG;
        superblock[0x60..0x64].copy_from_slice(&features.to_le_bytes());
        restore_checksum(&mut superblock);

        let err = Ext4Volume::open(image_from(&superblock), "bookworm").unwrap_err();
        assert!(
            matches!(err, AccessorError::Ext4 { reason,.. } if reason == "unsupported incompatible feature meta_bg")
        )
    }

    #[test]
    fn test_open_rejects_bigalloc() {
        let mut superblock = fixture();
        let features =
            u32::from_le_bytes(superblock[0x64..0x68].try_into().unwrap()) | SuperBlock::BIGALLOC;
        superblock[0x64..0x68].copy_from_slice(&features.to_le_bytes());
        restore_checksum(&mut superblock);

        let err = Ext4Volume::open(image_from(&superblock), "bookworm").unwrap_err();
        println!("{err:?}");
        assert!(
            matches!(err, AccessorError::Ext4 { reason,.. } if reason == "unsupported read-only feature bigalloc")
        )
    }
}
