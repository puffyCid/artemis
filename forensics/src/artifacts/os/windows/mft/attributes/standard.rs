use crate::{
    filesystem::ntfs::attributes::file_attribute_flags,
    utils::nom_helper::{Endian, nom_unsigned_eight_bytes, nom_unsigned_four_bytes},
};
use common::windows::AttributeFlags;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct Standard {
    pub(crate) created: u64,
    pub(crate) modified: u64,
    pub(crate) changed: u64,
    pub(crate) accessed: u64,
    pub(crate) file_attributes: Vec<AttributeFlags>,
    pub(crate) file_attributes_data: u32,
    pub(crate) owner_id: u32,
    pub(crate) sid_id: u32,
    pub(crate) quota: u64,
    pub(crate) usn: u64,
}

impl Standard {
    /// Parse standard file attribute. Mainly contains four timestamps
    pub(crate) fn parse_standard_info(data: &[u8]) -> nom::IResult<&[u8], Standard> {
        let (input, created) = nom_unsigned_eight_bytes(data, Endian::Le)?;
        let (input, modified) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, changed) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, accessed) = nom_unsigned_eight_bytes(input, Endian::Le)?;

        let (input, flag_data) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, _unknown) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, _unknown) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, _unknown) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let mut standard = Standard {
            created,
            modified,
            changed,
            accessed,
            file_attributes: file_attribute_flags(&flag_data),
            file_attributes_data: flag_data,
            owner_id: 0,
            sid_id: 0,
            quota: 0,
            usn: 0,
        };
        // If NTFS version is lower than 3.0. Only have 48 bytes
        if input.is_empty() {
            return Ok((input, standard));
        }

        let (input, owner_id) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, sid_id) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let (input, quota) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, usn) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        standard.owner_id = owner_id;
        standard.sid_id = sid_id;
        standard.quota = quota;
        standard.usn = usn;

        Ok((input, standard))
    }
}

#[cfg(test)]
mod tests {
    use super::Standard;
    use common::windows::AttributeFlags;

    #[test]
    fn test_standard_attribute() {
        let test = [
            172, 119, 65, 126, 194, 223, 218, 1, 172, 119, 65, 126, 194, 223, 218, 1, 172, 119, 65,
            126, 194, 223, 218, 1, 172, 119, 65, 126, 194, 223, 218, 1, 6, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0,
        ];

        let (_, result) = Standard::parse_standard_info(&test).unwrap();
        assert_eq!(result.accessed, 133665165395720108);
        assert_eq!(result.modified, 133665165395720108);
        assert_eq!(result.created, 133665165395720108);
        assert_eq!(result.changed, 133665165395720108);
        assert_eq!(result.sid_id, 256);
        assert_eq!(
            result.file_attributes,
            vec![AttributeFlags::Hidden, AttributeFlags::System]
        );
    }
}
