use super::block::{parse_block_bytes, BlockValue};
use crate::{
    artifacts::os::windows::outlook::{
        blocks::{block::Block, descriptors::parse_descriptor_block},
        error::OutlookError,
        header::FormatType,
        pages::btree::LeafBlockData,
    },
    filesystem::ntfs::reader::read_bytes,
    utils::nom_helper::{
        nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_one_byte,
        nom_unsigned_two_bytes, Endian,
    },
};
use log::warn;
use ntfs::NtfsFile;
use std::{collections::BTreeMap, io::BufReader};

/// Get block data from xblocks
pub(crate) fn parse_xblock<T: std::io::Seek + std::io::Read>(
    ntfs_file: Option<&NtfsFile<'_>>,
    fs: &mut BufReader<T>,
    block: &LeafBlockData,
    other_blocks: &[BTreeMap<u64, LeafBlockData>],
    format: &FormatType,
    block_value: &mut BlockValue,
) -> Result<(), OutlookError> {
    let size = if format != &FormatType::Unicode64_4k {
        64
    } else {
        512
    };

    println!("block type: {:?}", block.block_type);
    println!("block offset: {}", block.block_offset);
    // Need to align block size based on Outlook file format
    let mut alignment_size = (size - block.size % size) % size;
    if alignment_size == 0 {
        // If the actual data is perfectly aligned then we need to add another block
        alignment_size = size;
    }

    let footer_size = 24;
    // If alignment is less than footer size. Then footer is stored in next block
    if alignment_size < footer_size {
        alignment_size += size;
    }
    let bytes = read_bytes(
        &block.block_offset,
        block.size as u64 + alignment_size as u64,
        ntfs_file,
        fs,
    )
    .unwrap();

    let (_, entries) = xblock_data(&bytes, format, block_value).unwrap();
    let mut all_bytes = Vec::new();
    for entry in entries {
        for tree in other_blocks {
            if let Some(value) = tree.get(&entry) {
                alignment_size = (size - value.size % size) % size;
                if alignment_size == 0 {
                    // If the actual data is perfectly aligned then we need to add another block
                    alignment_size = size;
                }
                if alignment_size < footer_size {
                    alignment_size += size;
                }
                let bytes = read_bytes(
                    &value.block_offset,
                    value.size as u64 + alignment_size as u64,
                    ntfs_file,
                    fs,
                )
                .unwrap();

                let (_, block_data) = parse_block_bytes(&bytes, format).unwrap();
                all_bytes.push(block_data.data);
            }
        }
    }

    if block_value.block_type == Block::Unknown {
        block_value.data = all_bytes;
        block_value.block_type = Block::Xblock;
    }

    Ok(())
}

/// Parse xblock like data
fn xblock_data<'a>(
    data: &'a [u8],
    format: &FormatType,
    block_value: &mut BlockValue,
) -> nom::IResult<&'a [u8], Vec<u64>> {
    println!("block value: {block_value:?}");
    let (_, sig) = nom_unsigned_one_byte(data, Endian::Le)?;
    let sblock_sig = 2;
    if sig == sblock_sig {
        let (input, descriptor_tree) = parse_descriptor_block(data, format)?;
        block_value.block_type = Block::Descriptors;
        block_value.descriptors = descriptor_tree;
        return Ok((input, Vec::new()));
    } else if sig != 1 {
        // Its a raw block.
        let (_, block) = parse_block_bytes(data, format)?;
        if let Some(sig) = block.data.get(0) {
            if sig == &sblock_sig {
                let (_, descriptor_tree) = parse_descriptor_block(&block.data, format).unwrap();
                block_value.block_type = Block::Descriptors;
                block_value.descriptors = descriptor_tree;
                return Ok((&[], Vec::new()));
            } else if sig == &1 {
                let (_, result) = extract_xblock_entries(&block.data, format).unwrap();
                return Ok((&[], result));
            } else {
                block_value.block_type = Block::Raw;
                //block_value.data.push(block.data);
                panic!("got a raw block what todo: {:?}", block.data);
                return Ok((&[], Vec::new()));
            }
        }
    }
    return extract_xblock_entries(data, format);
}

/// Extract xblock and xxblock entries
fn extract_xblock_entries<'a>(
    data: &'a [u8],
    format: &FormatType,
) -> nom::IResult<&'a [u8], Vec<u64>> {
    let (input, _sig) = nom_unsigned_one_byte(data, Endian::Le)?;
    let (input, array_level) = nom_unsigned_one_byte(input, Endian::Le)?;

    if array_level != 1 {
        warn!("[outlook] Got possible xxblock. Level: {array_level}. Should be same format as xblock?");
    }
    let (input, number_entries) = nom_unsigned_two_bytes(input, Endian::Le)?;

    let (mut input, _total_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let mut count = 0;

    let mut entries = Vec::new();
    while count < number_entries {
        let (remaining, value) = if format == &FormatType::ANSI32 {
            let (bytes, value) = nom_unsigned_four_bytes(input, Endian::Le)?;
            (bytes, value as u64)
        } else {
            nom_unsigned_eight_bytes(input, Endian::Le)?
        };
        input = remaining;
        entries.push(value);
        count += 1;
    }

    Ok((input, entries))
}

