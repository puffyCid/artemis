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
use tracing::{debug, warn};

/// Extracted Extended Attribute values
///
/// <https://docs.kernel.org/filesystems/ext4/attributes.html>
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ExtendedAttribute {
    /// Name of the extended attribute
    ///
    /// Max size is 255 bytes
    pub(crate) name: Vec<u8>,
    /// Value associated with the attribute name
    pub(crate) value: Vec<u8>,
    /// Inode that contains the attribute data
    pub(crate) value_inode: u32,
    /// Size of the attribute value
    pub(crate) value_size: u32,
}

/// Get extended attributes that exist the inline data
pub(crate) fn parse_inline_attributes(
    inode: u32,
    data: &[u8],
) -> AccessorResult<Vec<ExtendedAttribute>> {
    if data.iter().all(|byte| *byte == 0) {
        return Ok(Vec::new());
    }

    let inline_header_size = 4;
    if data.len() < inline_header_size {
        return Err(ext4_error(format!(
            "inode {inode} inline attributes are {} bytes",
            data.len()
        )));
    }

    let (_, sig) = nom_u32(data, "inline attribute magic is truncated")?;
    let attr_sig = 0xea020000;
    if sig != attr_sig {
        return Err(ext4_error(format!(
            "inode {inode} inline attribute magic {sig:#x} is not {attr_sig:#x}"
        )));
    }

    parse_attributes(inode, data, inline_header_size, inline_header_size, false)
}

/// Get attributes that are stored in a ext4 block
pub(crate) fn parse_attribute_block(
    inode: u32,
    data: &[u8],
) -> AccessorResult<Vec<ExtendedAttribute>> {
    let header_size = 32;
    if data.len() < header_size {
        return Err(ext4_error(format!(
            "inode {inode} attribute block is {} bytes",
            data.len()
        )));
    }

    let (remaining, sig) = nom_u32(data, "attribute block magic is truncated")?;
    let attr_sig = 0xea020000;
    if sig != attr_sig {
        return Err(ext4_error(format!(
            "inode {inode} attribute block magic {sig:#x} is not {attr_sig:#x}"
        )));
    }

    let (remaining, _refcount) =
        nom_u32(remaining, "attribute block reference count is truncated")?;
    let (remaining, blocks) = nom_u32(remaining, "attribute block count is truncated")?;
    if blocks != 1 {
        return Err(ext4_error(format!(
            "inode {inode} attribute block count is {blocks}"
        )));
    }

    let (remaining, _hash) = nom_u32(remaining, "attribute block hash is truncated")?;
    let (remaining, _checksum) = nom_u32(remaining, "attribute block checksum is truncated")?;
    let (remaining, reserved0) = nom_u32(remaining, "attribute block reserved field is truncated")?;
    let (remaining, reserved1) = nom_u32(remaining, "attribute block reserved field is truncated")?;
    let (_, reserved2) = nom_u32(remaining, "attribute block reserved field is truncated")?;

    if reserved0 != 0 || reserved1 != 0 || reserved2 != 0 {
        return Err(ext4_error(format!(
            "inode {inode} attribute block reserved field is not zero"
        )));
    }

    parse_attributes(inode, data, header_size, 0, true)
}

/// Checksum CRC32C validation against an attribute block
pub(crate) fn verify_attribute_block(
    superblock: &SuperBlock,
    checksum_seed: Option<u32>,
    inode: u32,
    block_number: u64,
    data: &[u8],
) -> AccessorResult<()> {
    if !superblock.has_read_only(SuperBlock::METADATA_CSUM) {
        return Ok(());
    }

    let header_size = 32;
    if data.len() < header_size {
        return Err(ext4_error(format!(
            "inode {inode} attribute block is {} bytes",
            data.len()
        )));
    }

    let seed = checksum_seed.ok_or_else(|| ext4_error("metadata checksum seed is missing"))?;
    let (_, stored) = nom_u32(&data[16..], "attribute block checksum is truncated")?;

    let mut block = data.to_vec();
    block[16..20].fill(0);
    let mut actual = crc32c_check(seed, &block_number.to_le_bytes());

    actual = crc32c_check(actual, &block);

    if actual != stored {
        return Err(ext4_error(format!(
            "inode {inode} attribute block checksum mismatch: found {actual:#x}, stored {stored:#x}"
        )));
    }

    Ok(())
}

