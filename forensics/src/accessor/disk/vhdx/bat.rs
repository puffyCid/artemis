use crate::{
    accessor::{
        disk::vhdx::metadata::VhdxMetadata,
        error::{AccessorError, AccessorResult},
    },
    utils::nom_helper::nom_u64,
};

/// Map reads to VHDX guest
#[derive(Debug, Clone, PartialEq)]
pub(super) enum VhdxMapping {
    /// The next guest read are zeros
    Zero {
        /// Guest bytes covered
        ///
        /// These values are zeros
        length: u64,
    },
    /// The next guest read bytes return data
    Data {
        /// Offset in the VHDX file
        file_offset: u64,
        /// Guest bytes covered
        length: u64,
    },
}

/// Sector bitmap block values
#[derive(Debug, Clone, PartialEq)]
enum PayloadBlock {
    /// Block is new and not stored in the file
    Absent,
    /// Block is stored in the file
    Present {
        /// Offset to the block
        file_offset: u64,
    },
}

/// Parsed Block allocation table (BAT)
///
/// <https://github.com/libyal/libvhdi/blob/main/documentation/Virtual%20Hard%20Disk%20version%202%20(VHDX)%20image%20format.asciidoc#6-block-allocation-table-bat-region>
#[derive(Debug, Clone, PartialEq)]
pub(super) struct VhdxBat {
    /// Size of the block
    block_size: u64,
    /// Size of the virtal disk
    virtual_disk_size: u64,
    /// Parsed blocks
    blocks: Vec<PayloadBlock>,
}

impl VhdxBat {
    /// Map guest byte offset to VDHX file offset
    pub(super) fn map_offset(&self, guest_offset: u64) -> AccessorResult<VhdxMapping> {
        if guest_offset >= self.virtual_disk_size {
            return Err(AccessorError::volume(format!(
                "VHDX read offset {guest_offset} is past the virtual disk"
            )));
        }

        let block_number = guest_offset / self.block_size;
        let block_start = block_number.checked_mul(self.block_size).ok_or_else(|| {
            AccessorError::volume(format!("VHDX block {block_number} offset overflows"))
        })?;

        let offset_in_block = guest_offset - block_start;
        let block_end = block_start
            .saturating_add(self.block_size)
            .min(self.virtual_disk_size);

        let length = block_end - guest_offset;
        let block = self
            .blocks
            .get(usize::try_from(block_number).unwrap_or(usize::MAX))
            .ok_or_else(|| {
                AccessorError::volume(format!("VHDX BAT is missing block {block_number}"))
            })?;

        match block {
            PayloadBlock::Absent => Ok(VhdxMapping::Zero { length }),
            PayloadBlock::Present { file_offset } => Ok(VhdxMapping::Data {
                file_offset: *file_offset + offset_in_block,
                length,
            }),
        }
    }
}

