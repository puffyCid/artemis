use crate::utils::nom_helper::{Endian, nom_unsigned_two_bytes};

#[derive(Debug, PartialEq)]
pub(crate) struct PageTag {
    pub(crate) offset: u16,
    pub(crate) value_size: u16,
    pub(crate) flags: Vec<TagFlags>,
}

#[derive(Debug, PartialEq)]
pub(crate) enum TagFlags {
    Value,
    Defunct,
    CommonKey,
}

impl PageTag {
    /// Get the tags for the page
    pub(crate) fn parse_tags<'a>(
        data: &'a [u8],
        size: &usize,
    ) -> nom::IResult<&'a [u8], Vec<PageTag>> {
        let mut tags: Vec<PageTag> = Vec::new();
        let mut tag_data = data;

        let page_16k = 16384;
        let page_32k = 32768;
        // Loop through all data until empty
        while !tag_data.is_empty() {
            let (input, value_size) = nom_unsigned_two_bytes(tag_data, Endian::Le)?;
            let (input, offset) = nom_unsigned_two_bytes(input, Endian::Le)?;

            let bit_adjust = if size == &page_16k || size == &page_32k {
                32767
            } else {
                8191
            };
            let tag = PageTag {
                offset: offset & bit_adjust,
                value_size: value_size & bit_adjust,
                flags: PageTag::get_flags(offset),
            };
            tags.push(tag);
            tag_data = input;
        }

        /* Tags are parsed from last to first
         * Ex: If there are four (4) tags (starting at zero (0)).
         * We parse the third (3rd) first, then second (2nd) second, until we reach the zero (0) our fourth (4th) parsed tag
         * But the zero (0) tag is actually first, so we need to reverse our array
         */
        tags.reverse();
        Ok((tag_data, tags))
    }

    /// Get the flags associated with the page tags
    pub(crate) fn get_flags(offset: u16) -> Vec<TagFlags> {
        let flag_check = 13;
        let flag = offset >> flag_check;

        match flag {
            1 => vec![TagFlags::Value],
            2 => vec![TagFlags::Defunct],
            3 => vec![TagFlags::Value, TagFlags::Defunct],
            4 => vec![TagFlags::CommonKey],
            5 => vec![TagFlags::Value, TagFlags::CommonKey],
            6 => vec![TagFlags::Defunct, TagFlags::CommonKey],
            7 => vec![TagFlags::Value, TagFlags::Defunct, TagFlags::CommonKey],
            _ => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PageTag;
    use crate::artifacts::os::windows::ese::tags::TagFlags;

    #[test]
    fn test_parse_tags() {
        let test = [16, 0, 0, 0];
        let (_, result) = PageTag::parse_tags(&test, &16384).unwrap();
        assert_eq!(result[0].offset, 0);
        assert_eq!(result[0].value_size, 16);
    }

    #[test]
    fn test_parse_multiple_tags() {
        let test = [10, 0, 36, 0, 10, 0, 26, 0, 10, 0, 16, 0, 16, 0, 0, 0];
        let (_, result) = PageTag::parse_tags(&test, &16384).unwrap();
        assert_eq!(result[0].offset, 0);
        assert_eq!(result[0].value_size, 16);

        assert_eq!(result[3].offset, 36);
        assert_eq!(result[3].value_size, 10);
    }

    #[test]
    fn test_parse_tags_small_page() {
        let test = [55, 0, 13, 160];
        let (_, result) = PageTag::parse_tags(&test, &4096).unwrap();
        assert_eq!(result[0].offset, 13);
        assert_eq!(result[0].value_size, 55);
    }

    #[test]
    fn test_parse_tags_srum() {
        let test = [
            39, 1, 186, 172, 39, 1, 147, 171, 39, 1, 108, 170, 39, 1, 69, 169, 39, 1, 30, 168, 39,
            1, 247, 166, 39, 1, 208, 165, 39, 1, 169, 164, 39, 1, 130, 163, 39, 1, 91, 162, 39, 1,
            52, 161, 38, 1, 14, 160, 14, 0, 0, 0,
        ];
        let (_, results) = PageTag::parse_tags(&test, &7).unwrap();
        for tag in results {
            if tag.offset == 0 {
                continue;
            }
            assert_eq!(tag.flags, vec![TagFlags::Value, TagFlags::CommonKey]);
        }
    }
}
