use super::attribute::FileAttributes;
use crate::utils::nom_helper::{nom_unsigned_eight_bytes, nom_unsigned_four_bytes, Endian};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct Standard {
    pub(crate) created: u64,
    pub(crate) modified: u64,
    pub(crate) changed: u64,
    pub(crate) accessed: u64,
    pub(crate) file_attributes: Vec<FileAttributes>,
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
            file_attributes: Standard::get_attributes(&flag_data),
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

    /// Get file attributes
    pub(crate) fn get_attributes(data: &u32) -> Vec<FileAttributes> {
        let mut attrs = Vec::new();

        if (data & 0x1) == 0x1 {
            attrs.push(FileAttributes::ReadOnly);
        }
        if (data & 0x2) == 0x2 {
            attrs.push(FileAttributes::Hidden);
        }
        if (data & 0x4) == 0x4 {
            attrs.push(FileAttributes::System);
        }
        if (data & 0x8) == 0x8 {
            attrs.push(FileAttributes::Volume);
        }
        if (data & 0x10) == 0x10 {
            attrs.push(FileAttributes::Directory);
        }
        if (data & 0x20) == 0x20 {
            attrs.push(FileAttributes::Archive);
        }
        if (data & 0x40) == 0x40 {
            attrs.push(FileAttributes::Device);
        }
        if (data & 0x80) == 0x80 {
            attrs.push(FileAttributes::Normal);
        }
        if (data & 0x100) == 0x100 {
            attrs.push(FileAttributes::Temporary);
        }
        if (data & 0x200) == 0x200 {
            attrs.push(FileAttributes::Sparse);
        }
        if (data & 0x400) == 0x400 {
            attrs.push(FileAttributes::Reparse);
        }
        if (data & 0x800) == 0x800 {
            attrs.push(FileAttributes::Compressed);
        }
        if (data & 0x1000) == 0x1000 {
            attrs.push(FileAttributes::Offline);
        }
        if (data & 0x2000) == 0x2000 {
            attrs.push(FileAttributes::NotIndexed);
        }
        if (data & 0x4000) == 0x4000 {
            attrs.push(FileAttributes::Encrypted);
        }
        if (data & 0x8000) == 0x8000 {
            attrs.push(FileAttributes::Unknown);
        }
        if (data & 0x10000) == 0x10000 {
            attrs.push(FileAttributes::Virtual);
        }
        if (data & 0x10000000) == 0x10000000 {
            attrs.push(FileAttributes::Directory);
        }
        if (data & 0x20000000) == 0x20000000 {
            attrs.push(FileAttributes::IndexView);
        }

        attrs
    }
}

#[cfg(test)]
mod tests {
    use super::Standard;
    use crate::artifacts::os::windows::mft::attributes::standard::FileAttributes;

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
            vec![FileAttributes::Hidden, FileAttributes::System]
        );
    }

    #[test]
    fn test_get_attributes() {
        let test = 0x8000;
        let attributes = Standard::get_attributes(&test);
        assert_eq!(attributes, vec![FileAttributes::Unknown]);
    }
}