/// Parse attribute data in either inline or block data
fn parse_attributes(
    inode: u32,
    data: &[u8],
    start: usize,
    base: usize,
    sorted: bool,
) -> AccessorResult<Vec<ExtendedAttribute>> {
    let mut attributes = Vec::new();
    let mut value_at = Vec::new();
    let mut previous: Option<(u8, u8, Vec<u8>)> = None;
    let mut found_end = false;

    let mut offset = start;
    let entry_size = 16;
    let name_size = 255;

    while offset + 4 <= data.len() {
        if data[offset..offset + 4] == [0, 0, 0, 0] {
            found_end = true;
            offset += 4;

            break;
        }

        if offset + entry_size > data.len() {
            return Err(ext4_error(format!(
                "inode {inode} attribute entry at {offset} is truncated"
            )));
        }

        let (remaining, name_len) = nom_u8(&data[offset..], "attribute name length is truncated")?;
        let (remaining, name_index) = nom_u8(remaining, "attribute name index is truncated")?;
        let (remaining, value_off) = nom_u16(remaining, "attribute value offset is truncated")?;
        let (remaining, value_inode) = nom_u32(remaining, "attribute value inode is truncated")?;
        let (remaining, value_size) = nom_u32(remaining, "attribute value size is truncated")?;
        let (_, hash) = nom_u32(remaining, "attribute hash is truncated")?;

        if !sorted && hash != 0 {
            warn!("inode {inode} inline attribute hash is {hash:#x}");
        }

        if name_len > name_size {
            return Err(ext4_error(format!(
                "inode {inode} attribute name length is {name_len}"
            )));
        }

        let name_offset = offset + entry_size;
        let name_end = name_offset
            .checked_add(name_len as usize)
            .ok_or_else(|| ext4_error(format!("inode {inode} attribute name offset overflows")))?;
        if name_end > data.len() {
            return Err(ext4_error(format!(
                "inode {inode} attribute name at {offset} extends past the buffer"
            )));
        }

        let name_bytes = &data[name_offset..name_end];
        if name_bytes.contains(&0) {
            return Err(ext4_error(format!(
                "inode {inode} attribute name at {offset} contains a NUL byte"
            )));
        }

        // Ensure attributes round to 4 byte boundary
        let size = (entry_size + name_len as usize + 3) & !3;
        let next = offset
            .checked_add(size)
            .ok_or_else(|| ext4_error(format!("inode {inode} attribute entry offset overflows")))?;

        if next > data.len() {
            return Err(ext4_error(format!(
                "inode {inode} attribute entry at {offset} extends past the buffer"
            )));
        }

        if sorted && let Some((prev_index, prev_len, prev_name)) = &previous {
            let key = (name_index, name_len, name_bytes);
            let prev = (*prev_index, *prev_len, prev_name.as_slice());

            if key <= prev {
                warn!("inode {inode} attribute names are out of order");
            }
        }
        previous = Some((name_index, name_len, name_bytes.to_vec()));

        let value = if value_inode == 0 {
            let value_size = usize::try_from(value_size).map_err(|err| {
                ext4_error(format!(
                    "inode {inode} attribute value size does not fit in memory: {err:?}"
                ))
            })?;

            let value_start = base.checked_add(value_off as usize).ok_or_else(|| {
                ext4_error(format!("inode {inode} attribute value offset overflows"))
            })?;

            let value_end = value_start.checked_add(value_size).ok_or_else(|| {
                ext4_error(format!("inode {inode} attribute value offset overflows"))
            })?;

            if value_end > data.len() {
                return Err(ext4_error(format!(
                    "inode {inode} attribute value at {value_start} extends past the buffer"
                )));
            }
            value_at.push((value_start, value_size));
            data[value_start..value_end].to_vec()
        } else {
            Vec::new()
        };

        attributes.push(ExtendedAttribute {
            name: full_name(name_index, name_bytes),
            value,
            value_inode,
            value_size,
        });

        offset = next;
    }

    if !found_end {
        warn!("inode {inode} attribute list is truncated");
    }

    if value_at
        .iter()
        .any(|(start, size)| *size > 0 && *start < offset)
    {
        warn!("inode {inode} attribute value overlaps the entry list");
    }

    Ok(attributes)
}

