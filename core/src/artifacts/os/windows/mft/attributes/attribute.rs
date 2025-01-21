use super::{
    data::parse_data_run,
    filename::Filename,
    header::{AttributeHeader, AttributeType, ResidentFlag},
    nonresident::NonResident,
    resident::Resident,
    standard::Standard,
};
use crate::{
    artifacts::os::windows::{
        mft::attributes::{
            extended::ExtendedInfo, index::IndexRoot, list::AttributeList, object::ObjectId,
            reparse::ReparsePoint, stream::LoggedStream, volume::VolumeInfo,
        },
        securitydescriptor::descriptor::Descriptor,
    },
    utils::{encoding::base64_encode_standard, strings::extract_utf16_string},
};
use nom::bytes::complete::take;
use ntfs::NtfsFile;
use serde::Serialize;
use serde_json::{json, Value};
use std::io::BufReader;

#[derive(Debug, Serialize)]
pub(crate) struct EntryAttributes {
    pub(crate) standard: Vec<Standard>,
    pub(crate) filename: Vec<Filename>,
    pub(crate) attributes: Vec<Value>,
}

pub(crate) fn grab_attributes<'a, T: std::io::Seek + std::io::Read>(
    data: &'a [u8],
    reader: &mut BufReader<T>,
    ntfs_file: Option<&NtfsFile<'a>>,
    size: &u32,
    current_mft: &u32,
) -> nom::IResult<&'a [u8], EntryAttributes> {
    let mut entry_data = data;
    let header_size = 16;

    let mut entry_attributes = EntryAttributes {
        standard: Vec::new(),
        filename: Vec::new(),
        attributes: Vec::new(),
    };
    while entry_data.len() > header_size {
        let (input, mut header) = AttributeHeader::parse_header(entry_data)?;
        println!("{header:?}");

        // We are done if we have Unknown attribute or End attribute
        if header.attrib_type == AttributeType::Unknown
            || header.attrib_type == AttributeType::End
            || header.size == 0
        {
            break;
        }

        let mut attribute_size = header.size - header_size as u32;
        if attribute_size as usize > input.len() {
            attribute_size = header.small_size as u32 - header_size as u32;
        }
        let (remaining, input) = take(attribute_size)(input)?;
        entry_data = remaining;

        let mut input = if header.resident_flag == ResidentFlag::Resident {
            let (input, resident) = Resident::parse_resident(input)?;
            input
        } else {
            let (nonres_input, nonresident) = NonResident::parse_nonresident(input)?;
            println!("{nonresident:?} - input len: {}", input.len());
            if nonresident.data_runs_offset as usize > input.len() {
                entry_attributes
                    .attributes
                    .push(json!({format!("{:?} - non-resident", header.attrib_type): nonresident}));
                continue;
            }

            // if header.attrib_type == AttributeType::Data {
            // Go to data runs offset
            let (input, _) = take(nonresident.data_runs_offset)(input)?;
            input
            // } else {
            //    nonres_input
            //}
        };

        // Attributes may have a name, but the data could also be non-resident
        if ((header.name_size * 2) as usize) < input.len()
            && header.resident_flag == ResidentFlag::Resident
        {
            // Name is UTF16
            let (remaining, name_data) = take(header.name_size * 2)(input)?;
            if !name_data.is_empty() {
                header.name = extract_utf16_string(name_data);
            }
            input = remaining;
        }

        // Only support Standard and Filename attributes for now
        if header.attrib_type == AttributeType::StandardInformation {
            let (_, standard) = Standard::parse_standard_info(input)?;
            entry_attributes.standard.push(standard);
        } else if header.attrib_type == AttributeType::FileName {
            let (_, filename) = Filename::parse_filename(input)?;
            entry_attributes.filename.push(filename);
        } else if header.resident_flag == ResidentFlag::NonResident {
            let (_, runs) = parse_data_run(input)?;
            entry_attributes
                .attributes
                .push(serde_json::to_value(runs).unwrap_or_default());
        } else if header.attrib_type == AttributeType::Bitmap {
            let bitmap_data = base64_encode_standard(input);
            entry_attributes
                .attributes
                .push(json!({"bitmap":bitmap_data}));
        } else if header.attrib_type == AttributeType::ObjectId {
            let (_, object) = ObjectId::parse_object_id(input)?;
            println!("{object:?}");
            entry_attributes
                .attributes
                .push(serde_json::to_value(object).unwrap_or_default());
        } else if header.attrib_type == AttributeType::VolumeName {
            let name = extract_utf16_string(input);
            entry_attributes
                .attributes
                .push(json!({"volume_name":name}));
        } else if header.attrib_type == AttributeType::VolumeInformation {
            let (_, info) = VolumeInfo::parse_volume_info(input)?;
            println!("{info:?}");
            entry_attributes
                .attributes
                .push(serde_json::to_value(info).unwrap_or_default());
        } else if header.attrib_type == AttributeType::Data {
            let attrib_data = if input.is_empty() {
                String::new()
            } else {
                base64_encode_standard(input)
            };
            entry_attributes
                .attributes
                .push(json!({"data":attrib_data}));
            println!("{attrib_data}");
        } else if header.attrib_type == AttributeType::IndexRoot {
            let (_, index) = IndexRoot::parse_root(input)?;
            println!("{index:?}");
            entry_attributes.attributes.push(index);
        } else if header.attrib_type == AttributeType::LoggedStream {
            if header.name == "$TXF_DATA" {
                let (_, stream) = LoggedStream::parse_transactional_stream(input)?;
                println!("{stream:?}");
                entry_attributes
                    .attributes
                    .push(serde_json::to_value(stream).unwrap_or_default());
            }
        } else if header.attrib_type == AttributeType::SecurityDescriptor {
            let (_, sid) = Descriptor::parse_descriptor(input)?;
            println!("{sid:?}");
            entry_attributes
                .attributes
                .push(serde_json::to_value(sid).unwrap_or_default());
        } else if header.attrib_type == AttributeType::AttributeList {
            let (_, mut list) =
                AttributeList::parse_list(input, reader, ntfs_file, size, current_mft)?;
            println!("{list:?}");

            check_list(&mut list, &mut entry_attributes);
            entry_attributes
                .attributes
                .push(serde_json::to_value(list).unwrap_or_default());
        } else if header.attrib_type == AttributeType::ReparsePoint {
            let (_, point) = ReparsePoint::parse_reparse(input)?;
            println!("reparse point: {point:?}");

            entry_attributes
                .attributes
                .push(serde_json::to_value(point).unwrap_or_default());
        } else if header.attrib_type == AttributeType::ExtendedInfo {
            let (_, info) = ExtendedInfo::parse_extended_info(input)?;
            println!("extended info: {info:?}");

            entry_attributes
                .attributes
                .push(serde_json::to_value(info).unwrap_or_default());
        } else if header.attrib_type == AttributeType::Extended {
            let (_, info) = ExtendedInfo::parse_extended_attribute(input)?;
            println!("extended attrib: {info:?}");

            entry_attributes
                .attributes
                .push(serde_json::to_value(info).unwrap_or_default());
        } else {
            panic!("{header:?}");
        }
    }

    Ok((entry_data, entry_attributes))
}

