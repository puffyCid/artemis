use super::block::{parse_block_bytes, BlockData};
use crate::{
    artifacts::os::windows::outlook::{
        error::OutlookError, header::FormatType, pages::btree::LeafBlockData,
    },
    filesystem::ntfs::reader::read_bytes,
};
use ntfs::NtfsFile;
use std::io::BufReader;

pub(crate) fn parse_raw_block<T: std::io::Seek + std::io::Read>(
    ntfs_file: Option<&NtfsFile<'_>>,
    fs: &mut BufReader<T>,
    block: &LeafBlockData,
    format: &FormatType,
) -> Result<BlockData, OutlookError> {
    let size = if format != &FormatType::Unicode64_4k {
        64
    } else {
        512
    };

    // Need to align block size based on Outlook file format
    let alignment_size = (size - block.size % size) % size;
    println!("{alignment_size}");
    let bytes = read_bytes(
        &block.block_offset,
        (block.size + alignment_size) as u64,
        ntfs_file,
        fs,
    )
    .unwrap();

    let (_, block_data) = parse_block_bytes(&bytes, format).unwrap();
    println!("{block_data:?}");

    Ok(block_data)
}

#[cfg(test)]
mod tests {
    use super::parse_raw_block;
    use crate::{
        artifacts::os::windows::outlook::{
            header::FormatType,
            pages::btree::{BlockType, LeafBlockData},
        },
        filesystem::files::file_reader,
    };
    use std::{io::BufReader, path::PathBuf};

    #[test]
    fn test_parse_raw_block() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/outlook/windows11/block_raw.raw");
        let reader = file_reader(test_location.to_str().unwrap()).unwrap();

        let mut buf_reader = BufReader::new(reader);
        let test = LeafBlockData {
            index_id: 69820,
            block_type: BlockType::External,
            index: 1007681792,
            block_offset: 0,
            size: 456,
            reference_count: 1,
            file_offset_allocation_table: 2,
        };
        let block =
            parse_raw_block(None, &mut buf_reader, &test, &FormatType::Unicode64_4k).unwrap();

        assert_eq!(block.data.len(), 456);
        assert_eq!(block.block_size, 456);
        assert_eq!(block.sig, 63926);
        assert_eq!(block.crc, 3861511615);
        assert_eq!(block.back_pointer, 69820);
    }
}
