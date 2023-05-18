use super::sections::section::Section;
use crate::utils::{
    nom_helper::{nom_unsigned_eight_bytes, nom_unsigned_four_bytes, Endian},
    strings::extract_utf8_string,
};
use nom::bytes::complete::take;
use serde::Serialize;
use std::mem::size_of;

#[derive(Debug, Serialize)]
pub(crate) struct Segment64 {
    pub(crate) name: String,
    pub(crate) vmaddr: u64,
    pub(crate) vmsize: u64,
    pub(crate) file_offset: u64,
    pub(crate) file_size: u64,
    pub(crate) max_prot: u32,
    pub(crate) init_prot: u32,
    pub(crate) nsects: u32,
    pub(crate) flags: u32,
    pub(crate) sections: Vec<Section>,
}

impl Segment64 {
    /// Parse the Segment64 command data
    pub(crate) fn parse_segment64(data: &[u8]) -> nom::IResult<&[u8], Segment64> {
        let (segment_data, name) = take(size_of::<u128>())(data)?;
        let (segment_data, vmaddr) = nom_unsigned_eight_bytes(segment_data, Endian::Le)?;
        let (segment_data, vmsize) = nom_unsigned_eight_bytes(segment_data, Endian::Le)?;
        let (segment_data, file_offset) = nom_unsigned_eight_bytes(segment_data, Endian::Le)?;
        let (segment_data, file_size) = nom_unsigned_eight_bytes(segment_data, Endian::Le)?;

        let (segment_data, max_prot) = nom_unsigned_four_bytes(segment_data, Endian::Le)?;
        let (segment_data, init_prot) = nom_unsigned_four_bytes(segment_data, Endian::Le)?;
        let (segment_data, nsects) = nom_unsigned_four_bytes(segment_data, Endian::Le)?;
        let (mut segment_data, flags) = nom_unsigned_four_bytes(segment_data, Endian::Le)?;

        let mut sections: Vec<Section> = Vec::new();
        let mut sections_count = 0;
        while sections_count < nsects {
            let (section_data, section) = Section::parse_section(segment_data)?;
            segment_data = section_data;
            sections.push(section);
            sections_count += 1;
        }

        let segment = Segment64 {
            name: extract_utf8_string(name),
            vmaddr,
            vmsize,
            file_offset,
            file_size,
            max_prot,
            init_prot,
            nsects,
            flags,
            sections,
        };

        Ok((segment_data, segment))
    }

    pub(crate) fn parse_segment32(data: &[u8]) -> nom::IResult<&[u8], Segment64> {
        let (segment_data, name) = take(size_of::<u128>())(data)?;
        let (segment_data, vmaddr) = nom_unsigned_four_bytes(segment_data, Endian::Le)?;
        let (segment_data, vmsize) = nom_unsigned_four_bytes(segment_data, Endian::Le)?;
        let (segment_data, file_offset) = nom_unsigned_four_bytes(segment_data, Endian::Le)?;
        let (segment_data, file_size) = nom_unsigned_four_bytes(segment_data, Endian::Le)?;

        let (segment_data, max_prot) = nom_unsigned_four_bytes(segment_data, Endian::Le)?;
        let (segment_data, init_prot) = nom_unsigned_four_bytes(segment_data, Endian::Le)?;
        let (segment_data, nsects) = nom_unsigned_four_bytes(segment_data, Endian::Le)?;
        let (mut segment_data, flags) = nom_unsigned_four_bytes(segment_data, Endian::Le)?;

        let mut sections: Vec<Section> = Vec::new();
        let mut sections_count = 0;
        while sections_count < nsects {
            let (section_data, section) = Section::parse_section(segment_data)?;
            segment_data = section_data;
            sections.push(section);
            sections_count += 1;
        }

        let segment = Segment64 {
            name: extract_utf8_string(name),
            vmaddr: vmaddr.into(),
            vmsize: vmsize.into(),
            file_offset: file_offset.into(),
            file_size: file_size.into(),
            max_prot,
            init_prot,
            nsects,
            flags,
            sections,
        };

        Ok((segment_data, segment))
    }
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::macos::macho::commands::segments::Segment64;

    #[test]
    fn test_parse_segment64() {
        let test_data = [
            95, 95, 80, 65, 71, 69, 90, 69, 82, 79, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let (_, segs) = Segment64::parse_segment64(&test_data).unwrap();
        assert_eq!(segs.name, "__PAGEZERO");
        assert_eq!(segs.vmaddr, 0);
        assert_eq!(segs.vmsize, 0x100000000);
        assert_eq!(segs.file_offset, 0);
        assert_eq!(segs.file_size, 0);
        assert_eq!(segs.max_prot, 0);
        assert_eq!(segs.init_prot, 0);
        assert_eq!(segs.nsects, 0);
        assert_eq!(segs.flags, 0);
    }

    #[test]
    fn test_get_segment32() {
        let test_data = [
            95, 95, 80, 65, 71, 69, 90, 69, 82, 79, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let (_, segs) = Segment64::parse_segment32(&test_data).unwrap();
        assert_eq!(segs.name, "__PAGEZERO");
        assert_eq!(segs.vmaddr, 0);
        assert_eq!(segs.vmsize, 0x1);
        assert_eq!(segs.file_offset, 0);
        assert_eq!(segs.file_size, 0);
        assert_eq!(segs.max_prot, 0);
        assert_eq!(segs.init_prot, 0);
        assert_eq!(segs.nsects, 0);
        assert_eq!(segs.flags, 0);
    }
}
