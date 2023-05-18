/**
 * Windows `Shortcut` files are files that point to another file.  
 * The have the extension `lnk` and be found in any directory.  
 * The `Shortcut` files found in `C:\Users\<user>\AppData\Roaming\Microsoft\Windows\Recent` can be used to identify recent files and directories accessed by a user
 *
 * `Shortcut` files can also be used by malware to execute remote commands
 *
 * References:
 * `https://github.com/libyal/liblnk/blob/main/documentation/Windows%20Shortcut%20File%20(LNK)%20format.asciidoc`  
 * `https://winprotocoldoc.blob.core.windows.net/productionwindowsarchives/MS-SHLLINK/%5bMS-SHLLINK%5d.pdf`  
 * `https://www.intezer.com/blog/malware-analysis/how-threat-actors-abuse-lnk-files/`
 *
 * Other parsers:
 * `https://github.com/EricZimmerman/LECmd`  
 * `https://github.com/Velocidex/velociraptor`
 */
use super::{error::LnkError, header::LnkHeader, shortcut::ShortcutInfo};
use crate::filesystem::files::{list_files, read_file};
use log::error;

/// `Shortcut` files can be location anywhere. Provide a directory and parse any `lnk` (`Shortcut`) files
pub(crate) fn grab_lnk_directory(path: &str) -> Result<Vec<ShortcutInfo>, LnkError> {
    let files_results = list_files(path);
    let files = match files_results {
        Ok(results) => results,
        Err(err) => {
            error!("[shortcuts] Could not list files at path {path}: {err:?}");
            return Err(LnkError::ReadDirectory);
        }
    };

    let mut shortcut_info = Vec::new();
    for file in files {
        let result = grab_lnk_file(&file);
        match result {
            Ok(info) => shortcut_info.push(info),
            Err(_err) => {
                error!("[shortcuts] Failed to parse file: {file}");
                continue;
            }
        }
    }
    Ok(shortcut_info)
}

/// Parse a single `shortcut` file
pub(crate) fn grab_lnk_file(path: &str) -> Result<ShortcutInfo, LnkError> {
    let result = read_file(path);
    let lnk_data = match result {
        Ok(data) => data,
        Err(err) => {
            error!("[shortcuts] Could not read lnk file: {err:?}");
            return Err(LnkError::ReadFile);
        }
    };
    let mut shortcut_info = parse_lnk_data(&lnk_data)?;
    shortcut_info.source_path = path.to_string();
    Ok(shortcut_info)
}

/// Parse the raw bytes of `shortcut` data
pub(crate) fn parse_lnk_data(data: &[u8]) -> Result<ShortcutInfo, LnkError> {
    let result = LnkHeader::check_header(data);
    let is_header = match result {
        Ok((_, result)) => result,
        Err(_err) => {
            error!("[shortcuts] Could not parse lnk header");
            return Err(LnkError::BadHeader);
        }
    };

    if !is_header {
        return Err(LnkError::NotLnkData);
    }
    let shortcut_result = ShortcutInfo::get_shortcut_data(data);
    let shortcut_info = match shortcut_result {
        Ok((_, result)) => result,
        Err(_err) => {
            error!("[shortcuts] Could not parse shortcut data");
            return Err(LnkError::Parse);
        }
    };

    Ok(shortcut_info)
}

#[cfg(test)]
mod tests {
    use super::{grab_lnk_directory, grab_lnk_file};
    use crate::artifacts::os::windows::shellitems::items::ShellType::{
        Delegate, Directory, RootFolder,
    };
    use crate::artifacts::os::windows::shortcuts::parser::parse_lnk_data;
    use crate::artifacts::os::windows::{
        shellitems::items::ShellItem,
        shortcuts::{header::DataFlags, location::LocationFlag, volume::DriveType},
    };
    use crate::filesystem::directory::{get_user_paths, is_directory};
    use crate::filesystem::files::list_files;
    use crate::filesystem::ntfs::attributes::AttributeFlags;
    use std::path::PathBuf;

    #[test]
    fn test_recent_files() {
        let users = get_user_paths().unwrap();
        for user in users {
            let path = format!("{}\\AppData\\Roaming\\Microsoft\\Windows\\Recent", user);
            if !is_directory(&path) {
                continue;
            }
            let files = list_files(&path).unwrap();
            for file in files {
                if !file.ends_with("lnk") {
                    continue;
                }
                let result = grab_lnk_file(&file).unwrap();
                assert_eq!(result.source_path.ends_with("lnk"), true);
            }
        }
    }

    #[test]
    fn test_parse_lnk_data() {
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

        let result = parse_lnk_data(&test).unwrap();
        assert_eq!(result.created, 1667441367);
        assert_eq!(result.modified, 1670566100);
        assert_eq!(result.accessed, 1670566252);

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
                    created: 0,
                    modified: 0,
                    accessed: 0,
                    mft_entry: 0,
                    mft_sequence: 0
                },
                ShellItem {
                    value: String::from("Projects"),
                    shell_type: Delegate,
                    created: 1571626314,
                    modified: 1612040064,
                    accessed: 1612040064,
                    mft_entry: 226573,
                    mft_sequence: 7
                },
                ShellItem {
                    value: String::from("Rust"),
                    shell_type: Directory,
                    created: 1666575724,
                    modified: 1667441368,
                    accessed: 1670560382,
                    mft_entry: 1133647,
                    mft_sequence: 4
                },
                ShellItem {
                    value: String::from("artemis-core"),
                    shell_type: Directory,
                    created: 1667441368,
                    modified: 1670383114,
                    accessed: 1670560418,
                    mft_entry: 799135,
                    mft_sequence: 21
                }
            ]
        );
        assert_eq!(result.property_guid, "446d16b1-8dad-4870-a748-402ea43d788c");
        assert_eq!(result.hostname, "desktop-eis938n");

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
    fn test_dfir_artifact_win2012_lnk() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location
            .push("tests/test_data/dfir/windows/lnk/win2012/Windows.SystemToast.Share.lnk");
        let result = grab_lnk_file(&test_location.display().to_string()).unwrap();

        assert_eq!(
            result.data_flags,
            vec![
                DataFlags::IsUnicode,
                DataFlags::ForceNoLinkInfo,
                DataFlags::HasExpString,
                DataFlags::PreferEnvironmentPath
            ]
        );
        assert_eq!(result.created, -11644473600);
        assert_eq!(result.modified, -11644473600);
        assert_eq!(result.accessed, -11644473600);
        assert_eq!(result.property_guid, "46588ae2-4cbc-4338-bbfc-139326986dce");
        assert_eq!(result.environment_variable, "%windir%\\explorer.exe");
    }

    #[test]
    fn test_grab_lnk_directory() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/lnk/win11");
        let result = grab_lnk_directory(&test_location.display().to_string()).unwrap();

        assert_eq!(result.len(), 5);
    }
}
