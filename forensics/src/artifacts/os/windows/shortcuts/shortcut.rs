use super::{
    extras::{
        codepage::has_codepage, console::has_console, darwin::has_darwin,
        environment::has_environment, items::has_item, known::has_known, shim::has_shim,
        special::has_special,
    },
    location::LnkLocation,
    network::LnkNetwork,
    shellitems::parse_lnk_shellitems,
    volume::LnkVolume,
};

use crate::artifacts::os::windows::shortcuts::{
    extras::{property::has_property, tracker::has_tracker},
    strings::extract_string,
};
use crate::{artifacts::os::windows::shortcuts::header::LnkHeader, utils::time::unixepoch_to_iso};
use common::windows::DataFlags::{
    HasArguements, HasIconLocation, HasLinkInfo, HasName, HasRelativePath, HasTargetIdList,
    HasWorkingDirectory,
};
use common::windows::LocationFlag::{
    CommonNetworkRelativeLinkAndPathSuffix, VolumeIDAndLocalBasePath,
};
use common::windows::{DriveType, LocationFlag, NetworkProviderType, ShortcutInfo};
use nom::bytes::complete::take;

/// Parse and grab `shortcut` info from provided bytes
pub(crate) fn get_shortcut_data(data: &[u8]) -> nom::IResult<&[u8], ShortcutInfo> {
    let (input, header) = LnkHeader::parse_header(data)?;

    let mut shortcut_info = ShortcutInfo {
        source_path: String::new(),
        data_flags: header.data_flags,
        attribute_flags: header.attribute_flags,
        created: unixepoch_to_iso(header.created),
        modified: unixepoch_to_iso(header.modified),
        accessed: unixepoch_to_iso(header.access),
        file_size: header.file_size,
        location_flags: LocationFlag::None,
        path: String::new(),
        drive_serial: String::new(),
        drive_type: DriveType::None,
        volume_label: String::new(),
        network_provider: NetworkProviderType::None,
        network_share_name: String::new(),
        network_device_name: String::new(),
        description: String::new(),
        relative_path: String::new(),
        working_directory: String::new(),
        command_line_args: String::new(),
        icon_location: String::new(),
        hostname: String::new(),
        droid_volume_id: String::new(),
        droid_file_id: String::new(),
        birth_droid_volume_id: String::new(),
        birth_droid_file_id: String::new(),
        shellitems: Vec::new(),
        properties: Vec::new(),
        environment_variable: String::new(),
        console: Vec::new(),
        codepage: 0,
        special_folder_id: 0,
        darwin_id: String::new(),
        shim_layer: String::new(),
        known_folder: String::new(),
        is_abnormal: false,
    };

    let (input, _) = get_shortcut_info(input, &mut shortcut_info)?;

    Ok((input, shortcut_info))
}

