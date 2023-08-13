use crate::utils::{
    nom_helper::{
        nom_signed_four_bytes, nom_unsigned_eight_bytes, nom_unsigned_four_bytes,
        nom_unsigned_two_bytes, Endian,
    },
    uuid::format_guid_le_bytes,
};
use nom::bytes::complete::take;

#[derive(Debug)]
pub(crate) struct OleHeader {
    sig: u64,
    class_id: String,
    minor_version: u16,
    major_version: u16,
    byte_order: OleEndian,
    /**Raised to power of two (2) */
    pub(crate) sector_size: u16,
    /**Raised to power of two (2) */
    pub(crate) short_sector_size: u16,
    reserved: u16,
    reserved2: u32,
    reserved3: u32,
    pub(crate) total_sectors_number: u32,
    /**Sector ID (SID) of directory stream (chain) */
    pub(crate) sector_id_chain: u32,
    reserved4: u32,
    /**Typically 4096 bytes. Smaller sizes stored in short-streams */
    pub(crate) min_stream_size: u32,
    /**Sector ID (SID) of short-sectory allocation table (SSAT) */
    pub(crate) sector_id_ssat: i32,
    pub(crate) total_ssat_sectors: u32,
    /**Sector ID (SID) of master sector allocation table (SSAT) */
    pub(crate) sector_id_msat: u32,
    pub(crate) total_msat_sectors: u32,
    /**First part of the MSAT. Contains 109 SIDs */
    pub(crate) msat_sectors: Vec<u32>,
}

#[derive(Debug)]
enum OleEndian {
    BigEndian,
    LittleEndian,
    Unknown,
}

impl OleHeader {
    /// Parse Header information from OLE data
    pub(crate) fn parse_header(data: &[u8]) -> nom::IResult<&[u8], OleHeader> {
        let (input, sig) = nom_unsigned_eight_bytes(data, Endian::Le)?;
        let guid_size: u8 = 16;
        let (input, class_id_data) = take(guid_size)(input)?;

        let (input, minor_version) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, major_version) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, byte_order) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, sector_size) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, short_sector_size) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, reserved) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, reserved2) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, reserved3) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let (input, total_sectors_number) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, sector_id_chain) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, reserved4) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, min_stream_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, sector_id_ssat) = nom_signed_four_bytes(input, Endian::Le)?;
        let (input, total_ssat_sectors) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, sector_id_msat) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, total_msat_sectors) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let little = 0xfeff;
        let big = 0xfffe;
        let order = if byte_order == little {
            OleEndian::LittleEndian
        } else if byte_order == big {
            OleEndian::BigEndian
        } else {
            OleEndian::Unknown
        };

        let msat_size: u16 = 436;
        let (input, mut msat) = take(msat_size)(input)?;

        let mut msat_sectors = Vec::new();

        let unused = 4294967295;
        while !msat.is_empty() {
            let (msat_data, value) = nom_unsigned_four_bytes(msat, Endian::Le)?;
            if value == unused {
                break;
            }

            msat_sectors.push(value);
            msat = msat_data;
        }

        let header = OleHeader {
            sig,
            class_id: format_guid_le_bytes(class_id_data),
            minor_version,
            major_version,
            byte_order: order,
            sector_size,
            short_sector_size,
            reserved,
            reserved2,
            total_sectors_number,
            sector_id_chain,
            reserved3,
            reserved4,
            min_stream_size,
            sector_id_ssat,
            total_ssat_sectors,
            sector_id_msat,
            total_msat_sectors,
            msat_sectors,
        };
        Ok((input, header))
    }
}

#[cfg(test)]
mod tests {
    use super::OleHeader;
    use crate::filesystem::files::read_file;
    use std::path::PathBuf;

    #[test]
    fn test_parser_header() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/ole/win11/header.raw");
        let data = read_file(&test_location.display().to_string()).unwrap();

        let (_, result) = OleHeader::parse_header(&data).unwrap();
        assert_eq!(result.sig, 16220472316735377360);
        assert_eq!(result.class_id, "00000000-0000-0000-0000-000000000000");
        assert_eq!(result.minor_version, 62);
        assert_eq!(result.major_version, 3);

        assert_eq!(result.sector_size, 9);
        assert_eq!(result.short_sector_size, 6);
        assert_eq!(result.sector_id_chain, 1);
        assert_eq!(result.min_stream_size, 4096);
        assert_eq!(result.total_ssat_sectors, 1);
        assert_eq!(result.sector_id_ssat, 2);
        assert_eq!(result.sector_id_msat, 4294967294); //0xfffffffe
    }
}
