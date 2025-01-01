use super::{
    data::parse_data,
    filename::Filename,
    header::{AttributeHeader, AttributeType, ResidentFlag},
    nonresident::NonResident,
    resident::Resident,
    standard::Standard,
};
use nom::bytes::complete::take;

#[derive(Debug)]
pub(crate) struct EntryAttributes {
    pub(crate) standard: Vec<Standard>,
    pub(crate) filename: Vec<Filename>,
}

#[derive(Debug, PartialEq)]
pub(crate) enum FileAttributes {
    ReadOnly,
    Hidden,
    System,
    Volume,
    Directory,
    Archive,
    Device,
    Normal,
    Temporary,
    Sparse,
    Reparse,
    Compressed,
    Offline,
    NotIndexed,
    Encrypted,
    Virtual,
    IndexView,
    Unknown,
}

#[derive(Debug, PartialEq)]
pub(crate) enum Namespace {
    Posix,
    Windows,
    Dos,
    WindowsDos,
    Unknown,
}

pub(crate) fn grab_attributes(data: &[u8]) -> nom::IResult<&[u8], EntryAttributes> {
    let mut entry_data = data;
    let header_size = 16;

    let mut entry_attributes = EntryAttributes {
        standard: Vec::new(),
        filename: Vec::new(),
    };
    while entry_data.len() > header_size {
        let (input, header) = AttributeHeader::parse_header(entry_data)?;
        if header.size == 0 {
            break;
        }

        // We are done if we have Unkonwn attribute or End attribute
        if header.attrib_type == AttributeType::Unknown || header.attrib_type == AttributeType::End
        {
            break;
        }

        let mut attribute_size = header.size - header_size as u32;
        if attribute_size as usize > input.len() {
            attribute_size = header.small_size as u32 - header_size as u32;
        }
        let (remaining, input) = take(attribute_size)(input)?;
        entry_data = remaining;

        let input = if header.resident_flag == ResidentFlag::Resident {
            let (input, resident) = Resident::parse_resident(input)?;
            input
        } else {
            let (input, nonresdient) = NonResident::parse_nonresident(input)?;
            input
        };

        // Only support Standard and Filename attributes for now
        if header.attrib_type == AttributeType::StandardInformation {
            let (_, standard) = Standard::parse_standard_info(input)?;
            entry_attributes.standard.push(standard);
        } else if header.attrib_type == AttributeType::FileName {
            let (_, filename) = Filename::parse_filename(input)?;
            entry_attributes.filename.push(filename);
        } else if header.attrib_type == AttributeType::Data {
            parse_data(input).unwrap();
        } else {
            panic!("{header:?}");
        }
    }

    Ok((entry_data, entry_attributes))
}

#[cfg(test)]
mod tests {
    use super::grab_attributes;

    #[test]
    fn test_grab_attribtes() {
        let test = [
            16, 0, 0, 0, 96, 0, 0, 0, 0, 0, 24, 0, 0, 0, 0, 0, 72, 0, 0, 0, 24, 0, 0, 0, 172, 119,
            65, 126, 194, 223, 218, 1, 172, 119, 65, 126, 194, 223, 218, 1, 172, 119, 65, 126, 194,
            223, 218, 1, 172, 119, 65, 126, 194, 223, 218, 1, 6, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let (_, result) = grab_attributes(&test).unwrap();
        assert_eq!(result.standard[0].created, 133665165395720108);
        assert_eq!(result.standard[0].modified, 133665165395720108);
        assert_eq!(result.standard[0].accessed, 133665165395720108);
        assert_eq!(result.standard[0].changed, 133665165395720108);
        assert_eq!(result.standard[0].sid_id, 257);
    }
}
