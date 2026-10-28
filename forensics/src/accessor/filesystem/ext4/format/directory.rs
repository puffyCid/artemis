use crate::{
    accessor::{
        error::AccessorResult,
        filesystem::ext4::format::{
            checksum::crc32c_check,
            superblock::{SuperBlock, ext4_error},
        },
    },
    utils::nom_helper::{nom_u8, nom_u16, nom_u32},
};

/// Parsed `Directory` values
///
/// <https://kernel.org/doc/html/latest/filesystems/ext4/directory.html>
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DirectoryEntry {
    /// Inode number for entry
    pub(crate) inode: u32,
    /// Entry name in bytes
    pub(crate) name: Vec<u8>,
    /// Entry filetype
    ///
    /// Can be File, Directory, Char device, Block device, Pipe, Socket, Symbolic link
    pub(crate) file_type: u8,
}

/// Parse a `Directory` entries
pub(crate) fn parse_directory_block(
    data: &[u8],
    block_size: u32,
    filetype: bool,
) -> AccessorResult<Vec<DirectoryEntry>> {
    let block_size = usize::try_from(block_size).map_err(|err| {
        ext4_error(format!(
            "directory block size does not fit in memory: {err:?}"
        ))
    })?;

    if data.len() != block_size {
        return Err(ext4_error(format!(
            "directory block is {} bytes, expected {block_size}",
            data.len()
        )));
    }

    let mut entries = Vec::new();
    let mut offset = 0_usize;
    let mut found_tail = false;

    while offset + 8 <= data.len() {
        let (remaining, inode) = nom_u32(&data[offset..], "directory entry inode is truncated")?;
        let (remaining, raw_length) =
            nom_u16(remaining, "directory entry record length is truncated")?;

        let (name_length, file_type) = if filetype {
            let (remaining, name_length) =
                nom_u8(remaining, "directory entry name length is truncated")?;
            let (_, file_type) = nom_u8(remaining, "directory entry file type is truncated")?;
            (name_length as usize, file_type)
        } else {
            let (_, name_length) = nom_u16(remaining, "directory entry name length is truncated")?;
            (name_length as usize, 0)
        };

        // Determine the length of the directory entry
        // Commonly a multiple of 4
        let record_length = record_length(raw_length, block_size);
        if record_length < 8
            || !record_length.is_multiple_of(4)
            || offset + record_length > data.len()
        {
            return Err(ext4_error(format!(
                "directory entry at {offset} has record length {record_length}"
            )));
        }

        let file_type_checksum = 222;
        if file_type == file_type_checksum {
            if offset + record_length != data.len() {
                return Err(ext4_error(
                    "directory checksum tail is not at the end of the block",
                ));
            }

            found_tail = true;
            break;
        }

        let max_name_size = 255;
        if name_length > max_name_size || name_length + 8 > record_length {
            return Err(ext4_error(format!(
                "directory entry at {offset} has name length {name_length}. Should be smaller than {max_name_size} bytes"
            )));
        }

        let name_start = offset + 8;
        let name = data[name_start..name_start + name_length].to_vec();
        if name.iter().any(|byte| *byte == 0 || *byte == b'/') {
            return Err(ext4_error(format!(
                "directory entry at {offset} has an invalid name"
            )));
        }

        // Inode value of 0 means the entry is unused
        if inode != 0 {
            entries.push(DirectoryEntry {
                inode,
                name,
                file_type,
            });
        }

        offset += record_length;
    }

    if !found_tail && offset != data.len() {
        return Err(ext4_error(format!(
            "directory block stopped at {offset}, expected {block_size}"
        )));
    }

    Ok(entries)
}

