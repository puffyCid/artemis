use crate::utils::{
    nom_helper::{Endian, nom_unsigned_eight_bytes, nom_unsigned_four_bytes},
    strings::extract_utf16_string,
    time::filetime_to_unixepoch,
};
use log::error;
use nom::{bytes::complete::take, error::ErrorKind};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct RegHeader {
    signature: u32,
    pub(crate) primary_sequence_num: u32,
    pub(crate) secondary_sequence_num: u32,
    pub(crate) modified: i64,
    pub(crate) major_version: u32,
    pub(crate) minor_version: u32,
    file_type: u32,
    file_format: u32,
    pub(crate) root_offset: u32,
    pub(crate) hive_bins_size: u32, // Total size of all hbin cells
    pub(crate) cluster_factor: u32,
    filename: String,  // 64 bytes
    reserved: Vec<u8>, // 396 bytes, currently not parsing the small extra details of Windows 10 in reserved space
    pub(crate) checksum: u32,
    reserved2: Vec<u8>, // 3576 bytes
    boot_type: u32,
    boot_recover: u32,
    is_dirty: bool,
    valid_checksum: bool,
}

impl RegHeader {
    /// Parse the header structure of a Registry file
    pub(crate) fn parse_header(data: &[u8]) -> nom::IResult<&[u8], RegHeader> {
        let (input, signature) = nom_unsigned_four_bytes(data, Endian::Le)?;
        let sig = 0x66676572;

        if signature != sig {
            error!("[registry] Not a registry file, got signature: {signature}");
            return Err(nom::Err::Failure(nom::error::Error::new(
                input,
                ErrorKind::Fail,
            )));
        }
        let (input, primary_sequence_num) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, secondary_sequence_num) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, modified) = nom_unsigned_eight_bytes(input, Endian::Le)?;

        let (input, major_version) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, minor_version) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, file_type) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, file_format) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let (input, root_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, hive_bins_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, cluster_factor) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let filename_size: usize = 64;
        let (input, filename) = take(filename_size)(input)?;
        let reserved_size: usize = 396;
        let (input, reserved) = take(reserved_size)(input)?;

        let (input, checksum) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let reserved_size2: usize = 3576;
        let (input, reserved2) = take(reserved_size2)(input)?;
        let (input, boot_type) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, boot_recover) = nom_unsigned_four_bytes(input, Endian::Le)?;

        // If a Registry file is dirty parsing the associated .LOG files can provide additional data
        let is_dirty = primary_sequence_num != secondary_sequence_num;
        let checksum_source_size: usize = 508;
        let (_, checksum_source) = take(checksum_source_size)(data)?;
        let (_, verify_checksum) = RegHeader::verify_checksum(checksum_source)?;

        let valid_checksum = verify_checksum == checksum;

        let reg_header = RegHeader {
            signature,
            primary_sequence_num,
            secondary_sequence_num,
            modified: filetime_to_unixepoch(&modified),
            major_version,
            minor_version,
            file_type,
            file_format,
            root_offset,
            hive_bins_size,
            cluster_factor,
            filename: extract_utf16_string(filename),
            reserved: reserved.to_vec(),
            checksum,
            reserved2: reserved2.to_vec(),
            boot_type,
            boot_recover,
            is_dirty,
            valid_checksum,
        };

        Ok((input, reg_header))
    }

    /// Validate the Registry checksum value
    fn verify_checksum(data: &[u8]) -> nom::IResult<&[u8], u32> {
        let mut checksum = 0;
        let mut input = data;
        while !input.is_empty() {
            let (remaining_data, xor) = nom_unsigned_four_bytes(input, Endian::Le)?;

            checksum ^= xor;
            input = remaining_data;
        }
        Ok((input, checksum))
    }
}

#[cfg(test)]
mod tests {
    use super::RegHeader;
    use crate::filesystem::files::read_file;
    use std::path::PathBuf;

    #[test]
    fn test_parse_header() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/registry/win10/NTUSER.DAT");

        let buffer = read_file(&test_location.display().to_string()).unwrap();