#[cfg(test)]
mod tests {
    use super::{extract_xblock_entries, parse_xblock, xblock_data};
    use crate::{
        artifacts::os::windows::outlook::{
            blocks::block::{Block, BlockValue},
            header::FormatType,
            pages::btree::{get_block_btree, BlockType, LeafBlockData},
        },
        filesystem::files::file_reader,
    };
    use std::{collections::BTreeMap, io::BufReader, path::PathBuf};

    #[test]
    fn test_parse_xblock() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/outlook/windows11/test@outlook.com.ost");

        let reader = file_reader(test_location.to_str().unwrap()).unwrap();
        let mut buf_reader = BufReader::new(reader);
        let mut tree = Vec::new();

        get_block_btree(
            None,
            &mut buf_reader,
            &475136,
            &4096,
            &FormatType::Unicode64_4k,
            &mut tree,
        )
        .unwrap();

        let test = LeafBlockData {
            index_id: 22,
            block_type: BlockType::Internal,
            index: 5,
            block_offset: 152576,
            size: 56,
            total_size: 56,
            reference_count: 44,
        };

        let mut block = BlockValue {
            block_type: Block::Unknown,
            data: Vec::new(),
            descriptors: BTreeMap::new(),
        };

        parse_xblock(
            None,
            &mut buf_reader,
            &test,
            &tree,
            &FormatType::Unicode64_4k,
            &mut block,
        )
        .unwrap();

        assert!(block.data.is_empty());
    }

    #[test]
    fn test_xblock_data() {
        let test = [
            1, 1, 2, 0, 50, 158, 1, 0, 36, 44, 0, 0, 0, 0, 0, 0, 44, 44, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 24, 0,
            45, 236, 141, 12, 116, 94, 42, 44, 0, 0, 0, 0, 0, 0, 2, 0, 24, 0, 0, 0, 0, 0,
        ];

        let mut block = BlockValue {
            block_type: Block::Unknown,
            data: Vec::new(),
            descriptors: BTreeMap::new(),
        };

        let (_, entries) = xblock_data(&test, &FormatType::Unicode64_4k, &mut block).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    #[should_panic(expected = "Error")]
    fn test_extract_xblock_entries() {
        let test = [2, 1, 2, 0, 0];
        let _ = extract_xblock_entries(&test, &FormatType::Unicode64_4k).unwrap();
    }

    #[test]
    fn test_xblock_data_raw() {
        let test = [
            120, 156, 93, 209, 173, 10, 194, 80, 28, 134, 241, 63, 194, 22, 77, 178, 36, 88, 60,
            126, 84, 89, 146, 129, 105, 183, 96, 16, 65, 16, 6, 86, 171, 136, 168, 184, 160, 73,
            188, 4, 113, 85, 22, 13, 70, 87, 12, 226, 77, 172, 136, 201, 100, 48, 248, 113, 30,
            195, 206, 41, 63, 222, 39, 12, 206, 89, 78, 28, 249, 158, 161, 253, 67, 94, 117, 201,
            156, 13, 253, 84, 201, 118, 53, 213, 222, 233, 55, 244, 233, 197, 170, 214, 193, 128,
            238, 177, 155, 24, 210, 7, 236, 62, 70, 244, 53, 123, 133, 9, 61, 102, 239, 49, 165,
            95, 217, 23, 180, 102, 218, 66, 77, 155, 71, 69, 119, 217, 13, 244, 233, 93, 118, 27,
            3, 250, 152, 61, 194, 144, 238, 241, 110, 54, 70, 255, 239, 176, 59, 152, 208, 23, 236,
            9, 166, 244, 29, 123, 139, 214, 156, 123, 177, 207, 168, 232, 79, 246, 3, 75, 75, 109,
            108, 252, 175, 22, 253, 96, 244, 222, 167, 151, 197, 149, 163, 209, 223, 90, 109, 46,
            28, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 189, 0, 49, 100, 171, 48, 73, 18, 2, 42, 0, 0, 0, 0, 0,
            0, 2, 0, 24, 2, 0, 0, 0, 0,
        ];

        let mut block = BlockValue {
            block_type: Block::Unknown,
            data: Vec::new(),
            descriptors: BTreeMap::new(),
        };

        let (_, results) = xblock_data(&test, &FormatType::Unicode64_4k, &mut block).unwrap();
        assert!(results.is_empty());
    }
}