/// Check the `DirectoryEntry` checksum value
pub(crate) fn verify_directory_block(
    superblock: &SuperBlock,
    checksum_seed: Option<u32>,
    inode: u32,
    generation: u32,
    data: &[u8],
) -> AccessorResult<()> {
    if !superblock.has_read_only(SuperBlock::METADATA_CSUM) {
        return Ok(());
    }

    let volume_seed =
        checksum_seed.ok_or_else(|| ext4_error("metadata checksum seed is missing"))?;
    let seed = inode_checksum_seed(volume_seed, inode, generation);

    // Check if we are parsing hash tree directory
    // <https://kernel.org/doc/html/latest/filesystems/ext4/directory.html#hash-tree-directories>
    // Find the offset to the count value
    if let Some(count_offset) = directory_index_count_offset(data) {
        return verify_directory_index_checksum(inode, seed, data, count_offset);
    }

    verify_leaf_checksum(inode, seed, data)
}

/// Checked the CRC32C value for the `Directory` inode
fn inode_checksum_seed(seed: u32, inode: u32, generation: u32) -> u32 {
    let crc = crc32c_check(seed, &inode.to_le_bytes());

    crc32c_check(crc, &generation.to_le_bytes())
}

/// Check the leaf `Directory` checksum value
fn verify_leaf_checksum(inode: u32, seed: u32, data: &[u8]) -> AccessorResult<()> {
    let tail_size = 12;
    let length = data.len();

    if length < tail_size {
        return Err(ext4_error(format!(
            "inode {inode} directory block is too small for a checksum. Got {length} needs to be at least {tail_size}"
        )));
    }

    let tail = &data[length - tail_size..];
    let (remaining, reserved) = nom_u32(tail, "directory checksum tail is truncated")?;
    let (remaining, raw_length) = nom_u16(remaining, "directory checksum tail is truncated")?;
    let (remaining, name_length) = nom_u8(remaining, "directory checksum tail is truncated")?;
    let (remaining, file_type) = nom_u8(remaining, "directory checksum tail is truncated")?;
    let (_, stored) = nom_u32(remaining, "directory checksum is truncated")?;
    let record_length = record_length(raw_length, length);

    let file_type_checksum = 222;
    if reserved != 0
        || record_length != tail_size
        || name_length != 0
        || file_type != file_type_checksum
    {
        return Err(ext4_error(format!(
            "inode {inode} directory block is missing its checksum tail"
        )));
    }

    let actual = crc32c_check(seed, &data[..length - tail_size]);
    if actual != stored {
        return Err(ext4_error(format!(
            "inode {inode} directory checksum mismatch: found {actual:#x}, stored {stored:#x}"
        )));
    }

    Ok(())
}

/// Check the leaf `Directory` checksum value for hash tree directories
fn verify_directory_index_checksum(
    inode: u32,
    seed: u32,
    data: &[u8],
    count_offset: usize,
) -> AccessorResult<()> {
    if count_offset + 4 > data.len() {
        return Err(ext4_error(format!(
            "inode {inode} hash tree header is outside the block"
        )));
    }

    let (remaining, limit) = nom_u16(
        &data[count_offset..],
        "directory hash tree limit is truncated",
    )?;
    let (_, count) = nom_u16(remaining, "directory hash tree count is truncated")?;

    if limit == 0 || count == 0 || count > limit {
        return Err(ext4_error(format!(
            "inode {inode} hash tree count {count} is outside limit {limit}"
        )));
    }

    let size = 8;
    let tail = count_offset
        .checked_add(
            limit
                .checked_mul(size)
                .ok_or_else(|| ext4_error(format!("inode {inode} hash tree size overflows")))?
                as usize,
        )
        .ok_or_else(|| ext4_error(format!("inode {inode} hash tree size overflows")))?;

    let covered = count_offset
        .checked_add(
            count
                .checked_mul(size)
                .ok_or_else(|| ext4_error(format!("inode {inode} hash tree size overflows")))?
                as usize,
        )
        .ok_or_else(|| ext4_error(format!("inode {inode} hash tree size overflows")))?;

    if tail + size as usize > data.len() || covered > tail {
        return Err(ext4_error(format!(
            "inode {inode} hash tree checksum is outside the block"
        )));
    }

    let mut actual = crc32c_check(seed, &data[..covered]);
    actual = crc32c_check(actual, &data[tail..tail + 4]);
    actual = crc32c_check(actual, &0_u32.to_le_bytes());

    let (_, stored) = nom_u32(
        &data[tail + 4..],
        "directory hash tree checksum is truncated",
    )?;

    if actual != stored {
        return Err(ext4_error(format!(
            "inode {inode} hash tree checksum mismatch: found {actual:#x}, stored {stored:#x}"
        )));
    }

    Ok(())
}