        let (_, header) = RegHeader::parse_header(&buffer).unwrap();

        assert_eq!(header.signature, 0x66676572); // regf
        assert_eq!(header.primary_sequence_num, 20);
        assert_eq!(header.secondary_sequence_num, 20);
        assert_eq!(header.modified, -11644473600);
        assert_eq!(header.major_version, 1);
        assert_eq!(header.minor_version, 5);
        assert_eq!(header.file_type, 0);
        assert_eq!(header.file_format, 1);
        assert_eq!(header.root_offset, 32);

        assert_eq!(header.hive_bins_size, 196608);
        assert_eq!(header.filename, "\\??\\C:\\Users\\Default\\NTUSER.DAT");
        assert_eq!(header.reserved.len(), 396);
        assert_eq!(header.reserved2.len(), 3576);

        assert_eq!(header.boot_type, 0);
        assert_eq!(header.boot_recover, 0);
        assert_eq!(header.checksum, 89477030);
        assert_eq!(header.is_dirty, false);
        assert_eq!(header.valid_checksum, true);
    }

    #[test]
    fn test_get_all_hive_bins() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/registry/win10/NTUSER.DAT");

        let buffer = read_file(&test_location.display().to_string()).unwrap();

        let (_, header) = RegHeader::parse_header(&buffer).unwrap();

        assert_eq!(header.signature, 0x66676572); // regf
        assert_eq!(header.primary_sequence_num, 20);
        assert_eq!(header.secondary_sequence_num, 20);
        assert_eq!(header.modified, -11644473600);
        assert_eq!(header.major_version, 1);
        assert_eq!(header.minor_version, 5);
        assert_eq!(header.file_type, 0);
        assert_eq!(header.file_format, 1);
        assert_eq!(header.root_offset, 32);

        assert_eq!(header.hive_bins_size, 196608);
        assert_eq!(header.filename, "\\??\\C:\\Users\\Default\\NTUSER.DAT");
        assert_eq!(header.reserved.len(), 396);
        assert_eq!(header.reserved2.len(), 3576);

        assert_eq!(header.boot_type, 0);
        assert_eq!(header.boot_recover, 0);
        assert_eq!(header.checksum, 89477030);
        assert_eq!(header.is_dirty, false);
        assert_eq!(header.valid_checksum, true);
    }

    #[test]
    fn test_verify_checksum() {
        let test_data = [166, 79, 85, 5];
        let (_, result) = RegHeader::verify_checksum(&test_data).unwrap();
        assert_eq!(result, 89477030);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_parse_user_reg_headers() {
        use crate::filesystem::ntfs::{
            raw_files::{get_user_registry_files, raw_read_data},
            setup::setup_ntfs_parser,
        };

        let user_regs = get_user_registry_files(&'C').unwrap();

        let mut ntfs_parser = setup_ntfs_parser(&'C').unwrap();
        for reg in user_regs {
            let ntfs_file = reg
                .reg_reference
                .to_file(&ntfs_parser.ntfs, &mut ntfs_parser.fs)
                .unwrap();
            let ntfs_data = ntfs_file.data(&mut ntfs_parser.fs, "").unwrap().unwrap();
            let mut data_attr_value = ntfs_data
                .to_attribute()
                .unwrap()
                .value(&mut ntfs_parser.fs)
                .unwrap();

            let buffer = raw_read_data(&mut data_attr_value, &mut ntfs_parser.fs).unwrap();

            let (_, header) = RegHeader::parse_header(&buffer).unwrap();

            assert_eq!(header.signature, 0x66676572); // regf

            assert_eq!(header.major_version, 1);
            assert_eq!(header.file_type, 0);
            assert_eq!(header.file_format, 1);
            assert_eq!(header.root_offset, 32);

            assert_eq!(header.reserved.len(), 396);
            assert_eq!(header.reserved2.len(), 3576);

            assert_eq!(header.boot_type, 0);
            assert_eq!(header.boot_recover, 0);
            assert_eq!(header.valid_checksum, true);
        }
    }
}
