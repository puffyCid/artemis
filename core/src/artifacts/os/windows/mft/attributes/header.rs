use crate::utils::nom_helper::{
    nom_unsigned_four_bytes, nom_unsigned_one_byte, nom_unsigned_two_bytes, Endian,
};
use log::warn;
use serde::Serialize;

#[derive(Debug)]
pub(crate) struct AttributeHeader {
    pub(crate) attrib_type: AttributeType,
    /**Includes the `attrib_type` and size itself */
    pub(crate) size: u32,
    /**
     * Includes the `attrib_type` and size itself. Sometimes size will include "overloaded or remnant data?"
     * <https://github.com/libyal/libfsntfs/blob/main/documentation/New%20Technologies%20File%20System%20(NTFS).asciidoc>
     *   "Size (or record length) upper 2 bytes overloaded or remnant data?"
     */
    pub(crate) small_size: u16,
    pub(crate) resident_flag: ResidentFlag,
    pub(crate) name_size: u8,
    pub(crate) name: String,
    _name_offset: u16,
    pub(crate) _data_flags: Vec<DataFlags>,
    _attrib_id: u16,
}

#[derive(Debug, PartialEq, Serialize)]
pub(crate) enum AttributeType {
    Unused,
    StandardInformation,
    AttributeList,
    FileName,
    /*Removed in NTFS version 3.0 */
    //VolumeVersion,
    ObjectId,
    SecurityDescriptor,
    VolumeName,
    VolumeInformation,
    Data,
    IndexRoot,
    IndexAllocation,
    Bitmap,
    /*Removed in NTFS version 3.0 */
    //SymbolicLink,
    ReparsePoint,
    ExtendedInfo,
    Extended,
    /*Removed in NTFS version 3.0 */
    PropertySet,
    LoggedStream,
    UserDefined,
    End,
    Unknown,
}

#[derive(Debug, PartialEq)]
pub(crate) enum ResidentFlag {
    Resident,
    NonResident,
    Unknown,
}

#[derive(Debug, PartialEq)]
pub(crate) enum DataFlags {
    /*Likely LZNT1? */
    Compressed,
    CompressionMask,
    Encrypted,
    Sparse,
}

impl AttributeHeader {
    /// Parse the attribute header info
    pub(crate) fn parse_header(data: &[u8]) -> nom::IResult<&[u8], AttributeHeader> {
        let (input, type_data) = nom_unsigned_four_bytes(data, Endian::Le)?;
        let (_, small_size) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, size) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, resident_data) = nom_unsigned_one_byte(input, Endian::Le)?;

        let (input, name_size) = nom_unsigned_one_byte(input, Endian::Le)?;
        let (input, name_offset) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, flag_data) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, attrib_id) = nom_unsigned_two_bytes(input, Endian::Le)?;

        let header = AttributeHeader {
            attrib_type: AttributeHeader::get_type(&type_data),
            size,
            small_size,
            resident_flag: AttributeHeader::get_resident(&resident_data),
            name_size,
            name: String::new(),
            _name_offset: name_offset,
            _data_flags: AttributeHeader::get_data_flags(&flag_data),
            _attrib_id: attrib_id,
        };

        Ok((input, header))
    }

    /// Determine attribute type
    pub(crate) fn get_type(data: &u32) -> AttributeType {
        match data {
            0x0 => AttributeType::Unused,
            0x10 => AttributeType::StandardInformation,
            0x20 => AttributeType::AttributeList,
            0x30 => AttributeType::FileName,
            0x40 => AttributeType::ObjectId,
            0x50 => AttributeType::SecurityDescriptor,
            0x60 => AttributeType::VolumeName,
            0x70 => AttributeType::VolumeInformation,
            0x80 => AttributeType::Data,
            0x90 => AttributeType::IndexRoot,
            0xa0 => AttributeType::IndexAllocation,
            0xb0 => AttributeType::Bitmap,
            0xc0 => AttributeType::ReparsePoint,
            0xd0 => AttributeType::ExtendedInfo,
            0xe0 => AttributeType::Extended,
            0xf0 => AttributeType::PropertySet,
            0x100 => AttributeType::LoggedStream,
            0x1000 => AttributeType::UserDefined,
            0xffffffff => AttributeType::End,
            _ => {
                warn!("[mft] Got unknown attribyte type {data}");
                AttributeType::Unknown
            }
        }
    }

    /// Determine if data is resident or non-resident
    fn get_resident(data: &u8) -> ResidentFlag {
        match data {
            0x0 => ResidentFlag::Resident,
            0x1 => ResidentFlag::NonResident,
            _ => ResidentFlag::Unknown,
        }
    }

    /// Determine data flags for the file
    fn get_data_flags(data: &u16) -> Vec<DataFlags> {
        let mut flags = Vec::new();
        if (data & 0x1) == 0x1 {
            flags.push(DataFlags::Compressed);
        }
        if (data & 0xff) == 0xff {
            flags.push(DataFlags::CompressionMask);
        }
        if (data & 0x4000) == 0x4000 {
            flags.push(DataFlags::Encrypted);
        }
        if (data & 0x8000) == 0x8000 {
            flags.push(DataFlags::Sparse);
        }

        flags
    }
}

#[cfg(test)]
mod tests {
    use super::AttributeHeader;
    use crate::artifacts::os::windows::mft::attributes::header::{
        AttributeType, DataFlags, ResidentFlag,
    };

    #[test]
    fn test_parse_header() {
        let test = [16, 0, 0, 0, 96, 0, 0, 0, 0, 0, 24, 0, 0, 0, 0, 0];
        let (_, result) = AttributeHeader::parse_header(&test).unwrap();
        assert_eq!(result.attrib_type, AttributeType::StandardInformation);
        assert_eq!(result.size, 96);
        assert_eq!(result._name_offset, 24);
        assert_eq!(result.resident_flag, ResidentFlag::Resident);
    }

    #[test]
    fn test_get_type() {
        let test = [
            0x0, 0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80, 0x90, 0xa0, 0xb0, 0xc0, 0xd0,
            0xf0, 0x100, 0x1000, 0xffffffff,
        ];
        for entry in test {
            let result = AttributeHeader::get_type(&entry);
            assert_ne!(result, AttributeType::Unknown);
        }
    }

    #[test]
    fn test_get_resident() {
        let result = AttributeHeader::get_resident(&3);
        assert_eq!(result, ResidentFlag::Unknown);
    }

    #[test]
    fn test_get_data_flags() {
        let result = AttributeHeader::get_data_flags(&0x4000);
        assert_eq!(result, vec![DataFlags::Encrypted]);
    }
}