/// Check if the directory entry is a hash tree
fn directory_index_count_offset(data: &[u8]) -> Option<usize> {
    let block_size = data.len();

    if block_size < 8 {
        return None;
    }

    let first = record_length(u16::from_le_bytes([data[4], data[5]]), block_size);
    if first == block_size {
        return Some(8);
    }

    if first != 12 || block_size < 32 {
        return None;
    }

    let second = record_length(u16::from_le_bytes([data[16], data[17]]), block_size);
    if second != block_size - 12 {
        return None;
    }

    let reserved = u32::from_le_bytes([data[24], data[25], data[26], data[27]]);
    if reserved != 0 || data[29] != 8 {
        return None;
    }

    Some(32)
}

/// Determine the record length of a `Directory`
fn record_length(raw: u16, block_size: usize) -> usize {
    if block_size < 65536 {
        return raw as usize;
    }

    if raw == 0 || raw == u16::MAX {
        return block_size;
    }

    // For large 64kb block sizes
    // The max u16 size is "65535" which not a multiple of 4
    // We have to get the lower bits
    // https://lists.openwall.net/linux-ext4/2009/02/12/2
    raw as usize & 65532 | ((raw as usize & 3) << 16)
}

#[cfg(test)]
mod tests {
    use super::{
        DirectoryEntry, crc32c_check, inode_checksum_seed, parse_directory_block,
        verify_directory_block,
    };
    use crate::accessor::{
        error::AccessorError,
        filesystem::ext4::format::superblock::{SuperBlock, parse_superblock},
    };
    use std::{fs::read, path::PathBuf};

    fn superblock() -> SuperBlock {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/filesystems/ext4/debian/bookworm/firstblock.raw");
        parse_superblock(&read(path).unwrap()).unwrap()
    }

    fn ext4_reason(result: Result<impl std::fmt::Debug, AccessorError>) -> String {
        match result {
            Err(AccessorError::Ext4 { reason, .. }) => reason,
            other => panic!("expected an ext4 error, got {other:?}"),
        }
    }

