use super::{
    attribute::{EntryAttributes, grab_attributes},
    header::{AttributeHeader, AttributeType},
};
use crate::{
    accessor::io::reader::AccessorReader,
    artifacts::os::windows::mft::{fixup::Fixup, header::MftHeader},
    utils::{
        nom_helper::{
            Endian, nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_one_byte,
            nom_unsigned_two_bytes,
        },
        strings::extract_utf16_string,
    },
};
use nom::bytes::complete::take;
use serde::Serialize;
use tracing::error;

#[derive(Debug, Serialize)]
pub(crate) struct AttributeList {
    pub(crate) attribute_type: AttributeType,
    size: u16,
    name_size: u8,
    name_offset: u8,
    attribute_name: String,
    vcn: u64,
    parent_mft: u32,
    parent_sequence: u16,
    attribute_id: u16,
    pub(crate) attribute: EntryAttributes,
}

impl AttributeList {
    /// Start parsing the `AttributeList` attribute
    pub(crate) fn parse_list<'a>(
        data: &'a [u8],
        reader: &mut AccessorReader,
        entry_size: u32,
        current_mft: u32,
    ) -> nom::IResult<&'a [u8], Vec<AttributeList>> {
        let mut remaining = data;
        let min_size = 32;
        let mut lists = Vec::new();
        while remaining.len() >= min_size {
            let (input, attribute_type) = nom_unsigned_four_bytes(remaining, Endian::Le)?;
            let (input, size) = nom_unsigned_two_bytes(input, Endian::Le)?;
            let (input, name_size) = nom_unsigned_one_byte(input, Endian::Le)?;
            let (input, name_offset) = nom_unsigned_one_byte(input, Endian::Le)?;
            let (input, vcn) = nom_unsigned_eight_bytes(input, Endian::Le)?;
            let (input, parent_mft) = nom_unsigned_four_bytes(input, Endian::Le)?;
            let (input, _padding) = nom_unsigned_two_bytes(input, Endian::Le)?;
            let (input, parent_sequence) = nom_unsigned_two_bytes(input, Endian::Le)?;
            let (input, attribute_id) = nom_unsigned_two_bytes(input, Endian::Le)?;

            // adjust for UTF16. Double the name size
            let adjust = 2;
            if (name_size as u16 * adjust) as usize > input.len() {
                break;
            }
            let (input, name_data) = take(name_size as u16 * adjust)(input)?;
            let attribute_name = extract_utf16_string(name_data);

            let padding_size: u8 = 6;

            // Seen ADS attributes. Ex: Zone.Identifier
            if input.len() < padding_size as usize {
                break;
            }

            let (input, _padding) = take(padding_size)(input)?;
            remaining = input;

            let mut list = AttributeList {
                attribute_type: AttributeHeader::get_type(attribute_type),
                size,
                name_size,
                name_offset,
                attribute_name,
                vcn,
                parent_mft,
                parent_sequence,
                attribute_id,
                attribute: EntryAttributes::default(),
            };

            if list.parent_mft == current_mft {
                lists.push(list);
                continue;
            }

            let offset = list.parent_mft * entry_size;
            let list_mft = match reader.read_bytes(offset as u64, entry_size as usize) {
                Ok(result) => result,
                Err(err) => {
                    error!("Failed to read attribute list bytes: {err:?}");
                    lists.push(list);
                    continue;
                }
            };

            list.attribute = match AttributeList::grab_list_data(&list_mft, reader) {
                Ok((_, result)) => result,
                Err(_err) => {
                    error!("Failed to parse attribute list bytes");
                    lists.push(list);
                    continue;
                }
            };

            lists.push(list);
        }

        Ok((remaining, lists))
    }

    /// Parse each list entry
    fn grab_list_data<'a>(
        data: &'a [u8],
        reader: &mut AccessorReader,
    ) -> nom::IResult<&'a [u8], EntryAttributes> {
        let (remaining, header) = MftHeader::parse_header(data)?;
        let (remaining, fixup) = Fixup::get_fixup(remaining, header.fix_up_count)?;

        let mut mft_bytes = remaining.to_vec();
        Fixup::apply_fixup(&mut mft_bytes, &fixup);

        let (remaining, attribute) =
            grab_attributes(remaining, reader, header.total_size, header.index)?;

        Ok((remaining, attribute))
    }
}

#[cfg(test)]
mod tests {
    use super::AttributeList;
    use crate::accessor::access::Accessor;
    use std::path::PathBuf;

    #[test]
    fn test_parse_list() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/dfir/windows/mft/win11/MFT");