/// Parse a BAT entry
pub(super) fn parse_bat(
    data: &[u8],
    metadata: &VhdxMetadata,
    file_len: u64,
    occupied: &[(u64, u64)],
) -> AccessorResult<VhdxBat> {
    let block_size = metadata.block_size as u64;
    if block_size == 0 {
        return Err(AccessorError::volume("VHDX block size 0 is invalid"));
    }

    let payload_blocks = metadata.virtual_disk_size.div_ceil(block_size);
    let entry_count = bat_entry_count(metadata, payload_blocks)?;
    let table_bytes = entry_count
        .checked_mul(8)
        .ok_or_else(|| AccessorError::volume("VHDX BAT entry count overflows"))?;

    let max_bat_bytes = 32 * 1024 * 1024;
    if table_bytes > max_bat_bytes {
        return Err(AccessorError::volume(format!(
            "VHDX BAT is {table_bytes} bytes, above 32 MiB"
        )));
    }

    let data_len = u64::try_from(data.len())
        .map_err(|err| AccessorError::volume(format!("VHDX BAT is too large to parse: {err:?}")))?;
    if data_len < table_bytes {
        return Err(AccessorError::volume(format!(
            "VHDX BAT is {data_len} bytes. Wanted at least {table_bytes}"
        )));
    }

    let mut blocks = Vec::with_capacity(usize::try_from(payload_blocks).unwrap_or(usize::MAX));
    let mut present = Vec::new();
    let mut index = 0;

    let mut input = data;
    if metadata.leave_blocks_allocated {
        for _ in 0..payload_blocks {
            let (remaining, raw) = nom_u64(input, "VHDX BAT entry truncated")?;
            blocks.push(parse_payload(
                raw,
                index,
                block_size,
                file_len,
                occupied,
                &mut present,
            )?);
            index += 1;
            input = remaining;
        }
    } else {
        let chunk_ratio = metadata.chunk_ratio as u64;
        let mut remaining = payload_blocks;
        while remaining > 0 {
            let chunk = remaining.min(chunk_ratio);
            for _ in 0..chunk {
                let (remaining_input, raw) = nom_u64(input, "VHDX BAT entry truncated")?;
                blocks.push(parse_payload(
                    raw,
                    index,
                    block_size,
                    file_len,
                    occupied,
                    &mut present,
                )?);
                index += 1;
                input = remaining_input;
            }
            let (remaining_input, raw) = nom_u64(input, "VHDX BAT entry truncated")?;
            parse_bitmap(raw, index, file_len, occupied, &mut present)?;
            index += 1;
            remaining -= chunk;
            input = remaining_input;
        }
    }

    if index != entry_count {
        return Err(AccessorError::volume(
            "VHDX BAT entry count does not match the virtual disk",
        ));
    }

    Ok(VhdxBat {
        block_size,
        virtual_disk_size: metadata.virtual_disk_size,
        blocks,
    })
}

/// Number of BAT chunks
fn bat_entry_count(metadata: &VhdxMetadata, payload_blocks: u64) -> AccessorResult<u64> {
    if metadata.leave_blocks_allocated {
        return Ok(payload_blocks);
    }

    let chunk_ratio = metadata.chunk_ratio as u64;
    if chunk_ratio == 0 {
        return Err(AccessorError::volume("VHDX chunk ratio is zero"));
    }

    let bitmap_blocks = payload_blocks.div_ceil(chunk_ratio);
    payload_blocks
        .checked_add(bitmap_blocks)
        .ok_or_else(|| AccessorError::volume("VHDX BAT entry count overflows"))
}

/// Determine BAT payload and bitmap entry based on bits
///
/// <https://github.com/libyal/libvhdi/blob/main/documentation/Virtual%20Hard%20Disk%20version%202%20(VHDX)%20image%20format.asciidoc#61-block-allocation-table-bat-entry>
fn split_entry(raw: u64, index: u64) -> AccessorResult<(u64, u64)> {
    // block state
    // 6 types <https://github.com/libyal/libvhdi/blob/main/documentation/Virtual%20Hard%20Disk%20version%202%20(VHDX)%20image%20format.asciidoc#61-block-allocation-table-bat-entry>
    let state = raw & 7;
    let reserved = (raw >> 3) & 0x1ffff;
    if reserved != 0 {
        return Err(AccessorError::volume(format!(
            "VHDX BAT entry {index} has reserved bits set"
        )));
    }

    Ok((state, raw & !0xfffff))
}

/// Parse BAT payload data
fn parse_payload(
    raw: u64,
    index: u64,
    block_size: u64,
    file_len: u64,
    occupied: &[(u64, u64)],
    present: &mut Vec<(u64, u64)>,
) -> AccessorResult<PayloadBlock> {
    // Determine state of block and file offset
    let (state, file_offset) = split_entry(raw, index)?;
    match state {
        /*
         * All absent or unused payload data
         * 0 - Not Present
         * 1 - Undefined
         * 2 - Zero
         * 3 - Unampped
         */
        0..=3 => {
            if file_offset != 0 {
                return Err(AccessorError::volume(format!(
                    "VHDX BAT entry {index} offset {file_offset} must be zero"
                )));
            }
            Ok(PayloadBlock::Absent)
        }
        // 6 - Fully Present
        6 => {
            validate_present(index, file_offset, block_size, file_len, occupied, present)?;
            Ok(PayloadBlock::Present { file_offset })
        }
        // Partially present
        7 => Err(AccessorError::volume(format!(
            "VHDX BAT entry {index} is partially present"
        ))),
        _ => Err(AccessorError::volume(format!(
            "VHDX BAT entry {index} state {state} is invalid"
        ))),
    }
}

