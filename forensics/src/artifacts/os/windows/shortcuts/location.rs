use crate::utils::{
    nom_helper::{Endian, nom_unsigned_four_bytes},
    strings::{extract_utf8_string, extract_utf16_string},
};
use common::windows::LocationFlag;
use nom::{
    Needed,
    bytes::complete::{take, take_while},
};

#[derive(Debug)]
pub(crate) struct LnkLocation {
    pub(crate) size: u32,
    _header_size: u32,
    pub(crate) flags: LocationFlag,
    pub(crate) volume_offset: u32,
    local_path_offset: u32,
    pub(crate) network_share_offset: u32,
    common_path_offset: u32,
    unicode_local_path_offset: u32,
    unicode_common_path_offset: u32,
    pub(crate) local_path: String,
    pub(crate) common_path: String,
    pub(crate) unicode_local_path: String,
    pub(crate) unicode_common_path: String,
}

impl LnkLocation {
    /// Parse the Location information from `shortcut` data
    pub(crate) fn parse_location(data: &[u8]) -> nom::IResult<&[u8], LnkLocation> {
        let (input, size) = nom_unsigned_four_bytes(data, Endian::Le)?;

        // Size includes the size itself (4 bytes)
        let adjust_size = 4;
        if size < adjust_size {
            return Err(nom::Err::Incomplete(Needed::Unknown));
        }
        let (remaining_input, input) = take(size - adjust_size)(input)?;
        let (input, header_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, flag) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let netork_flag = 2;
        // Location will either be on disk or network device
        let flags = if flag == netork_flag {
            LocationFlag::CommonNetworkRelativeLinkAndPathSuffix
        } else {
            LocationFlag::VolumeIDAndLocalBasePath
        };

        let (input, volume_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, local_path_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, network_share_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (mut input, common_path_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let mut location = LnkLocation {
            size,
            _header_size: header_size,
            flags,
            volume_offset,
            local_path_offset,
            network_share_offset,
            common_path_offset,
            unicode_local_path_offset: 0,
            unicode_common_path_offset: 0,
            local_path: String::new(),
            common_path: String::new(),
            unicode_local_path: String::new(),
            unicode_common_path: String::new(),
        };

        let no_path = 0;
        // According to Microsoft the offset should never be greater than the size
        // https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-shllink/16cb4ca1-9339-4d0c-a68d-bf1d6cc0f943
        if location.local_path_offset != no_path && location.local_path_offset <= size {
            let (path_start, _) = take(location.local_path_offset)(data)?;
            let (_, path_data) = take_while(|b| b != 0)(path_start)?;
            location.local_path = extract_utf8_string(path_data);
        }

        // According to Microsoft the offset should never be greater than the size
        // https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-shllink/16cb4ca1-9339-4d0c-a68d-bf1d6cc0f943
        if location.common_path_offset != no_path && common_path_offset <= size {
            let (path_start, _) = take(common_path_offset)(data)?;
            let (_, path_data) = take_while(|b| b != 0)(path_start)?;
            location.common_path = extract_utf8_string(path_data);
        }

        let has_unicode_local_path = 28;
        if header_size > has_unicode_local_path {
            let (unicode_input, offset) = nom_unsigned_four_bytes(input, Endian::Le)?;

            input = unicode_input;
            location.unicode_local_path_offset = offset;
            // According to Microsoft the offset should never be greater than the size
            // https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-shllink/16cb4ca1-9339-4d0c-a68d-bf1d6cc0f943
            if location.unicode_local_path_offset > size {
                return Ok((remaining_input, location));
            }
        }

        let has_unicode_common_path = 32;
        if header_size > has_unicode_common_path {
            let (_, offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
            location.unicode_common_path_offset = offset;
            // According to Microsoft the offset should never be greater than the size
            // https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-shllink/16cb4ca1-9339-4d0c-a68d-bf1d6cc0f943
            if location.unicode_common_path_offset > size {
                return Ok((remaining_input, location));
            }
        }

        if header_size > has_unicode_local_path {
            let (_, unicode_local_path_start) = take(location.unicode_local_path_offset)(data)?;
            location.unicode_local_path = extract_utf16_string(unicode_local_path_start);
        }

        if header_size > has_unicode_common_path {
            let (_, unicode_common_path_start) = take(location.unicode_common_path_offset)(data)?;
            location.unicode_common_path = extract_utf16_string(unicode_common_path_start);
        }

        Ok((remaining_input, location))
    }
}

#[cfg(test)]
mod tests {
    use super::LnkLocation;
    use crate::artifacts::os::windows::shortcuts::location::LocationFlag;

    #[test]
    fn test_parse_location() {
        let test = [
            101, 0, 0, 0, 28, 0, 0, 0, 1, 0, 0, 0, 28, 0, 0, 0, 45, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0,
            0, 17, 0, 0, 0, 3, 0, 0, 0, 62, 147, 144, 66, 16, 0, 0, 0, 0, 67, 58, 92, 85, 115, 101,
            114, 115, 92, 98, 111, 98, 92, 80, 114, 111, 106, 101, 99, 116, 115, 92, 97, 114, 116,
            101, 109, 105, 115, 45, 99, 111, 114, 101, 92, 115, 114, 99, 92, 102, 105, 108, 101,
            115, 121, 115, 116, 101, 109, 92, 110, 116, 102, 115, 0, 0,
        ];

        let (_, results) = LnkLocation::parse_location(&test).unwrap();
        assert_eq!(results.size, 101);
        assert_eq!(results._header_size, 28);
        assert_eq!(results.flags, LocationFlag::VolumeIDAndLocalBasePath);
        assert_eq!(results.volume_offset, 28);
        assert_eq!(results.local_path_offset, 45);
        assert_eq!(results.network_share_offset, 0);
        assert_eq!(results.common_path_offset, 100);
        assert_eq!(results.unicode_local_path_offset, 0);
        assert_eq!(results.unicode_common_path_offset, 0);
        assert_eq!(
            results.local_path,
            "C:\\Users\\bob\\Projects\\artemis-core\\src\\filesystem\\ntfs"
        );
        assert_eq!(results.common_path, "");
        assert_eq!(results.unicode_common_path, "");
        assert_eq!(results.unicode_local_path, "");
    }
}