        let test = [
            16, 0, 0, 0, 32, 0, 0, 26, 0, 0, 0, 0, 0, 0, 0, 0, 9, 0, 0, 0, 0, 0, 9, 0, 0, 0, 68,
            67, 0, 0, 0, 0, 48, 0, 0, 0, 32, 0, 0, 26, 0, 0, 0, 0, 0, 0, 0, 0, 9, 0, 0, 0, 0, 0, 9,
            0, 7, 0, 0, 0, 0, 0, 0, 0, 128, 0, 0, 0, 40, 0, 4, 26, 0, 0, 0, 0, 0, 0, 0, 0, 35, 3,
            0, 0, 0, 0, 1, 0, 0, 0, 36, 0, 83, 0, 68, 0, 83, 0, 0, 0, 0, 0, 0, 0, 144, 0, 0, 0, 40,
            0, 4, 26, 0, 0, 0, 0, 0, 0, 0, 0, 9, 0, 0, 0, 0, 0, 9, 0, 17, 0, 36, 0, 83, 0, 68, 0,
            72, 0, 0, 0, 0, 0, 0, 0, 144, 0, 0, 0, 40, 0, 4, 26, 0, 0, 0, 0, 0, 0, 0, 0, 9, 0, 0,
            0, 0, 0, 9, 0, 16, 0, 36, 0, 83, 0, 73, 0, 73, 0, 0, 0, 0, 0, 0, 0, 160, 0, 0, 0, 40,
            0, 4, 26, 0, 0, 0, 0, 0, 0, 0, 0, 204, 9, 0, 0, 0, 0, 1, 0, 1, 0, 36, 0, 83, 0, 68, 0,
            72, 0, 0, 0, 0, 0, 0, 0, 160, 0, 0, 0, 40, 0, 4, 26, 0, 0, 0, 0, 0, 0, 0, 0, 204, 9, 0,
            0, 0, 0, 1, 0, 2, 0, 36, 0, 83, 0, 73, 0, 73, 0, 0, 0, 0, 0, 0, 0, 176, 0, 0, 0, 40, 0,
            4, 26, 0, 0, 0, 0, 0, 0, 0, 0, 204, 9, 0, 0, 0, 0, 1, 0, 3, 0, 36, 0, 83, 0, 68, 0, 72,
            0, 0, 0, 0, 0, 0, 0, 176, 0, 0, 0, 40, 0, 4, 26, 0, 0, 0, 0, 0, 0, 0, 0, 204, 9, 0, 0,
            0, 0, 1, 0, 4, 0, 36, 0, 83, 0, 73, 0, 73, 0, 0, 0, 0, 0, 0, 0,
        ];
        let mut reader = Accessor::with_defaults()
            .open_reader(test_location.to_str().unwrap())
            .unwrap();

        let (_, results) = AttributeList::parse_list(&test, &mut reader, 1024, 9).unwrap();
        assert_eq!(results.len(), 9);
    }

    #[test]
    #[should_panic(expected = "Eof")]
    fn test_grab_list_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/dfir/windows/mft/win11/MFT");

        let test = [
            16, 0, 0, 0, 32, 0, 0, 26, 0, 0, 0, 0, 0, 0, 0, 0, 9, 0, 0, 0, 0, 0, 9, 0, 0, 0, 68,
            67, 0, 0, 0, 0, 48, 0, 0, 0, 32, 0, 0, 26, 0, 0, 0, 0, 0, 0, 0, 0, 9, 0, 0, 0, 0, 0, 9,
            0, 7, 0, 0, 0, 0, 0, 0, 0, 128, 0, 0, 0, 40, 0, 4, 26, 0, 0, 0, 0, 0, 0, 0, 0, 35, 3,
            0, 0, 0, 0, 1, 0, 0, 0, 36, 0, 83, 0, 68, 0, 83, 0, 0, 0, 0, 0, 0, 0, 144, 0, 0, 0, 40,
            0, 4, 26, 0, 0, 0, 0, 0, 0, 0, 0, 9, 0, 0, 0, 0, 0, 9, 0, 17, 0, 36, 0, 83, 0, 68, 0,
            72, 0, 0, 0, 0, 0, 0, 0, 144, 0, 0, 0, 40, 0, 4, 26, 0, 0, 0, 0, 0, 0, 0, 0, 9, 0, 0,
            0, 0, 0, 9, 0, 16, 0, 36, 0, 83, 0, 73, 0, 73, 0, 0, 0, 0, 0, 0, 0, 160, 0, 0, 0, 40,
            0, 4, 26, 0, 0, 0, 0, 0, 0, 0, 0, 204, 9, 0, 0, 0, 0, 1, 0, 1, 0, 36, 0, 83, 0, 68, 0,
            72, 0, 0, 0, 0, 0, 0, 0, 160, 0, 0, 0, 40, 0, 4, 26, 0, 0, 0, 0, 0, 0, 0, 0, 204, 9, 0,
            0, 0, 0, 1, 0, 2, 0, 36, 0, 83, 0, 73, 0, 73, 0, 0, 0, 0, 0, 0, 0, 176, 0, 0, 0, 40, 0,
            4, 26, 0, 0, 0, 0, 0, 0, 0, 0, 204, 9, 0, 0, 0, 0, 1, 0, 3, 0, 36, 0, 83, 0, 68, 0, 72,
            0, 0, 0, 0, 0, 0, 0, 176, 0, 0, 0, 40, 0, 4, 26, 0, 0, 0, 0, 0, 0, 0, 0, 204, 9, 0, 0,
            0, 0, 1, 0, 4, 0, 36, 0, 83, 0, 73, 0, 73, 0, 0, 0, 0, 0, 0, 0,
        ];

        let mut reader = Accessor::with_defaults()
            .open_reader(test_location.to_str().unwrap())
            .unwrap();

        let _ = AttributeList::grab_list_data(&test, &mut reader).unwrap();
    }
}