/// Parse BAT bitmap states
///
/// <https://github.com/libyal/libvhdi/blob/main/documentation/Virtual%20Hard%20Disk%20version%202%20(VHDX)%20image%20format.asciidoc#61-block-allocation-table-bat-entry>
fn parse_bitmap(
    raw: u64,
    index: u64,
    file_len: u64,
    occupied: &[(u64, u64)],
    present: &mut Vec<(u64, u64)>,
) -> AccessorResult<()> {
    let (state, file_offset) = split_entry(raw, index)?;
    match state {
        // Block is new and not stored
        0 => {
            if file_offset != 0 {
                return Err(AccessorError::volume(format!(
                    "VHDX BAT entry {index} offset {file_offset} must be zero"
                )));
            }

            Ok(())
        }
        // Block is stored in file
        6 => validate_present(index, file_offset, 1048576, file_len, occupied, present),
        _ => Err(AccessorError::volume(format!(
            "VHDX BAT bitmap entry {index} state {state} is invalid"
        ))),
    }
}

/// Make sure a fully present payload state (6)
/// is at least 1MB in size, within the file size, and
/// does not overlap with other sections
fn validate_present(
    index: u64,
    file_offset: u64,
    length: u64,
    file_len: u64,
    occupied: &[(u64, u64)],
    present: &mut Vec<(u64, u64)>,
) -> AccessorResult<()> {
    if file_offset < 1048576 {
        return Err(AccessorError::volume(format!(
            "VHDX BAT entry {index} offset {file_offset} is inside the first 1 MiB"
        )));
    }

    let end = file_offset
        .checked_add(length)
        .ok_or_else(|| AccessorError::volume(format!("VHDX BAT entry {index} range overflows")))?;

    if end > file_len {
        return Err(AccessorError::volume(format!(
            "VHDX BAT entry {index} extends past the file"
        )));
    }

    let overlaps = occupied
        .iter()
        .chain(present.iter())
        .any(|&(start, len)| ranges_overlap(file_offset, length, start, len));

    if overlaps {
        return Err(AccessorError::volume(format!(
            "VHDX BAT entry {index} overlaps another range"
        )));
    }

    present.push((file_offset, length));

    Ok(())
}

/// Compare section ranges
fn ranges_overlap(start: u64, length: u64, other: u64, other_length: u64) -> bool {
    let Some(end) = start.checked_add(length) else {
        return true;
    };

    let Some(other_end) = other.checked_add(other_length) else {
        return true;
    };

    start < other_end && other < end
}

#[cfg(test)]
mod test {
    use super::{VhdxMapping, parse_bat};
    use crate::accessor::{
        disk::vhdx::metadata::{VhdxMetadata, parse_metadata},
        error::{AccessorError, AccessorResult},
    };
    use std::{fs::read, path::PathBuf};
    use uuid::Uuid;

    fn metadata(
        block_size: u32,
        virtual_disk_size: u64,
        fixed: bool,
        chunk_ratio: u32,
    ) -> VhdxMetadata {
        VhdxMetadata {
            block_size,
            leave_blocks_allocated: fixed,
            virtual_disk_size,
            disk_id: Uuid::nil(),
            logical_sector_size: 512,
            physical_sector_size: 512,
            chunk_ratio,
        }
    }

    fn bat_entry(state: u64, file_offset: u64) -> u64 {
        (file_offset & !0xF_FFFF) | (state & 7)
    }

