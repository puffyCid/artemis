use crate::{
    artifacts::os::windows::registry::error::RegistryError,
    filesystem::ntfs::reader::read_bytes,
    utils::nom_helper::{Endian, nom_unsigned_eight_bytes, nom_unsigned_four_bytes},
};
use log::error;
use ntfs::NtfsFile;
use serde::Serialize;
use std::io::BufReader;

#[derive(Debug, Serialize)]
pub(crate) struct HiveBin {
    signature: u32,
    offset: u32,
    pub(crate) size: u32,
    reserved: u64,
    timestamp: u64,
    spare: u32,
}

impl HiveBin {
    /// Parse the hive bin (hbin) header to determine size of hbin (should be multiple of 4096 bytes)
    pub(crate) fn parse_hive_bin_header(data: &[u8]) -> nom::IResult<&[u8], HiveBin> {
        let (input, signature) = nom_unsigned_four_bytes(data, Endian::Le)?;
        let (input, offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, size) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, reserved) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, timestamp) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, spare) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let hbin = HiveBin {
            signature,
            offset,
            size,
            reserved,
            timestamp,
            spare,
        };

        Ok((input, hbin))
    }

    pub(crate) fn read_hive_bin<T: std::io::Seek + std::io::Read>(
        reader: &mut BufReader<T>,
        ntfs_file: Option<&NtfsFile<'_>>,
    ) -> Result<HiveBin, RegistryError> {
        let bin_header_size = 32;
        let header_bytes = match read_bytes(4096, bin_header_size, ntfs_file, reader) {
            Ok(result) => result,
            Err(err) => {
                error!("[registry] Could not read hbin header bytes: {err:?}");
                return Err(RegistryError::ReadRegistry);
            }
        };

        let header = match HiveBin::parse_hive_bin_header(&header_bytes) {
            Ok((_, result)) => result,
            Err(_err) => {
                error!("[registry] Could not parse hbin header bytes");
                return Err(RegistryError::Parser);
            }
        };

        Ok(header)
    }
}

#[cfg(test)]
mod tests {
    use super::HiveBin;

    #[test]
    fn test_parse_hive_bin_header() {
        let test_data = [
            104, 98, 105, 110, 0, 0, 0, 0, 0, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0,
        ];

        let (_, result) = HiveBin::parse_hive_bin_header(&test_data).unwrap();

        assert_eq!(result.signature, 0x6e696268); // hbin
        assert_eq!(result.offset, 0);
        assert_eq!(result.size, 4096);
        assert_eq!(result.reserved, 0);
        assert_eq!(result.timestamp, 0);
        assert_eq!(result.spare, 0);
    }
}
