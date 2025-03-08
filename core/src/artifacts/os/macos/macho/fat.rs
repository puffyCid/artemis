use crate::utils::nom_helper::{Endian, nom_unsigned_four_bytes};

#[derive(Debug)]
pub(crate) struct FatHeader {
    _signature: u32,
    pub(crate) number_arch: u32,
    pub(crate) archs: Vec<FatArchHeader>,
}

#[derive(Debug)]
pub(crate) struct FatArchHeader {
    _cpu_type: u32,
    _cpu_subtype: u32,
    pub(crate) offset: u32,
    pub(crate) size: u32,
    _reserved: u32,
}

impl FatHeader {
    /// Parse the FAT header
    pub(crate) fn parse_header(data: &[u8]) -> nom::IResult<&[u8], FatHeader> {
        let (macho_data, _signature) = nom_unsigned_four_bytes(data, Endian::Be)?;
        let (mut macho_data, number_arch) = nom_unsigned_four_bytes(macho_data, Endian::Be)?;

        let mut header = FatHeader {
            _signature,
            number_arch,
            archs: Vec::new(),
        };

        let mut arch_count = 0;
        while arch_count < header.number_arch {
            let (header_data, arch_data) = FatHeader::parse_arch(macho_data)?;
            macho_data = header_data;
            header.archs.push(arch_data);
            arch_count += 1;
        }

        Ok((macho_data, header))
    }

    /// Check if file is a FAT macho binary
    pub(crate) fn is_fat(data: &[u8]) -> nom::IResult<&[u8], bool> {
        let (_, signature) = nom_unsigned_four_bytes(data, Endian::Be)?;

        let fat_sig = 0xcafebabe;
        if signature != fat_sig {
            return Ok((data, false));
        }
        Ok((data, true))
    }

    fn parse_arch(data: &[u8]) -> nom::IResult<&[u8], FatArchHeader> {
        let (macho_data, _cpu_type) = nom_unsigned_four_bytes(data, Endian::Be)?;
        let (macho_data, _cpu_subtype) = nom_unsigned_four_bytes(macho_data, Endian::Be)?;
        let (macho_data, offset) = nom_unsigned_four_bytes(macho_data, Endian::Be)?;
        let (macho_data, size) = nom_unsigned_four_bytes(macho_data, Endian::Be)?;
        let (macho_data, _reserved) = nom_unsigned_four_bytes(macho_data, Endian::Be)?;

        let fat_arch = FatArchHeader {
            _cpu_type,
            _cpu_subtype,
            offset,
            size,
            _reserved,
        };
        Ok((macho_data, fat_arch))
    }
}

#[cfg(test)]
mod tests {
    use super::FatHeader;

    #[test]
    fn test_parse_header() {
        let test_data = [
            202, 254, 186, 190, 0, 0, 0, 2, 1, 0, 0, 7, 0, 0, 0, 3, 0, 0, 64, 0, 0, 1, 28, 96, 0,
            0, 0, 14, 1, 0, 0, 12, 128, 0, 0, 2, 0, 1, 128, 0, 0, 1, 90, 160, 0, 0, 0, 14,
        ];

        let (_, results) = FatHeader::parse_header(&test_data).unwrap();

        assert_eq!(results._signature, 0xcafebabe);
        assert_eq!(results.number_arch, 2);

        assert_eq!(results.archs[0]._cpu_type, 0x1000007);
        assert_eq!(results.archs[0]._cpu_subtype, 0x3);
        assert_eq!(results.archs[0].offset, 0x4000);
        assert_eq!(results.archs[0].size, 0x11c60);
        assert_eq!(results.archs[0]._reserved, 0xe);

        assert_eq!(results.archs[1]._cpu_type, 0x100000C);
        assert_eq!(results.archs[1]._cpu_subtype, 0x80000002);
        assert_eq!(results.archs[1].offset, 0x18000);
        assert_eq!(results.archs[1].size, 0x15aa0);
        assert_eq!(results.archs[1]._reserved, 0xe);
    }

    #[test]
    fn test_parse_arch() {
        let test_data = [
            1, 0, 0, 12, 128, 0, 0, 2, 0, 1, 128, 0, 0, 1, 90, 160, 0, 0, 0, 14,
        ];

        let (_, results) = FatHeader::parse_arch(&test_data).unwrap();
        assert_eq!(results._cpu_type, 0x100000C);
        assert_eq!(results._cpu_subtype, 0x80000002);
        assert_eq!(results.offset, 0x18000);
        assert_eq!(results.size, 0x15aa0);
        assert_eq!(results._reserved, 0xe);
    }
}
