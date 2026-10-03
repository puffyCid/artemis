use crate::accessor::bootsector::gpt::GptPartition;
use nom::{
    bytes::complete::take,
    number::complete::{le_u8, le_u16, le_u32},
};
use tracing::warn;

#[derive(Debug)]
struct BootInfo {
    boot_type: BootType,
    partitions: Vec<Partition>,
    gpt_partitions: Option<Vec<GptPartition>>,
}

#[derive(Debug, PartialEq)]
enum BootType {
    MasterBootRecord,
    GuidPartitionTable,
}

#[derive(Debug, Clone)]
struct Partition {
    partition_type: PartitionType,
    partition_type_value: u8,
    first_sector_offset: u32,
    last_sector_offset: u32,
    first_logical_offset: u32,
    /**Offset to partition data */
    offset_start: u64,
    sectors_in_partition: u32,
    partition_size: u64,
    bootable: bool,
}

#[derive(PartialEq, Debug, Clone)]
enum PartitionType {
    Ntfs,
    Linux,
    Unknown,
    Fat16,
    Fat32,
    ExFat,
    Protective,
    Extended,
    LinuxSwap,
    LinuxLvm,
    None,
}

/// Parse the Master Boot Record (MBR) partition. We must be able to parse this in order to parse the rest of the filesystem
pub(crate) fn parse_mbr(data: &[u8]) -> nom::IResult<&[u8], BootInfo> {
    let boot_binary_code: u16 = 440;
    let (input, _binary) = take(boot_binary_code)(data)?;

    let (input, _disk_id) = le_u32(input)?;
    let (mut input, _reserved) = le_u16(input)?;

    let mut info = BootInfo {
        boot_type: BootType::MasterBootRecord,
        partitions: Vec::new(),
        gpt_partitions: None,
    };

    let partition_size: u8 = 16;
    let max_partitions = 4;
    let mut count = 0;

    // Last two partitions usually do not have anything
    while count < max_partitions {
        let (remaining, partition) = take(partition_size)(input)?;
        input = remaining;

        let (_, (part, is_gpt)) = parse_partition(partition)?;
        info.partitions.push(part);

        if is_gpt {
            info.boot_type = BootType::GuidPartitionTable;
        }
        count += 1;
    }
    let (input, _valid_bootsector) = le_u16(input)?;
    Ok((input, info))
}

/// Parse the partition data. It is very small, 16 bytes.
fn parse_partition(data: &[u8]) -> nom::IResult<&[u8], (Partition, bool)> {
    let (input, bootable) = le_u8(data)?;
    let sector_size: u8 = 3;
    let (input, sector_start) = take(sector_size)(input)?;
    let (input, partition_type) = le_u8(input)?;
    let (input, sector_last) = take(sector_size)(input)?;

    let (input, first_logical_offset) = le_u32(input)?;
    let (input, sectors_in_partition) = le_u32(input)?;
    // Safe to reference by slice index because we use nom to ensure our sector_last and sector_start length is at least 3 bytes in size
    let last_sector_offset =
        ((sector_last[0] as u32) << 16) + ((sector_last[1] as u32) << 8) + sector_last[2] as u32;
    let first_sector_offset =
        ((sector_start[0] as u32) << 16) + ((sector_start[1] as u32) << 8) + sector_start[2] as u32;

    let mut is_gpt = false;
    let part = Partition {
        partition_type: get_partition_type(partition_type),
        partition_type_value: partition_type,
        first_sector_offset,
        last_sector_offset,
        first_logical_offset,
        offset_start: first_logical_offset as u64 * 512,
        sectors_in_partition,
        partition_size: (sectors_in_partition as u64 * 512),
        bootable: bootable == 0x80,
    };

    if part.partition_type == PartitionType::Protective {
        is_gpt = true;
    }

    Ok((input, (part, is_gpt)))
}

/// Determine the partition type, only a few are supported right now
/// There are a lot: <https://en.wikipedia.org/wiki/Partition_type#List_of_partition_IDs>
fn get_partition_type(part: u8) -> PartitionType {
    match part {
        0x0 => PartitionType::None,
        0x7 | 0x27 => PartitionType::Ntfs,
        0x83 => PartitionType::Linux,
        0x82 => PartitionType::LinuxSwap,
        0x8e => PartitionType::LinuxLvm,
        0xc => PartitionType::Fat32,
        0xee | 0xef => PartitionType::Protective,
        0x5 | 0xf => PartitionType::Extended,
        _ => PartitionType::Unknown,
    }
}

