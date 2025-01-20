use super::{
    attribute::{grab_attributes, EntryAttributes},
    header::{AttributeHeader, AttributeType},
};
use crate::{
    artifacts::os::windows::mft::{fixup::Fixup, header::MftHeader},
    filesystem::ntfs::reader::read_bytes,
    utils::{
        nom_helper::{
            nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_one_byte,
            nom_unsigned_two_bytes, Endian,
        },
        strings::extract_utf16_string,
    },
};
use log::error;
use nom::{bytes::complete::take, error::ErrorKind};
use ntfs::NtfsFile;
use serde::Serialize;
use std::io::BufReader;

#[derive(Debug, Serialize)]
pub(crate) struct AttributeList {
    attribute_type: AttributeType,
    size: u16,
    name_size: u8,
    name_offset: u8,
    attribute_name: String,
    vcn: u64,
    parent_mft: u32,
    parent_sequence: u16,
    attribute_id: u16,
    attribute: EntryAttributes,
}

impl AttributeList {
    pub(crate) fn parse_list<'a, T: std::io::Seek + std::io::Read>(
        data: &'a [u8],
        reader: &mut BufReader<T>,
        ntfs_file: Option<&NtfsFile<'a>>,
        entry_size: &u32,
        current_mft: &u32,
    ) -> nom::IResult<&'a [u8], Vec<AttributeList>> {
        let mut remaining = data;
        let min_size = 32;
        let mut lists = Vec::new();
        while remaining.len() >= min_size {
            println!("llist loop len: {}", remaining.len());
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
                attribute_type: AttributeHeader::get_type(&attribute_type),
                size,
                name_size,
                name_offset,
                attribute_name,
                vcn,
                parent_mft,
                parent_sequence,
                attribute_id,
                attribute: EntryAttributes {
                    filename: Vec::new(),
                    standard: Vec::new(),
                    attributes: Vec::new(),
                },
            };

            if list.parent_mft == *current_mft {
                lists.push(list);
                continue;
            }

            let offset = list.parent_mft * entry_size;
            let list_mft = match read_bytes(&(offset as u64), *entry_size as u64, ntfs_file, reader)
            {
                Ok(result) => result,
                Err(err) => {
                    error!("[mft] Failed to read attribute list bytes: {err:?}");
                    return Err(nom::Err::Failure(nom::error::Error::new(
                        &[],
                        ErrorKind::Fail,
                    )));
                }
            };

            list.attribute = match AttributeList::grab_list_data(&list_mft, reader, ntfs_file) {
                Ok((_, result)) => result,
                Err(_err) => {
                    panic!("[mft] Failed to parse attribute list bytes");
                    continue;
                }
            };
            lists.push(list);
        }

        Ok((remaining, lists))
    }

    fn grab_list_data<'a, T: std::io::Seek + std::io::Read>(
        data: &'a [u8],
        reader: &mut BufReader<T>,
        ntfs_file: Option<&NtfsFile<'a>>,
    ) -> nom::IResult<&'a [u8], EntryAttributes> {
        println!("listheader len: {}", data.len());
        let (remaining, header) = MftHeader::parse_header(&data)?;
        let (remaining, fixup) = Fixup::get_fixup(remaining, header.fix_up_count)?;

        let (remaining, attribute) = grab_attributes(
            remaining,
            reader,
            ntfs_file,
            &header.total_size,
            &header.index,
        )?;

        Ok((remaining, attribute))
    }
}
