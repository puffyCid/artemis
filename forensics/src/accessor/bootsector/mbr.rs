use crate::{
    accessor::error::{AccessorError, AccessorResult},
    utils::nom_helper::{nom_take, nom_u8, nom_u16, nom_u32},
};

/// Parsed Master Boot Record partition table
#[derive(PartialEq, Debug, Clone)]
pub(crate) struct Mbr {
    /// Unique Disk for the MBR
    pub(crate) disk_id: u32,
    /// Array of MBR entries identified
    pub(crate) entries: Vec<MbrEntry>,
}

/// Single MBR entry record
#[derive(PartialEq, Debug, Clone)]
pub(crate) struct MbrEntry {
    /// The MBR entry slot number
    pub(crate) slot: u8,
    /// Raw status byte
    pub(crate) status: u8,
    /// `PartitionType` value
    pub(crate) partition_type: PartitionType,
    /// Raw `PartitionType` numeric value
    pub(crate) partition_type_raw: u8,
    /// Starting logical block address
    pub(crate) start_lba: u32,
    /// Number of sectors in the partition
    pub(crate) sector_count: u32,
}

#[derive(PartialEq, Debug, Clone)]
pub(crate) enum PartitionType {
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
    Efi,
    None,
}

impl MbrEntry {
    /// Return whether the `MbrEntry` bootable
    pub(crate) fn is_bootable(&self) -> bool {
        self.status == 0x80
    }

    /// Return whether the `MbrEntry` points to extended boot record
    pub(crate) fn is_extended(&self) -> bool {
        matches!(self.partition_type_raw, 0x5 | 0xf)
    }

    /// Return whether the `MbrEntry` is protective GPT partition
    pub(crate) fn is_protective_gpt(&self) -> bool {
        self.partition_type_raw == 0xee
    }

    /// Calculate the byte offset of the partition for logical sector size
    pub(crate) fn byte_offset(&self, sector_size: u64) -> u64 {
        self.start_lba as u64 * sector_size
    }

    /// Calculate the byte length of the partition for a logical sector size
    pub(crate) fn byte_length(&self, sector_size: u64) -> u64 {
        self.sector_count as u64 * sector_size
    }
}

impl Mbr {
    /// Return whether the Master Boot Record contains a protective GPT partition
    pub(crate) fn is_protective_gpt(&self) -> bool {
        self.entries.iter().any(MbrEntry::is_protective_gpt)
    }
}

/// Parse the Master Boot Record bytes
pub(crate) fn parse_mbr(sector: &[u8]) -> AccessorResult<Mbr> {
    let (disk_id, entries) = parse_partition_table(sector)?;
    Ok(Mbr { disk_id, entries })
}

/// Parse the extended partitions: <https://en.wikipedia.org/wiki/Extended_boot_record>
pub(crate) fn parse_ebr(sector: &[u8]) -> AccessorResult<Vec<MbrEntry>> {
    let (_disk_id, entries) = parse_partition_table(sector)?;

    Ok(entries)
}

/// Parse the Master Boot Record table
fn parse_partition_table(sector: &[u8]) -> AccessorResult<(u32, Vec<MbrEntry>)> {
    let boot_code: u16 = 440;

    let (input, _boot) = nom_take(sector, boot_code, "MBR boot code is truncated")?;
    let (input, disk_id) = nom_u32(input, "MBR disk ID is truncated")?;
    let (mut input, _reserved) = nom_u16(input, "MBR reserved field is truncated")?;

    let mut entries = Vec::new();
    let mbr_entry_len: u8 = 16;
    for slot in 0..4 {
        let (remaining, entry) = nom_take(
            input,
            mbr_entry_len,
            &format!("MBR partition entry {slot} is truncated"),
        )?;
        input = remaining;

        let value = parse_mbr_entry(slot, entry)?;
        if value.partition_type_raw != 0 && value.sector_count != 0 {
            entries.push(value);
        }
    }

    let (_, sig) = nom_u16(input, "MBR signature is truncated")?;
    if sig != 0xaa55 {
        return Err(AccessorError::volume(format!(
            "Invalid MBR sig {sig:#06x}. Wanted 0xaa55"
        )));
    }

    Ok((disk_id, entries))
}

/// Parse each MBR entry
fn parse_mbr_entry(slot: u8, data: &[u8]) -> AccessorResult<MbrEntry> {
    let (input, status) = nom_u8(data, "MBR partition status is truncated")?;
    let (input, _start_chs) = nom_take(input, 3_u8, "MBR partition start CHS is truncated")?;
    let (input, partition_type_raw) = nom_u8(input, "MBR partition type is truncated")?;
    let (input, _end_chs) = nom_take(input, 3_u8, "MBR partition end CHS is truncated")?;
    let (input, start_lba) = nom_u32(input, "MBR partition start LBA is truncated")?;
    let (_, sector_count) = nom_u32(input, "MBR partition sector count is truncated")?;

    Ok(MbrEntry {
        slot,
        status,
        partition_type: get_partition_type(partition_type_raw),
        partition_type_raw,
        start_lba,
        sector_count,
    })
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
        0xee => PartitionType::Protective,
        0xef => PartitionType::Efi,
        0x5 | 0xf => PartitionType::Extended,
        _ => PartitionType::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use crate::accessor::{
        bootsector::mbr::{
            PartitionType, get_partition_type, parse_ebr, parse_mbr, parse_mbr_entry,
        },
        error::AccessorError,
    };
    use std::{fs::read, path::PathBuf};

    #[test]
    fn test_parse_ebr() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/bootsector/mbr/extended_partition.raw");
        let bytes = read(test_location.to_str().unwrap()).unwrap();
        let results = parse_ebr(&bytes).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].partition_type, PartitionType::Linux);
        assert_eq!(results[0].sector_count, 19529728);
        assert_eq!(results[1].start_lba, 19531468);
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
        let result = parse_mbr(&test).unwrap();
        assert_eq!(result.disk_id, 4033035010);
        assert_eq!(result.entries.len(), 2);

        assert_eq!(result.entries[1].byte_offset(512), 7535066112);
        assert_eq!(result.entries[0].slot, 0);
        assert_eq!(result.entries[0].partition_type_raw, 0x83);
        assert_eq!(result.entries[0].partition_type, PartitionType::Linux);

        assert!(result.entries[1].is_extended());
        assert!(!result.is_protective_gpt());
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
        let results = parse_ebr(&test).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].partition_type, PartitionType::LinuxLvm);
        assert_eq!(results[0].byte_offset(512), 1024);
        assert_eq!(results[0].start_lba, 2);
    }

    #[test]
    fn test_parse_mbr_bad_signature() {
        let sector = [0u8; 512];
        let error = parse_mbr(&sector).unwrap_err();
        assert!(matches!(
            error,
            AccessorError::Volume { reason } if reason.contains("Invalid MBR sig")
        ));
    }

    #[test]
    fn test_parse_mbr_entry() {
        let test = [128, 4, 1, 4, 131, 254, 194, 255, 0, 8, 0, 0, 0, 128, 224, 0];

        let results = parse_mbr_entry(1, &test).unwrap();

        assert_eq!(results.partition_type_raw, 0x83);
        assert_eq!(results.start_lba, 2048);
        assert_eq!(results.sector_count, 14712832);

        assert_eq!(results.byte_length(512), 7532969984);
        assert!(results.is_bootable());
        assert!(!results.is_protective_gpt());
    }
}