/// Parse the extended partitions: <https://en.wikipedia.org/wiki/Extended_boot_record>
fn parse_extended(
    data: &[u8],
    root_offset: u64,
    extended_offset: u64,
) -> nom::IResult<&[u8], (Vec<Partition>, bool)> {
    let mut parts = Vec::new();

    // *may* contain another boot loader
    let unused: u16 = 446;
    let (input, _) = take(unused)(data)?;
    let entry_size: u8 = 16;
    let (input, first_entry) = take(entry_size)(input)?;
    let (_, (mut first_part, _)) = parse_partition(first_entry)?;

    let mut has_extened = false;
    // The first entry in an extended partition should never? be extended Type. But check just in case
    if first_part.partition_type == PartitionType::Extended {
        warn!(
            "The first MBR extended partition entry is an extended partition type. This should not happen? Got: {first_part:?}"
        );
        has_extened = true;
        first_part.offset_start += root_offset;
    }

    let (input, second_entry) = take(entry_size)(input)?;
    let (_, (mut extended_part, _)) = parse_partition(second_entry)?;

    // Extended partition only has two partitions. But technically allows 4?
    let (input, _third_entry) = take(entry_size)(input)?;
    let (input, _fourth_entry) = take(entry_size)(input)?;
    let (input, sig) = le_u16(input)?;

    let part_sig = 43605;
    if sig != part_sig {
        warn!("Did not get expected MBR extended signature. Expected 0xAA55, got: {sig}");
    }

    if extended_part.partition_type == PartitionType::Extended {
        has_extened = true;
    }

    // The last extended partition value will have None partition type
    if has_extened || extended_part.partition_type == PartitionType::None {
        // Add root offset to ensure we are using the absolute offset to the partition
        extended_part.offset_start += root_offset;
        // First entry offset combines the current extended partition offset and the relative offset
        first_part.offset_start =
            (first_part.first_logical_offset as u64 + extended_offset / 512) * 512;
    }
    parts.push(first_part);

    parts.push(extended_part);
    Ok((input, (parts, has_extened)))
}

#[cfg(test)]
mod tests {
    use crate::accessor::bootsector::mbr::{
        BootType, PartitionType, get_partition_type, parse_extended, parse_mbr, parse_partition,
    };
    use std::{fs::read, path::PathBuf};