    fn bat_bytes(entries: &[u64]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(entries.len() * 8);
        for entry in entries {
            bytes.extend(entry.to_le_bytes());
        }
        bytes
    }

    fn volume_reason<T>(result: AccessorResult<T>) -> String {
        match result {
            Err(AccessorError::Volume { reason }) => reason,
            Err(err) => panic!("expected a volume error, got {err}"),
            Ok(_) => panic!("expected a volume error"),
        }
    }

    #[test]
    fn test_parse_bat_fixture() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/disks/vhdx/ntfs.vhdx");
        let bytes = read(path).unwrap();
        let metadata = parse_metadata(&bytes[0x300000..0x300000 + 0x11000]).unwrap();
        let occupied = [(0, 0x100000), (0x200000, 0x100000), (0x300000, 0x100000)];
        let bat = parse_bat(
            &bytes[0x200000..0x300000],
            &metadata,
            bytes.len() as u64,
            &occupied,
        )
        .unwrap();

        assert_eq!(
            bat.map_offset(0).unwrap(),
            VhdxMapping::Data {
                file_offset: 0x800000,
                length: 8000000,
            }
        );

        assert_eq!(
            bat.map_offset(100).unwrap(),
            VhdxMapping::Data {
                file_offset: 0x800000 + 100,
                length: 8000000 - 100,
            }
        );
        let reason = volume_reason(bat.map_offset(8000000));
        assert!(reason.contains("past the virtual disk"), "{reason}");
    }

    #[test]
    fn test_parse_bat_zero_states_and_present_block() {
        let block_size = 1024 * 1024;
        let zero_states = metadata(block_size, block_size as u64 * 4, false, 4);
        let bytes = bat_bytes(&[
            bat_entry(0, 0),
            bat_entry(1, 0),
            bat_entry(2, 0),
            bat_entry(3, 0),
            bat_entry(0, 0),
        ]);

        let bat = parse_bat(&bytes, &zero_states, 0, &[]).unwrap();
        assert_eq!(
            bat.map_offset(0).unwrap(),
            VhdxMapping::Zero {
                length: block_size as u64,
            }
        );
        assert_eq!(
            bat.map_offset(block_size as u64 * 3 + 10).unwrap(),
            VhdxMapping::Zero {
                length: block_size as u64 - 10,
            }
        );

        let present = metadata(block_size, 1000, false, 512);
        let bytes = bat_bytes(&[bat_entry(6, block_size as u64), bat_entry(0, 0)]);
        let bat = parse_bat(&bytes, &present, block_size as u64 * 2, &[]).unwrap();

        assert_eq!(
            bat.map_offset(0).unwrap(),
            VhdxMapping::Data {
                file_offset: block_size as u64,
                length: 1000,
            }
        );
    }

    #[test]
    fn test_parse_bat_fixed_and_dynamic_indexes() {
        let block_size = 1024 * 1024;
        let fixed = metadata(block_size, block_size as u64 * 2, true, 0);
        let bytes = bat_bytes(&[
            bat_entry(6, block_size as u64),
            bat_entry(6, block_size as u64 * 2),
        ]);

        let bat = parse_bat(&bytes, &fixed, block_size as u64 * 3, &[]).unwrap();
        assert_eq!(
            bat.map_offset(block_size as u64).unwrap(),
            VhdxMapping::Data {
                file_offset: block_size as u64 * 2,
                length: block_size as u64,
            }
        );

        let block_size = 256 * 1024 * 1024;
        let dynamic = metadata(block_size, block_size as u64 * 17, false, 16);
        let mut entries = vec![bat_entry(0, 0); 19];

        entries[17] = bat_entry(6, 1048576);
        let bat = parse_bat(
            &bat_bytes(&entries),
            &dynamic,
            1048576 + block_size as u64,
            &[],
        )
        .unwrap();

        assert_eq!(
            bat.map_offset(block_size as u64 * 16).unwrap(),
            VhdxMapping::Data {
                file_offset: 1048576,
                length: block_size as u64,
            }
        );
    }

    #[test]
    fn test_parse_bat_accepts_a_present_bitmap() {
        let block_size = 1024 * 1024;
        let metadata = metadata(block_size, block_size as u64, false, 512);
        let bytes = bat_bytes(&[
            bat_entry(6, block_size as u64),
            bat_entry(6, block_size as u64 * 3),
        ]);

        let bat = parse_bat(&bytes, &metadata, block_size as u64 * 4, &[]).unwrap();
        assert_eq!(
            bat.map_offset(0).unwrap(),
            VhdxMapping::Data {
                file_offset: block_size as u64,
                length: block_size as u64,
            }
        );
    }

    #[test]
    fn test_parse_bat_rejects_bad_entries() {
        let block_size = 1024 * 1024;
        let one_block = metadata(block_size, block_size as u64, false, 512);
        let file_len = block_size as u64 * 2;

        let bytes = bat_bytes(&[bat_entry(7, block_size as u64), bat_entry(0, 0)]);
        let reason = volume_reason(parse_bat(&bytes, &one_block, file_len, &[]));
        assert!(reason.contains("partially present"), "{reason}");

        let bytes = bat_bytes(&[bat_entry(6, block_size as u64) | (1 << 3), bat_entry(0, 0)]);

        let reason = volume_reason(parse_bat(&bytes, &one_block, file_len, &[]));
        assert!(reason.contains("reserved"), "{reason}");

        let bytes = bat_bytes(&[bat_entry(0, block_size as u64), bat_entry(0, 0)]);
        let reason = volume_reason(parse_bat(&bytes, &one_block, file_len, &[]));
        assert!(reason.contains("must be zero"), "{reason}");

        let bytes = bat_bytes(&[bat_entry(6, 0), bat_entry(0, 0)]);
        let reason = volume_reason(parse_bat(&bytes, &one_block, file_len, &[]));
        assert!(reason.contains("first 1 MiB"), "{reason}");

        let bytes = bat_bytes(&[bat_entry(6, block_size as u64), bat_entry(0, 0)]);
        let reason = volume_reason(parse_bat(&bytes, &one_block, block_size as u64, &[]));
        assert!(reason.contains("past the file"), "{reason}");

        let bytes = bat_bytes(&[bat_entry(4, 0), bat_entry(0, 0)]);
        let reason = volume_reason(parse_bat(&bytes, &one_block, file_len, &[]));
        assert!(reason.contains("state 4"), "{reason}");

        let bytes = bat_bytes(&[bat_entry(6, block_size as u64), bat_entry(1, 0)]);
        let reason = volume_reason(parse_bat(&bytes, &one_block, file_len, &[]));
        assert!(reason.contains("bitmap"), "{reason}");

        let fixed = metadata(2 * 1024 * 1024, 4 * 1024 * 1024, true, 0);
        let bytes = bat_bytes(&[
            bat_entry(6, block_size as u64),
            bat_entry(6, 2 * 1024 * 1024),
        ]);
        let reason = volume_reason(parse_bat(&bytes, &fixed, 4 * 1024 * 1024, &[]));
        assert!(reason.contains("overlaps"), "{reason}");

        let bytes = bat_bytes(&[bat_entry(6, 0x200000), bat_entry(0, 0)]);
        let occupied = [(0x200000_u64, 0x100000_u64)];
        let reason = volume_reason(parse_bat(
            &bytes,
            &one_block,
            0x200000 + block_size as u64,
            &occupied,
        ));
        assert!(reason.contains("overlaps"), "{reason}");

        let huge = metadata(1024 * 1024, 64 * 1024 * 1024 * 1024 * 1024, false, 4096);
        let reason = volume_reason(parse_bat(&[], &huge, 0, &[]));
        assert!(reason.contains("32 MiB"), "{reason}");

        let zero_chunk = metadata(block_size, block_size as u64, false, 0);
        let reason = volume_reason(parse_bat(&[], &zero_chunk, 0, &[]));
        assert!(reason.contains("chunk ratio"), "{reason}");
    }
}