/// Parse the structure of `shortcut` data
fn get_shortcut_info<'a>(
    data: &'a [u8],
    shortcut_info: &mut ShortcutInfo,
) -> nom::IResult<&'a [u8], ()> {
    let mut input = data;

    // Based on flags in `Shortcut` header parse other parts of the structure
    for flags in &shortcut_info.data_flags {
        // Two (2) structures may follow the header
        //  TargetIDList - List of `shellitems`
        //  LocationInfo - Where the target file the `shortcut` points to exists. Either on disk or network device (ex: network share)
        if flags == &HasTargetIdList {
            let (remaining_input, shellitems) = parse_lnk_shellitems(input)?;
            shortcut_info.shellitems = shellitems;
            input = remaining_input;
        }

        if flags == &HasLinkInfo {
            let (remaining_input, location) = LnkLocation::parse_location(input)?;
            shortcut_info.location_flags = location.flags;
            shortcut_info.path = location.local_path;

            if shortcut_info.location_flags == CommonNetworkRelativeLinkAndPathSuffix {
                let (network_data, _) = take(location.network_share_offset)(input)?;
                let (_, network_share) = LnkNetwork::parse_network(network_data)?;
                shortcut_info.network_device_name = network_share.device_name;
                shortcut_info.network_share_name = network_share.share_name;
                shortcut_info.network_provider = network_share.provider_type;
            } else if shortcut_info.location_flags == VolumeIDAndLocalBasePath {
                let (volume_data, _) = take(location.volume_offset)(input)?;
                let (_, volume) = LnkVolume::parse_volume(volume_data)?;
                shortcut_info.volume_label = volume.volume_label;
                shortcut_info.drive_serial = volume.drive_serial;
                shortcut_info.drive_type = volume.drive_type;
            }

            input = remaining_input;
        }

        // After TargetIDList and LocationInfo five (5) strings may exists depending on the flags set in the header
        if flags == &HasName {
            let (remaining_input, (description, is_abnormal)) =
                extract_string(input, &shortcut_info.data_flags, false)?;
            input = remaining_input;

            shortcut_info.description = description;
            if is_abnormal {
                shortcut_info.is_abnormal = is_abnormal;
            }
        }

        if flags == &HasRelativePath {
            let (remaining_input, (relative_path, is_abnormal)) =
                extract_string(input, &shortcut_info.data_flags, false)?;
            input = remaining_input;

            shortcut_info.relative_path = relative_path;
            shortcut_info.is_abnormal = is_abnormal;
        }

        if flags == &HasWorkingDirectory {
            let (remaining_input, (working_dir, is_abnormal)) =
                extract_string(input, &shortcut_info.data_flags, false)?;
            input = remaining_input;

            shortcut_info.working_directory = working_dir;
            if is_abnormal {
                shortcut_info.is_abnormal = is_abnormal;
            }
        }

        if flags == &HasArguements {
            let (remaining_input, (args, is_abnormal)) =
                extract_string(input, &shortcut_info.data_flags, true)?;
            input = remaining_input;

            shortcut_info.command_line_args = args;
            if is_abnormal {
                shortcut_info.is_abnormal = is_abnormal;
            }
        }

        if flags == &HasIconLocation {
            let (remaining_input, (icon_path, is_abnormal)) =
                extract_string(input, &shortcut_info.data_flags, false)?;
            input = remaining_input;

            shortcut_info.icon_location = icon_path;
            if is_abnormal {
                shortcut_info.is_abnormal = is_abnormal;
            }
        }
    }

    let (found_tracker, tracker) = has_tracker(input);
    if found_tracker {
        shortcut_info.birth_droid_file_id = tracker.birth_droid_file_id;
        shortcut_info.birth_droid_volume_id = tracker.birth_droid_volume_id;
        shortcut_info.droid_file_id = tracker.droid_file_id;
        shortcut_info.droid_volume_id = tracker.droid_volume_id;
        shortcut_info.hostname = tracker.machine_id;
    }
    let (found_prop, stores) = has_property(input);
    if found_prop {
        shortcut_info.properties = stores;
    }
    let (found_env, path) = has_environment(input);
    if found_env {
        shortcut_info.environment_variable = path;
    }

    let (found_console, console) = has_console(data);
    if found_console {
        shortcut_info.console = console;
    }

    let (found_page, codepage) = has_codepage(data);
    if found_page {
        shortcut_info.codepage = codepage;
    }

    let (found_special, special) = has_special(data);
    if found_special {
        shortcut_info.special_folder_id = special;
    }

    let (found_darwin, darwin) = has_darwin(data);
    if found_darwin {
        shortcut_info.darwin_id = darwin;
    }

    let (found_shim, shim) = has_shim(data);
    if found_shim {
        shortcut_info.shim_layer = shim;
    }

    let (found_known, known) = has_known(data);
    if found_known {
        shortcut_info.known_folder = known;
    }

    let (has_items, mut items) = has_item(data);
    if has_items {
        shortcut_info.shellitems.append(&mut items);
    }

    Ok((input, ()))
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::shortcuts::header::LnkHeader;
    use crate::artifacts::os::windows::shortcuts::shortcut::{
        ShortcutInfo, get_shortcut_data, get_shortcut_info,
    };
    use crate::utils::time::unixepoch_to_iso;
    use common::windows::AttributeFlags;
    use common::windows::LocationFlag;
    use common::windows::ShellType::{Delegate, Directory, RootFolder};
    use common::windows::{DataFlags, DriveType, NetworkProviderType, ShellItem};

    #[test]
    fn test_get_shortcut_data() {
        let test = [
            76, 0, 0, 0, 1, 20, 2, 0, 0, 0, 0, 0, 192, 0, 0, 0, 0, 0, 0, 70, 139, 0, 32, 0, 16, 0,
            0, 0, 230, 35, 108, 77, 41, 239, 216, 1, 66, 63, 211, 253, 148, 11, 217, 1, 159, 47,
            36, 163, 148, 11, 217, 1, 0, 16, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 76, 1, 20, 0, 31, 68, 71, 26, 3, 89, 114, 63, 167, 68, 137, 197, 85, 149,
            254, 107, 48, 238, 134, 0, 116, 0, 30, 0, 67, 70, 83, 70, 24, 0, 49, 0, 0, 0, 0, 0, 62,
            82, 204, 166, 16, 0, 80, 114, 111, 106, 101, 99, 116, 115, 0, 0, 0, 0, 116, 26, 89, 94,
            150, 223, 211, 72, 141, 103, 23, 51, 188, 238, 40, 186, 197, 205, 250, 223, 159, 103,
            86, 65, 137, 71, 197, 199, 107, 192, 182, 127, 66, 0, 9, 0, 4, 0, 239, 190, 85, 79,
            123, 22, 62, 82, 204, 166, 46, 0, 0, 0, 13, 117, 3, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 87, 118, 218, 0, 80, 0, 114, 0, 111, 0, 106, 0, 101, 0, 99, 0,
            116, 0, 115, 0, 0, 0, 68, 0, 78, 0, 49, 0, 0, 0, 0, 0, 99, 85, 46, 17, 16, 0, 82, 117,
            115, 116, 0, 0, 58, 0, 9, 0, 4, 0, 239, 190, 88, 85, 66, 13, 137, 85, 33, 36, 46, 0, 0,
            0, 79, 76, 17, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 26, 88, 14, 0,
            82, 0, 117, 0, 115, 0, 116, 0, 0, 0, 20, 0, 98, 0, 49, 0, 0, 0, 0, 0, 135, 85, 81, 26,
            16, 0, 65, 82, 84, 69, 77, 73, 126, 49, 0, 0, 74, 0, 9, 0, 4, 0, 239, 190, 99, 85, 46,
            17, 137, 85, 51, 36, 46, 0, 0, 0, 159, 49, 12, 0, 0, 0, 21, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 75, 189, 183, 0, 97, 0, 114, 0, 116, 0, 101, 0, 109, 0, 105, 0, 115,
            0, 45, 0, 99, 0, 111, 0, 114, 0, 101, 0, 0, 0, 24, 0, 0, 0, 86, 0, 0, 0, 28, 0, 0, 0,
            1, 0, 0, 0, 28, 0, 0, 0, 45, 0, 0, 0, 0, 0, 0, 0, 85, 0, 0, 0, 17, 0, 0, 0, 3, 0, 0, 0,
            111, 18, 157, 212, 16, 0, 0, 0, 0, 67, 58, 92, 85, 115, 101, 114, 115, 92, 98, 111, 98,
            92, 80, 114, 111, 106, 101, 99, 116, 115, 92, 82, 117, 115, 116, 92, 97, 114, 116, 101,
            109, 105, 115, 45, 99, 111, 114, 101, 0, 0, 41, 0, 46, 0, 46, 0, 92, 0, 46, 0, 46, 0,
            92, 0, 46, 0, 46, 0, 92, 0, 46, 0, 46, 0, 92, 0, 46, 0, 46, 0, 92, 0, 80, 0, 114, 0,
            111, 0, 106, 0, 101, 0, 99, 0, 116, 0, 115, 0, 92, 0, 82, 0, 117, 0, 115, 0, 116, 0,
            92, 0, 97, 0, 114, 0, 116, 0, 101, 0, 109, 0, 105, 0, 115, 0, 45, 0, 99, 0, 111, 0,
            114, 0, 101, 0, 96, 0, 0, 0, 3, 0, 0, 160, 88, 0, 0, 0, 0, 0, 0, 0, 100, 101, 115, 107,
            116, 111, 112, 45, 101, 105, 115, 57, 51, 56, 110, 0, 104, 69, 141, 62, 17, 228, 24,
            73, 143, 120, 151, 205, 108, 179, 64, 197, 192, 88, 241, 9, 106, 90, 237, 17, 161, 13,
            8, 0, 39, 110, 180, 94, 104, 69, 141, 62, 17, 228, 24, 73, 143, 120, 151, 205, 108,
            179, 64, 197, 192, 88, 241, 9, 106, 90, 237, 17, 161, 13, 8, 0, 39, 110, 180, 94, 69,
            0, 0, 0, 9, 0, 0, 160, 57, 0, 0, 0, 49, 83, 80, 83, 177, 22, 109, 68, 173, 141, 112,
            72, 167, 72, 64, 46, 164, 61, 120, 140, 29, 0, 0, 0, 104, 0, 0, 0, 0, 72, 0, 0, 0, 144,
            47, 84, 8, 0, 0, 0, 0, 0, 0, 80, 31, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let (_, result) = get_shortcut_data(&test).unwrap();
        assert_eq!(result.created, "2022-11-03T02:09:27.000Z");
        assert_eq!(result.modified, "2022-12-09T06:08:20.000Z");
        assert_eq!(result.accessed, "2022-12-09T06:10:52.000Z");

        assert_eq!(
            result.data_flags,
            [
                DataFlags::HasTargetIdList,
                DataFlags::HasLinkInfo,
                DataFlags::HasRelativePath,
                DataFlags::IsUnicode,
                DataFlags::DisableKnownFolderTracking
            ]
        );
        assert_eq!(result.attribute_flags, [AttributeFlags::Directory]);
        assert_eq!(result.file_size, 4096);
        assert_eq!(
            result.location_flags,
            LocationFlag::VolumeIDAndLocalBasePath
        );
        assert_eq!(result.path, "C:\\Users\\bob\\Projects\\Rust\\artemis-core");
        assert_eq!(result.drive_serial, "D49D126F");
        assert_eq!(result.drive_type, DriveType::DriveFixed);
        assert_eq!(
            result.relative_path,
            "..\\..\\..\\..\\..\\Projects\\Rust\\artemis-core"
        );
        assert_eq!(
            result.shellitems,
            vec![
                ShellItem {
                    value: String::from("59031a47-3f72-44a7-89c5-5595fe6b30ee"),
                    shell_type: RootFolder,
                    created: String::from("1970-01-01T00:00:00.000Z"),
                    modified: String::from("1970-01-01T00:00:00.000Z"),
                    accessed: String::from("1970-01-01T00:00:00.000Z"),
                    mft_entry: 0,
                    mft_sequence: 0,
                    stores: vec![],
                },
                ShellItem {
                    value: String::from("Projects"),
                    shell_type: Delegate,
                    created: "2019-10-21T02:51:54.000Z".to_string(),
                    modified: "2021-01-30T20:54:24.000Z".to_string(),
                    accessed: "2021-01-30T20:54:24.000Z".to_string(),
                    mft_entry: 226573,
                    mft_sequence: 7,
                    stores: vec![],
                },
                ShellItem {
                    value: String::from("Rust"),
                    shell_type: Directory,
                    created: "2022-10-24T01:42:04.000Z".to_string(),
                    modified: "2022-11-03T02:09:28.000Z".to_string(),
                    accessed: "2022-12-09T04:33:02.000Z".to_string(),
                    mft_entry: 1133647,
                    mft_sequence: 4,
                    stores: vec![],
                },
                ShellItem {
                    value: String::from("artemis-core"),
                    shell_type: Directory,
                    created: "2022-11-03T02:09:28.000Z".to_string(),
                    modified: "2022-12-07T03:18:34.000Z".to_string(),
                    accessed: "2022-12-09T04:33:38.000Z".to_string(),
                    mft_entry: 799135,
                    mft_sequence: 21,
                    stores: vec![],
                }
            ]
        );
        assert_eq!(result.properties.len(), 1);
        assert_eq!(result.hostname, "desktop-eis938n");
        assert!(!result.is_abnormal);
        assert_eq!(
            result.birth_droid_file_id,
            "09f158c0-5a6a-11ed-a10d-0800276eb45e"
        );
        assert_eq!(
            result.birth_droid_volume_id,
            "3e8d4568-e411-4918-8f78-97cd6cb340c5"
        );
        assert_eq!(result.droid_file_id, "09f158c0-5a6a-11ed-a10d-0800276eb45e");
        assert_eq!(
            result.droid_volume_id,
            "3e8d4568-e411-4918-8f78-97cd6cb340c5"
        );
    }

    #[test]
    fn test_get_shortcut_info() {
        let test = [
            76, 0, 0, 0, 1, 20, 2, 0, 0, 0, 0, 0, 192, 0, 0, 0, 0, 0, 0, 70, 139, 0, 32, 0, 16, 0,
            0, 0, 230, 35, 108, 77, 41, 239, 216, 1, 66, 63, 211, 253, 148, 11, 217, 1, 159, 47,
            36, 163, 148, 11, 217, 1, 0, 16, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 76, 1, 20, 0, 31, 68, 71, 26, 3, 89, 114, 63, 167, 68, 137, 197, 85, 149,
            254, 107, 48, 238, 134, 0, 116, 0, 30, 0, 67, 70, 83, 70, 24, 0, 49, 0, 0, 0, 0, 0, 62,
            82, 204, 166, 16, 0, 80, 114, 111, 106, 101, 99, 116, 115, 0, 0, 0, 0, 116, 26, 89, 94,
            150, 223, 211, 72, 141, 103, 23, 51, 188, 238, 40, 186, 197, 205, 250, 223, 159, 103,
            86, 65, 137, 71, 197, 199, 107, 192, 182, 127, 66, 0, 9, 0, 4, 0, 239, 190, 85, 79,
            123, 22, 62, 82, 204, 166, 46, 0, 0, 0, 13, 117, 3, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 87, 118, 218, 0, 80, 0, 114, 0, 111, 0, 106, 0, 101, 0, 99, 0,
            116, 0, 115, 0, 0, 0, 68, 0, 78, 0, 49, 0, 0, 0, 0, 0, 99, 85, 46, 17, 16, 0, 82, 117,
            115, 116, 0, 0, 58, 0, 9, 0, 4, 0, 239, 190, 88, 85, 66, 13, 137, 85, 33, 36, 46, 0, 0,
            0, 79, 76, 17, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 26, 88, 14, 0,
            82, 0, 117, 0, 115, 0, 116, 0, 0, 0, 20, 0, 98, 0, 49, 0, 0, 0, 0, 0, 135, 85, 81, 26,
            16, 0, 65, 82, 84, 69, 77, 73, 126, 49, 0, 0, 74, 0, 9, 0, 4, 0, 239, 190, 99, 85, 46,
            17, 137, 85, 51, 36, 46, 0, 0, 0, 159, 49, 12, 0, 0, 0, 21, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 75, 189, 183, 0, 97, 0, 114, 0, 116, 0, 101, 0, 109, 0, 105, 0, 115,
            0, 45, 0, 99, 0, 111, 0, 114, 0, 101, 0, 0, 0, 24, 0, 0, 0, 86, 0, 0, 0, 28, 0, 0, 0,
            1, 0, 0, 0, 28, 0, 0, 0, 45, 0, 0, 0, 0, 0, 0, 0, 85, 0, 0, 0, 17, 0, 0, 0, 3, 0, 0, 0,
            111, 18, 157, 212, 16, 0, 0, 0, 0, 67, 58, 92, 85, 115, 101, 114, 115, 92, 98, 111, 98,
            92, 80, 114, 111, 106, 101, 99, 116, 115, 92, 82, 117, 115, 116, 92, 97, 114, 116, 101,
            109, 105, 115, 45, 99, 111, 114, 101, 0, 0, 41, 0, 46, 0, 46, 0, 92, 0, 46, 0, 46, 0,
            92, 0, 46, 0, 46, 0, 92, 0, 46, 0, 46, 0, 92, 0, 46, 0, 46, 0, 92, 0, 80, 0, 114, 0,
            111, 0, 106, 0, 101, 0, 99, 0, 116, 0, 115, 0, 92, 0, 82, 0, 117, 0, 115, 0, 116, 0,
            92, 0, 97, 0, 114, 0, 116, 0, 101, 0, 109, 0, 105, 0, 115, 0, 45, 0, 99, 0, 111, 0,
            114, 0, 101, 0, 96, 0, 0, 0, 3, 0, 0, 160, 88, 0, 0, 0, 0, 0, 0, 0, 100, 101, 115, 107,
            116, 111, 112, 45, 101, 105, 115, 57, 51, 56, 110, 0, 104, 69, 141, 62, 17, 228, 24,
            73, 143, 120, 151, 205, 108, 179, 64, 197, 192, 88, 241, 9, 106, 90, 237, 17, 161, 13,
            8, 0, 39, 110, 180, 94, 104, 69, 141, 62, 17, 228, 24, 73, 143, 120, 151, 205, 108,
            179, 64, 197, 192, 88, 241, 9, 106, 90, 237, 17, 161, 13, 8, 0, 39, 110, 180, 94, 69,
            0, 0, 0, 9, 0, 0, 160, 57, 0, 0, 0, 49, 83, 80, 83, 177, 22, 109, 68, 173, 141, 112,
            72, 167, 72, 64, 46, 164, 61, 120, 140, 29, 0, 0, 0, 104, 0, 0, 0, 0, 72, 0, 0, 0, 144,
            47, 84, 8, 0, 0, 0, 0, 0, 0, 80, 31, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let (input, header) = LnkHeader::parse_header(&test).unwrap();

        let mut shortcut_info = ShortcutInfo {
            source_path: String::new(),
            data_flags: header.data_flags,
            attribute_flags: header.attribute_flags,
            created: unixepoch_to_iso(header.created),
            modified: unixepoch_to_iso(header.modified),
            accessed: unixepoch_to_iso(header.access),
            file_size: header.file_size,
            location_flags: LocationFlag::None,
            path: String::new(),
            drive_serial: String::new(),
            drive_type: DriveType::None,
            volume_label: String::new(),
            network_provider: NetworkProviderType::None,
            network_share_name: String::new(),
            network_device_name: String::new(),
            description: String::new(),
            relative_path: String::new(),
            working_directory: String::new(),
            command_line_args: String::new(),
            icon_location: String::new(),
            hostname: String::new(),
            droid_volume_id: String::new(),
            droid_file_id: String::new(),
            birth_droid_volume_id: String::new(),
            birth_droid_file_id: String::new(),
            shellitems: Vec::new(),
            properties: Vec::new(),
            environment_variable: String::new(),
            console: Vec::new(),
            codepage: 0,
            special_folder_id: 0,
            darwin_id: String::new(),
            shim_layer: String::new(),
            known_folder: String::new(),
            is_abnormal: false,
        };

        let (_, _) = get_shortcut_info(input, &mut shortcut_info).unwrap();
        assert_eq!(shortcut_info.created, "2022-11-03T02:09:27.000Z");
        assert_eq!(shortcut_info.modified, "2022-12-09T06:08:20.000Z");
        assert_eq!(shortcut_info.accessed, "2022-12-09T06:10:52.000Z");

        assert_eq!(
            shortcut_info.data_flags,
            [
                DataFlags::HasTargetIdList,
                DataFlags::HasLinkInfo,
                DataFlags::HasRelativePath,
                DataFlags::IsUnicode,
                DataFlags::DisableKnownFolderTracking
            ]
        );
        assert_eq!(shortcut_info.attribute_flags, [AttributeFlags::Directory]);
        assert_eq!(shortcut_info.file_size, 4096);
        assert_eq!(
            shortcut_info.location_flags,
            LocationFlag::VolumeIDAndLocalBasePath
        );
        assert_eq!(
            shortcut_info.path,
            "C:\\Users\\bob\\Projects\\Rust\\artemis-core"
        );
        assert_eq!(shortcut_info.drive_serial, "D49D126F");
        assert_eq!(shortcut_info.drive_type, DriveType::DriveFixed);
        assert_eq!(
            shortcut_info.relative_path,
            "..\\..\\..\\..\\..\\Projects\\Rust\\artemis-core"
        );
        assert_eq!(
            shortcut_info.shellitems,
            vec![
                ShellItem {
                    value: String::from("59031a47-3f72-44a7-89c5-5595fe6b30ee"),
                    shell_type: RootFolder,
                    created: String::from("1970-01-01T00:00:00.000Z"),
                    modified: String::from("1970-01-01T00:00:00.000Z"),
                    accessed: String::from("1970-01-01T00:00:00.000Z"),
                    mft_entry: 0,
                    mft_sequence: 0,
                    stores: vec![],
                },
                ShellItem {
                    value: String::from("Projects"),
                    shell_type: Delegate,
                    created: "2019-10-21T02:51:54.000Z".to_string(),
                    modified: "2021-01-30T20:54:24.000Z".to_string(),
                    accessed: "2021-01-30T20:54:24.000Z".to_string(),
                    mft_entry: 226573,
                    mft_sequence: 7,
                    stores: vec![],
                },
                ShellItem {
                    value: String::from("Rust"),
                    shell_type: Directory,
                    created: "2022-10-24T01:42:04.000Z".to_string(),
                    modified: "2022-11-03T02:09:28.000Z".to_string(),
                    accessed: "2022-12-09T04:33:02.000Z".to_string(),
                    mft_entry: 1133647,
                    mft_sequence: 4,
                    stores: vec![],
                },
                ShellItem {
                    value: String::from("artemis-core"),
                    shell_type: Directory,
                    created: "2022-11-03T02:09:28.000Z".to_string(),
                    modified: "2022-12-07T03:18:34.000Z".to_string(),
                    accessed: "2022-12-09T04:33:38.000Z".to_string(),
                    mft_entry: 799135,
                    mft_sequence: 21,
                    stores: vec![],
                }
            ]
        );
        assert_eq!(shortcut_info.properties.len(), 1);
        assert_eq!(shortcut_info.hostname, "desktop-eis938n");

        assert_eq!(
            shortcut_info.birth_droid_file_id,
            "09f158c0-5a6a-11ed-a10d-0800276eb45e"
        );
        assert_eq!(
            shortcut_info.birth_droid_volume_id,
            "3e8d4568-e411-4918-8f78-97cd6cb340c5"
        );
        assert_eq!(
            shortcut_info.droid_file_id,
            "09f158c0-5a6a-11ed-a10d-0800276eb45e"
        );
        assert_eq!(
            shortcut_info.droid_volume_id,
            "3e8d4568-e411-4918-8f78-97cd6cb340c5"
        );
    }
}