/// Get the attribute name based on index value
fn full_name(index: u8, name: &[u8]) -> Vec<u8> {
    let prefix: &[u8] = match index {
        0 => b"",
        1 => b"user.",
        2 => b"system.posix_acl_access",
        3 => b"system.posix_acl_default",
        4 => b"trusted.",
        6 => b"security.",
        7 => b"system.",
        8 => b"system.richacl",
        _ => {
            debug!("Unknown index: {index} with name size {}", name.len());
            b""
        }
    };

    let mut attr_name = Vec::with_capacity(prefix.len() + name.len());
    attr_name.extend_from_slice(prefix);
    attr_name.extend_from_slice(name);

    attr_name
}

#[cfg(test)]
mod tests {
    use super::{
        ExtendedAttribute, crc32c_check, parse_attribute_block, parse_inline_attributes,
        verify_attribute_block,
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
        index: u8,
        value_off: u16,
        value_size: u32,
        name: &[u8],
    ) {
        data[offset] = u8::try_from(name.len()).unwrap();
        data[offset + 1] = index;

        put_u16(data, offset + 2, value_off);
        put_u32(data, offset + 4, 0);
        put_u32(data, offset + 8, value_size);
        put_u32(data, offset + 12, 0);

        data[offset + 16..offset + 16 + name.len()].copy_from_slice(name);
    }

    fn inline_user_small() -> Vec<u8> {
        let mut data = vec![0_u8; 64];
        put_u32(&mut data, 0, 0xea020000);

        write_entry(&mut data, 4, 1, 52, 5, b"small");
        data[56..61].copy_from_slice(b"hello");
        data
    }

    #[test]
    fn test_parse_inline_attributes_reads_a_user_name() {
        let attributes = parse_inline_attributes(12, &inline_user_small()).unwrap();
        assert_eq!(
            attributes,
            vec![ExtendedAttribute {
                name: b"user.small".to_vec(),
                value: b"hello".to_vec(),
                value_inode: 0,
                value_size: 5,
            }]
        );
    }

    #[test]
    fn test_parse_inline_attributes_returns_nothing_for_zeros() {
        let data = vec![0_u8; 32];
        assert!(parse_inline_attributes(12, &data).unwrap().is_empty());
    }

    #[test]
    fn test_parse_inline_attributes_rejects_a_bad_magic() {
        let mut data = inline_user_small();
        data[0] = 1;
        let reason = ext4_reason(parse_inline_attributes(12, &data));
        assert!(reason.contains("magic"), "{reason}");
    }

    #[test]
    fn test_parse_inline_attributes_rejects_a_value_inode() {
        let mut data = inline_user_small();
        put_u32(&mut data, 8, 99);
        let value = parse_inline_attributes(12, &data).unwrap();

        assert_eq!(value[0].name, b"user.small");
        assert!(value[0].value.is_empty());
        assert_eq!(value[0].value_inode, 99);
        assert_eq!(value[0].value_size, 5);
    }