    #[test]
    fn test_parse_extended() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/bootsector/mbr/extended_partition.raw");
        let bytes = read(test_location.to_str().unwrap()).unwrap();
        let (_, (results, has_extended)) = parse_extended(&bytes, 0, 0).unwrap();
        assert!(has_extended);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].partition_type, PartitionType::Linux);
        assert_eq!(results[0].offset_start, 1024);
        assert_eq!(results[1].offset_start, 10000111616);
        assert_eq!(results[1].partition_type, PartitionType::Extended);
    }

    #[test]
    fn test_get_partition_type() {
        let test = [0x0, 0x7, 0x27, 0x83, 0x82, 0x8e, 0xc, 0xee, 0xef, 0x5, 0xf];
        for entry in test {
            assert_ne!(get_partition_type(entry), PartitionType::Unknown);
        }
    }

    #[test]
    fn test_parse_parition() {
        let test = [128, 4, 1, 4, 131, 254, 194, 255, 0, 8, 0, 0, 0, 128, 224, 0];
        let (_, (result, is_gpt)) = parse_partition(&test).unwrap();
        assert_eq!(result.partition_size, 7532969984);
        assert_eq!(result.first_sector_offset, 262404);
        assert!(!is_gpt);
    }

    #[test]
    fn test_parse_mbr() {
        let test = [
            235, 99, 144, 16, 142, 208, 188, 0, 176, 184, 0, 0, 142, 216, 142, 192, 251, 190, 0,
            124, 191, 0, 6, 185, 0, 2, 243, 164, 234, 33, 6, 0, 0, 190, 190, 7, 56, 4, 117, 11,
            131, 198, 16, 129, 254, 254, 7, 117, 243, 235, 22, 180, 2, 176, 1, 187, 0, 124, 178,
            128, 138, 116, 1, 139, 76, 2, 205, 19, 234, 0, 124, 0, 0, 235, 254, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 1, 0, 0, 0, 0, 0, 0, 0, 255, 250, 144, 144, 246,
            194, 128, 116, 5, 246, 194, 112, 116, 2, 178, 128, 234, 121, 124, 0, 0, 49, 192, 142,
            216, 142, 208, 188, 0, 32, 251, 160, 100, 124, 60, 255, 116, 2, 136, 194, 82, 190, 128,
            125, 232, 23, 1, 190, 5, 124, 180, 65, 187, 170, 85, 205, 19, 90, 82, 114, 61, 129,
            251, 85, 170, 117, 55, 131, 225, 1, 116, 50, 49, 192, 137, 68, 4, 64, 136, 68, 255,
            137, 68, 2, 199, 4, 16, 0, 102, 139, 30, 92, 124, 102, 137, 92, 8, 102, 139, 30, 96,
            124, 102, 137, 92, 12, 199, 68, 6, 0, 112, 180, 66, 205, 19, 114, 5, 187, 0, 112, 235,
            118, 180, 8, 205, 19, 115, 13, 90, 132, 210, 15, 131, 216, 0, 190, 139, 125, 233, 130,
            0, 102, 15, 182, 198, 136, 100, 255, 64, 102, 137, 68, 4, 15, 182, 209, 193, 226, 2,
            136, 232, 136, 244, 64, 137, 68, 8, 15, 182, 194, 192, 232, 2, 102, 137, 4, 102, 161,
            96, 124, 102, 9, 192, 117, 78, 102, 161, 92, 124, 102, 49, 210, 102, 247, 52, 136, 209,
            49, 210, 102, 247, 116, 4, 59, 68, 8, 125, 55, 254, 193, 136, 197, 48, 192, 193, 232,
            2, 8, 193, 136, 208, 90, 136, 198, 187, 0, 112, 142, 195, 49, 219, 184, 1, 2, 205, 19,
            114, 30, 140, 195, 96, 30, 185, 0, 1, 142, 219, 49, 246, 191, 0, 128, 142, 198, 252,
            243, 165, 31, 97, 255, 38, 90, 124, 190, 134, 125, 235, 3, 190, 149, 125, 232, 52, 0,
            190, 154, 125, 232, 46, 0, 205, 24, 235, 254, 71, 82, 85, 66, 32, 0, 71, 101, 111, 109,
            0, 72, 97, 114, 100, 32, 68, 105, 115, 107, 0, 82, 101, 97, 100, 0, 32, 69, 114, 114,
            111, 114, 13, 10, 0, 187, 1, 0, 180, 14, 205, 16, 172, 60, 0, 117, 244, 195, 0, 0, 0,
            0, 0, 0, 0, 0, 2, 59, 99, 240, 0, 0, 128, 4, 1, 4, 131, 254, 194, 255, 0, 8, 0, 0, 0,
            128, 224, 0, 0, 254, 194, 255, 15, 254, 194, 255, 254, 143, 224, 0, 2, 104, 159, 1, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 85, 170,
        ];
        let (_, result) = parse_mbr(&test).unwrap();
        assert_eq!(result.boot_type, BootType::MasterBootRecord);
        assert_eq!(result.partitions.len(), 4);

        assert_eq!(result.partitions[1].offset_start, 7535066112);
    }

    #[test]
    fn test_parse_extended_lvm() {
        let test = [
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
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 254, 194, 255, 142, 254, 194, 255, 2, 0, 0, 0, 0,
            40, 167, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 85, 170,
        ];
        let (_, (results, has_extended)) = parse_extended(&test, 832568320, 0).unwrap();
        assert!(!has_extended);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].partition_type, PartitionType::LinuxLvm);
        assert_eq!(results[0].offset_start, 1024);
        assert_eq!(results[1].offset_start, 832568320);
        assert_eq!(results[1].partition_type, PartitionType::None);
    }
}
