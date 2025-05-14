use super::{header::EseHeader, tags::PageTag};
use crate::utils::nom_helper::{
    Endian, nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_two_bytes,
};
use nom::bytes::complete::take;

/**
 * Page header structure for ESE Windows 7+
 */
#[derive(Debug, PartialEq)]
pub(crate) struct PageHeader {
    checksum: u64,
    database_last_modified: String,
    previous_page_number: u32,
    pub(crate) next_page_number: u32,
    father_data_page: u32,
    available_page_size: u16,
    available_uncommitted_data_size: u16,
    first_available_data_offset: u16,
    first_available_page_tag: u16,
    pub(crate) page_flags: Vec<PageFlags>,
    extended_checksum: u64,
    extended_checksum2: u64,
    extended_checksum3: u64,
    page_number: u64,
    unknown: u64,
    pub(crate) page_tags: Vec<PageTag>,
}

#[derive(Debug, PartialEq)]
pub(crate) enum PageFlags {
    Root,
    Leaf,
    ParentBranch,
    Empty,
    SpaceTree,
    Index,
    LongValue,
    Unknown,
    Scrubbed,
    NewRecord,
}
impl PageHeader {
    /// Parse the page header. Supports Winodws 7+
    pub(crate) fn parse_header(data: &[u8]) -> nom::IResult<&[u8], PageHeader> {
        let (input, checksum) = nom_unsigned_eight_bytes(data, Endian::Le)?;
        let (input, database_last_modified) = EseHeader::get_database_time(input)?;
        let (input, previous_page_number) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, next_page_number) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, father_data_page) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let (input, available_page_size) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, available_uncommitted_data_size) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (remaining, first_available_data_offset) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, mut first_available_page_tag) = nom_unsigned_two_bytes(remaining, Endian::Le)?;

        // If the first_available_page_tag is larger than the page.
        // Then its actually one byte
        // Seen in Windows 11 24H2.  Perhaps the upper bytes are flags?
        // https://github.com/Velocidex/go-ese/issues/26
        // Could also be the value is now 12 bits (instead of 16 bits/2 bytes)?
        // https://github.com/fox-it/dissect.esedb/pull/46
        let tag_size = 4;
        if first_available_page_tag as usize > data.len()
            || (first_available_page_tag * tag_size) as usize > data.len()
        {
            first_available_page_tag &= 0xfff;
        }

        let (mut page_data, page_flags) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let page_16k = 16384;
        let page_32k = 32768;

        let mut header = PageHeader {
            checksum,
            database_last_modified,
            previous_page_number,
            next_page_number,
            father_data_page,
            available_page_size,
            available_uncommitted_data_size,
            first_available_data_offset,
            first_available_page_tag,
            page_flags: PageHeader::get_flags(&page_flags),
            extended_checksum: 0,
            extended_checksum2: 0,
            extended_checksum3: 0,
            page_number: 0,
            unknown: 0,
            page_tags: Vec::new(),
        };

        // Extra checksums for larger pages
        if data.len() == page_16k || data.len() == page_32k {
            let (input, extended_checksum) = nom_unsigned_eight_bytes(page_data, Endian::Le)?;
            let (input, extended_checksum2) = nom_unsigned_eight_bytes(input, Endian::Le)?;
            let (input, extended_checksum3) = nom_unsigned_eight_bytes(input, Endian::Le)?;
            let (input, page_number) = nom_unsigned_eight_bytes(input, Endian::Le)?;
            let (remaining_input, unknown) = nom_unsigned_eight_bytes(input, Endian::Le)?;
            page_data = remaining_input;
            header.extended_checksum = extended_checksum;
            header.extended_checksum2 = extended_checksum2;
            header.extended_checksum3 = extended_checksum3;
            header.page_number = page_number;
            header.unknown = unknown;
        }

        /* We now need the page tags
         * However, the tags exist at the end of the page data
         * We can get there by multiplying `first_available_page_tag` * tag_size (4 bytes) and use that to reach the starting offset of the tags
         * Ex: Page size is 4k and we have one (1) tag. 4k - tag_size = start of tags offset
         */
        let tag_data: usize = (first_available_page_tag * tag_size).into();

        // Tag data size is obtained from first_available_page_tag
        let start = data.len() - tag_data;

        let (tag_start, _) = take(start)(data)?;
        // We now have start of tag data
        let (_, page_tags) = PageTag::parse_tags(tag_start, &data.len())?;
        header.page_tags = page_tags;

        // For large page sizes the tag flags are actually part of the first 2 bytes at the tag offset
        if data.len() == page_16k || data.len() == page_32k {
            for tag in header.page_tags.iter_mut() {
                let (input, _) = take(tag.offset)(page_data)?;
                let (_, flags) = nom_unsigned_two_bytes(input, Endian::Le)?;

                tag.flags = PageTag::get_flags(&flags);
            }
        }

        Ok((page_data, header))
    }

    /// Get the page flags
    fn get_flags(page_flags: &u32) -> Vec<PageFlags> {
        let root = 0x1;
        let leaf = 0x2;
        let parent = 0x4;
        let empty = 0x8;
        let space = 0x20;
        let index = 0x40;
        let long_value = 0x80;
        let unknown = 0x400;
        let unknown2 = 0x800;
        let unknown3 = 0x8000;
        let unknown4 = 0x10000;
        let scrubbed = 0x4000;
        let record = 0x2000;

        let mut flags = Vec::new();

        if (page_flags & root) == root {
            flags.push(PageFlags::Root);
        }
        if (page_flags & leaf) == leaf {
            flags.push(PageFlags::Leaf);
        }
        if (page_flags & parent) == parent {
            flags.push(PageFlags::ParentBranch);
        }
        if (page_flags & empty) == empty {
            flags.push(PageFlags::Empty);
        }
        if (page_flags & space) == space {
            flags.push(PageFlags::SpaceTree);
        }
        if (page_flags & index) == index {
            flags.push(PageFlags::Index);
        }
        if (page_flags & long_value) == long_value {
            flags.push(PageFlags::LongValue);
        }
        if (page_flags & unknown) == unknown {
            flags.push(PageFlags::Unknown);
        }
        if (page_flags & unknown2) == unknown2 {
            flags.push(PageFlags::Unknown);
        }
        if (page_flags & unknown3) == unknown3 {
            flags.push(PageFlags::Unknown);
        }
        if (page_flags & unknown4) == unknown4 {
            flags.push(PageFlags::Unknown);
        }
        if (page_flags & scrubbed) == scrubbed {
            flags.push(PageFlags::Scrubbed);
        }
        if (page_flags & record) == record {
            flags.push(PageFlags::NewRecord);
        }
        flags
    }
}