    #[test]
    fn test_parse_attribute_block_returns_unsorted_entries() {
        let mut data = vec![0_u8; 128];
        put_u32(&mut data, 0, 0xea020000);
        put_u32(&mut data, 4, 1);
        put_u32(&mut data, 8, 1);

        write_entry(&mut data, 32, 1, 112, 4, b"b");
        write_entry(&mut data, 52, 1, 116, 4, b"a");

        data[112..116].copy_from_slice(b"beta");
        data[116..120].copy_from_slice(b"able");

        let attributes = parse_attribute_block(4, &data).unwrap();
        assert_eq!(
            attributes,
            vec![
                ExtendedAttribute {
                    name: b"user.b".to_vec(),
                    value: b"beta".to_vec(),
                    value_inode: 0,
                    value_size: 4,
                },
                ExtendedAttribute {
                    name: b"user.a".to_vec(),
                    value: b"able".to_vec(),
                    value_inode: 0,
                    value_size: 4,
                },
            ]
        );
    }

    #[test]
    fn test_verify_attribute_block_checks_the_checksum() {
        let superblock = superblock();
        let seed = superblock.checksum_seed().unwrap();
        let mut data = vec![0_u8; 128];

        put_u32(&mut data, 0, 0xea020000);
        put_u32(&mut data, 4, 1);
        put_u32(&mut data, 8, 1);

        write_entry(&mut data, 32, 1, 112, 4, b"a");
        write_entry(&mut data, 52, 1, 116, 4, b"b");

        data[112..116].copy_from_slice(b"able");
        data[116..120].copy_from_slice(b"beta");

        let block_number = 50_u64;
        let mut actual = crc32c_check(seed, &block_number.to_le_bytes());
        actual = crc32c_check(actual, &data);
        put_u32(&mut data, 16, actual);

        verify_attribute_block(&superblock, Some(seed), 4, block_number, &data).unwrap();
        let attributes = parse_attribute_block(4, &data).unwrap();
        assert_eq!(
            attributes,
            vec![
                ExtendedAttribute {
                    name: b"user.a".to_vec(),
                    value: b"able".to_vec(),
                    value_inode: 0,
                    value_size: 4,
                },
                ExtendedAttribute {
                    name: b"user.b".to_vec(),
                    value: b"beta".to_vec(),
                    value_inode: 0,
                    value_size: 4,
                },
            ]
        );

        data[112] ^= 0xff;
        let reason = ext4_reason(verify_attribute_block(
            &superblock,
            Some(seed),
            4,
            block_number,
            &data,
        ));
        assert!(reason.contains("checksum mismatch"), "{reason}");
    }

    #[test]
    fn test_verify_attribute_block_skips_checksums_when_metadata_csum_is_off() {
        let mut superblock = superblock();
        superblock.read_only_features &= !SuperBlock::METADATA_CSUM;
        let data = vec![0_u8; 32];

        verify_attribute_block(&superblock, None, 1, 50, &data).unwrap();
    }

    #[test]
    fn test_parse_inline_attributes_uses_the_name_index() {
        let cases: [(u8, &[u8], &[u8]); 8] = [
            (0, b"small", b"small"),
            (1, b"small", b"user.small"),
            (2, b"", b"system.posix_acl_access"),
            (3, b"", b"system.posix_acl_default"),
            (4, b"small", b"trusted.small"),
            (6, b"small", b"security.small"),
            (7, b"small", b"system.small"),
            (8, b"", b"system.richacl"),
        ];

        for (index, name, full) in cases {
            let mut data = vec![0_u8; 64];
            put_u32(&mut data, 0, 0xea020000);
            write_entry(&mut data, 4, index, 52, 5, name);
            data[56..61].copy_from_slice(b"hello");

            let attributes = parse_inline_attributes(12, &data).unwrap();
            assert_eq!(attributes[0].name, full);
        }
    }

    #[test]
    fn test_parse_inline_attributes_keeps_an_unknown_index() {
        let mut data = inline_user_small();
        data[5] = 5;

        let attributes = parse_inline_attributes(12, &data).unwrap();
        assert_eq!(attributes[0].name, b"small");
    }

    #[test]
    fn test_parse_inline_attributes_keeps_a_nonzero_hash() {
        let mut data = inline_user_small();
        put_u32(&mut data, 16, 0x1111);

        let attributes = parse_inline_attributes(12, &data).unwrap();
        assert_eq!(attributes[0].value, b"hello");
    }
}