// Check if Attribute List contains any FILENAME or STANDARD attributes. Sometimes they are stored here
fn check_list(attribs: &mut [AttributeList], entries: &mut EntryAttributes) {
    for entry in attribs {
        if entry.attribute_type == AttributeType::FileName {
            entries.filename.append(&mut entry.attribute.filename);
        }
        if entry.attribute_type == AttributeType::StandardInformation {
            entries.standard.append(&mut entry.attribute.standard);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::grab_attributes;
    use crate::artifacts::os::windows::mft::reader::setup_mft_reader;
    use std::io::BufReader;

    #[test]
    fn test_grab_attribtes() {
        let test = [
            16, 0, 0, 0, 96, 0, 0, 0, 0, 0, 24, 0, 0, 0, 0, 0, 72, 0, 0, 0, 24, 0, 0, 0, 172, 119,
            65, 126, 194, 223, 218, 1, 172, 119, 65, 126, 194, 223, 218, 1, 172, 119, 65, 126, 194,
            223, 218, 1, 172, 119, 65, 126, 194, 223, 218, 1, 6, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let reader = setup_mft_reader("").unwrap();
        let mut buf_reader = BufReader::new(reader);

        let (_, result) = grab_attributes(&test, &mut buf_reader, None, &0, &0).unwrap();
        assert_eq!(result.standard[0].created, 133665165395720108);
        assert_eq!(result.standard[0].modified, 133665165395720108);
        assert_eq!(result.standard[0].accessed, 133665165395720108);
        assert_eq!(result.standard[0].changed, 133665165395720108);
        assert_eq!(result.standard[0].sid_id, 257);
    }
}