#[cfg(test)]
mod tests {
    use super::PageHeader;
    use crate::{
        artifacts::os::windows::ese::{page::PageFlags, tags::PageTag},
        filesystem::files::read_file,
    };
    use std::path::PathBuf;

    #[test]
    fn test_parse_header() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/ese/win10/page.raw");
        let test = read_file(test_location.to_str().unwrap()).unwrap();

        let (_, results) = PageHeader::parse_header(&test).unwrap();
        assert_eq!(
            results,
            PageHeader {
                checksum: 9028031379431680016,
                database_last_modified: String::from("70:0:0"),
                previous_page_number: 0,
                next_page_number: 0,
                father_data_page: 1,
                available_page_size: 16284,
                available_uncommitted_data_size: 0,
                first_available_data_offset: 16,
                first_available_page_tag: 1,
                page_flags: vec![
                    PageFlags::Root,
                    PageFlags::Leaf,
                    PageFlags::Unknown,
                    PageFlags::Unknown,
                    PageFlags::NewRecord
                ],
                extended_checksum: 1,
                extended_checksum2: 1,
                extended_checksum3: 7740441600458769,
                page_number: 1,
                unknown: 0,
                page_tags: vec![PageTag {
                    offset: 0,
                    value_size: 16,
                    flags: vec![]
                }]
            }
        )
    }

    #[test]
    fn test_page_flags() {
        let test = 43011;

        let results = PageHeader::get_flags(&test);
        assert_eq!(
            results,
            vec![
                PageFlags::Root,
                PageFlags::Leaf,
                PageFlags::Unknown,
                PageFlags::Unknown,
                PageFlags::NewRecord
            ]
        );
    }

    #[test]
    fn test_page_win11_24h2() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/ese/win11/catalog_24h2.raw");
        let test = read_file(test_location.to_str().unwrap()).unwrap();

        let (_, results) = PageHeader::parse_header(&test).unwrap();

        assert_eq!(results.first_available_page_tag, 87);
        assert_eq!(results.page_tags.len(), 87);
    }
}