    fn put_u16(data: &mut [u8], offset: usize, value: u16) {
        data[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn put_u32(data: &mut [u8], offset: usize, value: u32) {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_entry(
        data: &mut [u8],
        offset: usize,
        inode: u32,
        record_length: u16,
        file_type: u8,
        name: &[u8],
    ) {
        put_u32(data, offset, inode);
        put_u16(data, offset + 4, record_length);
        data[offset + 6] = u8::try_from(name.len()).unwrap();
        data[offset + 7] = file_type;
        data[offset + 8..offset + 8 + name.len()].copy_from_slice(name);
    }

    fn write_tail(data: &mut [u8]) {
        let offset = data.len() - 12;
        put_u16(data, offset + 4, 12);
        data[offset + 7] = 0xde;
    }

    fn seal_leaf(data: &mut [u8], seed: u32, inode: u32, generation: u32) {
        write_tail(data);
        let checksum_seed = inode_checksum_seed(seed, inode, generation);
        let sum = crc32c_check(checksum_seed, &data[..data.len() - 12]);
        put_u32(data, data.len() - 4, sum);
    }

    #[test]
    fn test_parse_directory_block_reads_entries_and_skips_holes() {
        let mut data = vec![0_u8; 128];
        write_entry(&mut data, 0, 2, 12, 2, b".");
        write_entry(&mut data, 12, 2, 12, 2, b"..");
        write_entry(&mut data, 24, 0, 16, 1, b"gone");
        write_entry(&mut data, 40, 15, 88, 1, b"hello");

        let entries = parse_directory_block(&data, 128, true).unwrap();

        assert_eq!(
            entries,
            vec![
                DirectoryEntry {
                    inode: 2,
                    name: b".".to_vec(),
                    file_type: 2,
                },
                DirectoryEntry {
                    inode: 2,
                    name: b"..".to_vec(),
                    file_type: 2,
                },
                DirectoryEntry {
                    inode: 15,
                    name: b"hello".to_vec(),
                    file_type: 1,
                },
            ]
        );
    }

    #[test]
    fn test_parse_directory_block_stops_at_the_checksum_tail() {
        let mut data = vec![0_u8; 64];
        write_entry(&mut data, 0, 11, 20, 2, b"lost+found");
        write_entry(&mut data, 20, 12, 32, 1, b"afile");
        write_tail(&mut data);

        let entries = parse_directory_block(&data, 64, true).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[1].name, b"afile");
    }

    #[test]
    fn test_parse_directory_block_rejects_a_slash_in_the_name() {
        let mut data = vec![0_u8; 32];
        write_entry(&mut data, 0, 4, 32, 1, b"a/b");

        let reason = ext4_reason(parse_directory_block(&data, 32, true));
        assert!(reason.contains("invalid name"), "{reason}");
    }

    #[test]
    fn test_parse_directory_block_rejects_a_record_past_the_block() {
        let mut data = vec![0_u8; 32];
        write_entry(&mut data, 0, 4, 64, 1, b"a");

        let reason = ext4_reason(parse_directory_block(&data, 32, true));
        assert!(reason.contains("record length"), "{reason}");
    }

    #[test]
    fn test_verify_directory_block_checks_a_leaf_tail() {
        let superblock = superblock();
        let seed = superblock.checksum_seed().unwrap();
        let mut data = vec![0_u8; 128];

        write_entry(&mut data, 0, 2, 12, 2, b".");
        write_entry(&mut data, 12, 2, 104, 2, b"..");
        seal_leaf(&mut data, seed, 7, 0xabcd);

        verify_directory_block(&superblock, Some(seed), 7, 0xabcd, &data).unwrap();

        data[8] ^= 0xff;
        let reason = ext4_reason(verify_directory_block(
            &superblock,
            Some(seed),
            7,
            0xabcd,
            &data,
        ));
        assert!(reason.contains("checksum mismatch"), "{reason}");
    }

    #[test]
    fn test_verify_directory_block_checks_a_hash_tree_root() {
        let superblock = superblock();
        let seed = superblock.checksum_seed().unwrap();
        let mut data = vec![0_u8; 128];
        write_entry(&mut data, 0, 2, 12, 2, b".");
        write_entry(&mut data, 12, 2, 116, 2, b"..");

        data[29] = 8;

        put_u16(&mut data, 32, 2);
        put_u16(&mut data, 34, 2);
        put_u32(&mut data, 40, 0);
        put_u32(&mut data, 44, 1);

        let checksum_seed = inode_checksum_seed(seed, 9, 3);
        let mut actual = crc32c_check(checksum_seed, &data[..48]);
        actual = crc32c_check(actual, &data[48..52]);
        actual = crc32c_check(actual, &0_u32.to_le_bytes());
        put_u32(&mut data, 52, actual);

        verify_directory_block(&superblock, Some(seed), 9, 3, &data).unwrap();

        data[44] ^= 0xff;
        let reason = ext4_reason(verify_directory_block(&superblock, Some(seed), 9, 3, &data));
        assert!(reason.contains("checksum mismatch"), "{reason}");
    }

    #[test]
    fn test_verify_directory_block_skips_checksums_when_metadata_csum_is_off() {
        let mut superblock = superblock();
        superblock.read_only_features &= !SuperBlock::METADATA_CSUM;
        let data = vec![0_u8; 32];

        verify_directory_block(&superblock, None, 1, 0, &data).unwrap();
    }
}
