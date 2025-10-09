use crate::{
    artifacts::os::windows::pe::resources::{EventLogResource, read_eventlog_resource},
    filesystem::{
        directory::get_parent_directory,
        files::{get_filename, is_file},
    },
    utils::{
        nom_helper::{Endian, nom_unsigned_four_bytes},
        strings::extract_utf16_string,
    },
};
use log::error;
use nom::{bytes::complete::take, error::ErrorKind};

/// Parse MUI files. Used mainly for international languages
pub(crate) fn parse_mui<'a>(
    data: &'a [u8],
    path: &str,
) -> nom::IResult<&'a [u8], EventLogResource> {
    let (input, _sig) = nom_unsigned_four_bytes(data, Endian::Le)?;
    // Size is the entire data
    let (input, _size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, _version) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, _unknown) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, _file_type) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, _attributes) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, _fallback_location) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let checksum_size: u8 = 16;
    let (input, _service_checksum) = take(checksum_size)(input)?;
    let (input, _checksum) = take(checksum_size)(input)?;

    let unknown_size: u8 = 24;
    let (input, _unknown2) = take(unknown_size)(input)?;

    let (input, _main_name_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, _main_name_size) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let (input, _main_id_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, _main_id_size) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let (input, _main_name_type_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, _main_name_type_size) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let (input, _main_type_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, _main_type_size) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let (input, lang_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, lang_size) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let (input, fallback_lang_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (_input, fallback_lang_size) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let empty = 0;
    let lang = if lang_offset == empty && lang_size == empty {
        let (lang_start, _) = take(fallback_lang_offset)(data)?;
        let (_, lang_data) = take(fallback_lang_size)(lang_start)?;
        extract_utf16_string(lang_data)
    } else {
        let (lang_start, _) = take(lang_offset)(data)?;
        let (_, lang_data) = take(lang_size)(lang_start)?;
        extract_utf16_string(lang_data)
    };

    let real_path = if !path.ends_with(".mui") {
        let parent = get_parent_directory(path);
        let filename = get_filename(path);
        format!("{parent}\\{lang}\\{filename}.mui")
    } else {
        path.to_string()
    };

    if !is_file(&real_path) {
        error!("[eventlogs] No MUI file at {real_path}");
        return Err(nom::Err::Failure(nom::error::Error::new(
            &[],
            ErrorKind::Fail,
        )));
    }

    let resource_result = read_eventlog_resource(&real_path);
    let mut resource = match resource_result {
        Ok(result) => result,
        Err(err) => {
            error!("[eventlogs] Could not parse MUI file at {real_path}: {err:?}");
            return Err(nom::Err::Failure(nom::error::Error::new(
                &[],
                ErrorKind::Fail,
            )));
        }
    };

    // If we do not have `WEVT_TEMPLATE` data. It may be in a MUN file
    if resource.wevt_data.is_empty() && !path.is_empty() {
        let drive = path.chars().next().unwrap_or('C');
        let filename = get_filename(path);
        let mun_path = format!("{drive}:\\Windows\\SystemResources\\{filename}.mun");

        // If there is no MUN file and MUI file does not have the WEVT_TEMPLATE resource
        // The the original DLL will have it. Should get parsed at `parse_resource()`
        // Ex: C:\Windows\System32\wisp.dll. Has a MUI file for `MESSAGETABLE` but no `WEVT_TEMPLATE`. wisp.dll has the `WEVT_TEMPLATE`
        if !is_file(&mun_path) {
            return Ok((&[], resource));
        }

        let mun_resource = match read_eventlog_resource(&mun_path) {
            Ok(result) => result,
            Err(err) => {
                error!("[eventlogs] Could not parse MUN file at {real_path}: {err:?}");
                return Ok((&[], resource));
            }
        };
        resource.wevt_data = mun_resource.wevt_data;
    }

    Ok((&[], resource))
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::parse_mui;

    #[test]
    fn test_parse_mui() {
        let test = [
            205, 254, 205, 254, 240, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 17, 0, 0, 0, 0, 0, 0, 0, 2,
            0, 0, 0, 26, 143, 14, 5, 117, 105, 8, 126, 13, 52, 84, 38, 75, 63, 119, 183, 88, 247,
            216, 48, 42, 28, 12, 12, 115, 100, 155, 217, 179, 57, 173, 215, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 136, 0, 0, 0, 42, 0, 0, 0, 184, 0, 0,
            0, 4, 0, 0, 0, 192, 0, 0, 0, 14, 0, 0, 0, 208, 0, 0, 0, 12, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 224, 0, 0, 0, 12, 0, 0, 0, 0, 0, 0, 0, 87, 0, 69, 0, 86, 0, 84, 0, 95, 0, 84, 0,
            69, 0, 77, 0, 80, 0, 76, 0, 65, 0, 84, 0, 69, 0, 0, 0, 77, 0, 85, 0, 73, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 0, 0, 0, 0, 0, 0, 0, 77, 0, 85, 0, 73, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 11, 0, 0, 0, 16, 0, 0, 0, 0, 0, 0, 0, 101, 0, 110, 0, 45,
            0, 85, 0, 83, 0, 0, 0, 0, 0, 0, 0,
        ];

        let (_, result) = parse_mui(&test, "C:\\WINDOWS\\System32\\fdeploy.dll").unwrap();
        assert!(!result.message_data.is_empty());
    }
}
