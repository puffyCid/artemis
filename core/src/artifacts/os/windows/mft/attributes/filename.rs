use super::standard::Standard;
use crate::utils::{
    nom_helper::{
        nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_one_byte,
        nom_unsigned_two_bytes, Endian,
    },
    strings::extract_utf16_string,
};
use common::windows::{FileAttributes, Namespace};
use nom::bytes::complete::take;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct Filename {
    pub(crate) parent_mft: u32,
    pub(crate) parent_sequence: u16,
    pub(crate) created: u64,
    pub(crate) modified: u64,
    pub(crate) changed: u64,
    pub(crate) accessed: u64,
    pub(crate) allocated_size: u64,
    pub(crate) size: u64,
    pub(crate) file_attributes: Vec<FileAttributes>,
    pub(crate) file_attributes_data: u32,
    pub(crate) extended_data: u32,
    pub(crate) name_size: u8,
    pub(crate) namespace: Namespace,
    /**UTF16 (but not strict UTF16) */
    pub(crate) name: String,
}

impl Filename {
    /// Parse Filename attribute. Contains timestamps and the filename
    pub(crate) fn parse_filename(data: &[u8]) -> nom::IResult<&[u8], Filename> {
        let (input, parent_mft) = nom_unsigned_four_bytes(data, Endian::Le)?;
        let (input, _padding) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, parent_sequence) = nom_unsigned_two_bytes(input, Endian::Le)?;

        let (input, created) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, modified) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, changed) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, accessed) = nom_unsigned_eight_bytes(input, Endian::Le)?;

        let (input, allocated_size) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, size) = nom_unsigned_eight_bytes(input, Endian::Le)?;

        let (input, flag_data) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, extended_data) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, name_size) = nom_unsigned_one_byte(input, Endian::Le)?;
        let (input, namespace_data) = nom_unsigned_one_byte(input, Endian::Le)?;

        // adjust for UTF16. Double the name size
        let adjust = 2;
        let (input, name_data) = take(name_size as u16 * adjust)(input)?;
        let name = extract_utf16_string(name_data);

        let filename = Filename {
            parent_mft,
            parent_sequence,
            created,
            modified,
            changed,
            accessed,
            allocated_size,
            size,
            file_attributes: Standard::get_attributes(&flag_data),
            file_attributes_data: flag_data,
            extended_data,
            name_size,
            namespace: Filename::get_namespace(&namespace_data),
            name,
        };

        Ok((input, filename))
    }

    /// Determine Namespace associated with entry
    fn get_namespace(space: &u8) -> Namespace {
        match space {
            0 => Namespace::Posix,
            1 => Namespace::Windows,
            2 => Namespace::Dos,
            3 => Namespace::WindowsDos,
            _ => Namespace::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Filename;
    use crate::artifacts::os::windows::mft::attributes::filename::Namespace;

    #[test]
    fn test_parse_filename() {
        let test = [
            5, 0, 0, 0, 0, 0, 5, 0, 172, 119, 65, 126, 194, 223, 218, 1, 172, 119, 65, 126, 194,
            223, 218, 1, 172, 119, 65, 126, 194, 223, 218, 1, 172, 119, 65, 126, 194, 223, 218, 1,
            0, 0, 76, 59, 0, 0, 0, 0, 0, 0, 76, 59, 0, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0, 4, 3, 36,
            0, 77, 0, 70, 0, 84, 0,
        ];

        let (_, result) = Filename::parse_filename(&test).unwrap();
        assert_eq!(result.created, 133665165395720108);
        assert_eq!(result.changed, 133665165395720108);
        assert_eq!(result.accessed, 133665165395720108);
        assert_eq!(result.modified, 133665165395720108);
        assert_eq!(result.size, 994836480);
        assert_eq!(result.name, "$MFT");
        assert_eq!(result.namespace, Namespace::WindowsDos);
    }
}
