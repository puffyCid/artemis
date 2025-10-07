use crate::utils::{
    nom_helper::{Endian, nom_unsigned_four_bytes},
    strings::{extract_utf8_string, extract_utf16_string},
};
use common::windows::DriveType;
use nom::bytes::complete::{take, take_while};

#[derive(Debug)]
pub(crate) struct LnkVolume {
    size: u32,
    pub(crate) drive_type: DriveType,
    pub(crate) drive_serial: String,
    label_offset: u32,
    unicode_volume_label_offset: u32,
    pub(crate) volume_label: String,
    pub(crate) unicode_volume_label: String,
}

impl LnkVolume {
    /// Parse volume metadata from `shortcut` data
    pub(crate) fn parse_volume(data: &[u8]) -> nom::IResult<&[u8], LnkVolume> {
        let (input, size) = nom_unsigned_four_bytes(data, Endian::Le)?;

        // Size includes the size itself (4 bytes)
        let adjust_size = 4;
        let (remaining_input, input) = take(size - adjust_size)(input)?;

        let (input, drive_type) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, drive_serial) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, label_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let mut volume_info = LnkVolume {
            size,
            drive_type: LnkVolume::get_drive_type(drive_type),
            drive_serial: format!("{drive_serial:X}"),
            label_offset,
            unicode_volume_label_offset: 0,
            volume_label: String::new(),
            unicode_volume_label: String::new(),
        };

        // According to Microsoft the offset should never be greater than the size
        // https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-shllink/16cb4ca1-9339-4d0c-a68d-bf1d6cc0f943
        if label_offset > volume_info.size {
            return Ok((remaining_input, volume_info));
        }
        let has_unicode_offset = 16;
        if label_offset > has_unicode_offset {
            let (_, offset) = nom_unsigned_four_bytes(input, Endian::Le)?;

            volume_info.unicode_volume_label_offset = offset;
        }

        let (volume_label_start, _) = take(volume_info.label_offset)(data)?;
        let (_, volume_data) = take_while(|b| b != 0)(volume_label_start)?;
        volume_info.volume_label = extract_utf8_string(volume_data);

        // According to Microsoft the offset should never be greater than the size
        // https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-shllink/16cb4ca1-9339-4d0c-a68d-bf1d6cc0f943
        if volume_info.unicode_volume_label_offset > volume_info.size {
            return Ok((remaining_input, volume_info));
        }
        let no_unicode = 0;
        if volume_info.unicode_volume_label_offset != no_unicode {
            let (volume_label_start, _) = take(volume_info.unicode_volume_label_offset)(data)?;

            volume_info.unicode_volume_label = extract_utf16_string(volume_label_start);
        }
        Ok((remaining_input, volume_info))
    }

    /// Get drive types from `shortcut` data
    fn get_drive_type(drive_type: u32) -> DriveType {
        match drive_type {
            0 => DriveType::DriveUnknown,
            1 => DriveType::DriveNotRootDir,
            2 => DriveType::DriveRemovable,
            3 => DriveType::DriveFixed,
            4 => DriveType::DriveRemote,
            5 => DriveType::DriveCdrom,
            6 => DriveType::DriveRamdisk,
            _ => DriveType::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LnkVolume;
    use crate::artifacts::os::windows::shortcuts::volume::DriveType::{self, DriveNotRootDir};

    #[test]
    fn test_get_drive_type() {
        let test = 1;
        let result = LnkVolume::get_drive_type(test);
        assert_eq!(result, DriveNotRootDir);
    }

    #[test]
    fn test_parse_volume() {
        let test = [
            17, 0, 0, 0, 3, 0, 0, 0, 111, 18, 157, 212, 16, 0, 0, 0, 0, 67, 58, 92, 85, 115, 101,
            114, 115, 92, 98, 111, 98, 92, 80, 114, 111, 106, 101, 99, 116, 115, 92, 82, 117, 115,
            116, 92, 97, 114, 116, 101, 109, 105, 115, 45, 99, 111, 114, 101, 0, 0, 41, 0, 46, 0,
            46, 0, 92, 0, 46, 0, 46, 0, 92, 0, 46, 0, 46, 0, 92, 0, 46, 0, 46, 0, 92, 0, 46, 0, 46,
            0, 92, 0, 80, 0, 114, 0, 111, 0, 106, 0, 101, 0, 99, 0, 116, 0, 115, 0, 92, 0, 82, 0,
            117, 0, 115, 0, 116, 0, 92, 0, 97, 0, 114, 0, 116, 0, 101, 0, 109, 0, 105, 0, 115, 0,
            45, 0, 99, 0, 111, 0, 114, 0, 101, 0, 96, 0, 0, 0, 3, 0, 0, 160, 88, 0, 0, 0, 0, 0, 0,
            0, 100, 101, 115, 107, 116, 111, 112, 45, 101, 105, 115, 57, 51, 56, 110, 0, 104, 69,
            141, 62, 17, 228, 24, 73, 143, 120, 151, 205, 108, 179, 64, 197, 192, 88, 241, 9, 106,
            90, 237, 17, 161, 13, 8, 0, 39, 110, 180, 94, 104, 69, 141, 62, 17, 228, 24, 73, 143,
            120, 151, 205, 108, 179, 64, 197, 192, 88, 241, 9, 106, 90, 237, 17, 161, 13, 8, 0, 39,
            110, 180, 94, 69, 0, 0, 0, 9, 0, 0, 160, 57, 0, 0, 0, 49, 83, 80, 83, 177, 22, 109, 68,
            173, 141, 112, 72, 167, 72, 64, 46, 164, 61, 120, 140, 29, 0, 0, 0, 104, 0, 0, 0, 0,
            72, 0, 0, 0, 144, 47, 84, 8, 0, 0, 0, 0, 0, 0, 80, 31, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0,
        ];
        let (_, result) = LnkVolume::parse_volume(&test).unwrap();

        assert_eq!(result.drive_serial, "D49D126F");
        assert_eq!(result.size, 17);
        assert_eq!(result.drive_type, DriveType::DriveFixed);
        assert_eq!(result.label_offset, 16);
        assert_eq!(result.volume_label, "");
        assert_eq!(result.unicode_volume_label_offset, 0);
        assert_eq!(result.unicode_volume_label, "");
    }

    #[test]
    fn test_parse_bad_volume() {
        let test = [17, 0, 0, 0, 3, 0, 0, 0, 16, 0, 0, 0, 0, 67, 58, 92, 87, 105];
        let (_, result) = LnkVolume::parse_volume(&test).unwrap();
        assert_eq!(result.size, 17);
        assert_eq!(result.drive_type, DriveType::DriveFixed);
        // Bad offset
        assert_eq!(result.label_offset, 1547322112);
        assert_eq!(result.volume_label, "");
    }
}
