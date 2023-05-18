use crate::utils::{
    nom_helper::{nom_unsigned_eight_bytes, nom_unsigned_four_bytes, Endian},
    strings::extract_utf8_string,
};
use nom::bytes::complete::take;
use serde::Serialize;
use std::mem::size_of;

#[derive(Debug, Serialize)]
pub(crate) struct Section {
    pub(crate) section_name: String,
    pub(crate) segment_name: String,
    pub(crate) addr: u64,
    pub(crate) size: u64,
    pub(crate) offset: u32,
    pub(crate) align: u32,
    pub(crate) relocation_offset: u32,
    pub(crate) number_relocation_entries: u32,
    pub(crate) flags: u32,
    reserved: u32,
    reserved2: u32,
    reserved3: u32,
}

impl Section {
    /// Get Sections associated from Segments
    pub(crate) fn parse_section(data: &[u8]) -> nom::IResult<&[u8], Section> {
        let (section_data, sect_name) = take(size_of::<u128>())(data)?;
        let (section_data, seg_name) = take(size_of::<u128>())(section_data)?;

        let (section_data, addr) = nom_unsigned_eight_bytes(section_data, Endian::Le)?;
        let (section_data, size) = nom_unsigned_eight_bytes(section_data, Endian::Le)?;
        let (section_data, offset) = nom_unsigned_four_bytes(section_data, Endian::Le)?;
        let (section_data, align) = nom_unsigned_four_bytes(section_data, Endian::Le)?;

        let (section_data, relocation_offset) = nom_unsigned_four_bytes(section_data, Endian::Le)?;
        let (section_data, number_relocation_entries) =
            nom_unsigned_four_bytes(section_data, Endian::Le)?;
        let (section_data, flags) = nom_unsigned_four_bytes(section_data, Endian::Le)?;
        let (section_data, reserved) = nom_unsigned_four_bytes(section_data, Endian::Le)?;
        let (section_data, reserved2) = nom_unsigned_four_bytes(section_data, Endian::Le)?;
        let (section_data, reserved3) = nom_unsigned_four_bytes(section_data, Endian::Le)?;

        let section = Section {
            section_name: extract_utf8_string(sect_name),
            segment_name: extract_utf8_string(seg_name),
            addr,
            size,
            offset,
            align,
            relocation_offset,
            number_relocation_entries,
            flags,
            reserved,
            reserved2,
            reserved3,
        };

        Ok((section_data, section))
    }
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::macos::macho::commands::sections::section::Section;

    #[test]
    fn test_parse_section64() {
        let test_data = [
            95, 95, 116, 101, 120, 116, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 95, 95, 84, 69, 88, 84, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 172, 57, 0, 0, 1, 0, 0, 0, 99, 58, 0, 0, 0, 0, 0, 0, 172, 57,
            0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0,
        ];
        let (_, sect) = Section::parse_section(&test_data).unwrap();
        assert_eq!(sect.section_name, "__text");
        assert_eq!(sect.segment_name, "__TEXT");
        assert_eq!(sect.addr, 0x1000039ac);
        assert_eq!(sect.size, 0x3a63);
        assert_eq!(sect.offset, 0x39ac);
        assert_eq!(sect.align, 2);
        assert_eq!(sect.relocation_offset, 0);
        assert_eq!(sect.number_relocation_entries, 0);
        assert_eq!(sect.flags, 0x80000400);
        assert_eq!(sect.reserved, 0);
        assert_eq!(sect.reserved2, 0);
        assert_eq!(sect.reserved3, 0);
    }
}
