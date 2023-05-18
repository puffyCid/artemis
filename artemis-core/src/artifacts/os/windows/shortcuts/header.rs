use crate::{
    filesystem::ntfs::attributes::{file_attribute_flags, AttributeFlags},
    utils::{
        nom_helper::{
            nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_two_bytes, Endian,
        },
        time::filetime_to_unixepoch,
        uuid::format_guid_le_bytes,
    },
};
use nom::bytes::complete::take;
use serde::Serialize;
use std::mem::size_of;

#[derive(Debug)]
pub(crate) struct LnkHeader {
    /**Should always be 0x4c (76) */
    _size: u32,
    /**Should be 00021401-0000-0000-c000-000000000046 */
    _class_id: String,
    pub(crate) data_flags: Vec<DataFlags>,
    pub(crate) attribute_flags: Vec<AttributeFlags>,
    pub(crate) created: i64,
    pub(crate) access: i64,
    pub(crate) modified: i64,
    pub(crate) file_size: u32,
    _icon_index: u32,
    _window_value: u32,
    _hot_key: u16,
    _unknown: u16,
    _unknown2: u32,
    _unknown3: u32,
}

#[derive(Debug, PartialEq, Serialize)]
pub(crate) enum DataFlags {
    HasTargetIdList,
    HasLinkInfo,
    HasName,
    HasRelativePath,
    HasWorkingDirectory,
    HasArguements,
    HasIconLocation,
    IsUnicode,
    ForceNoLinkInfo,
    HasExpString,
    RunInSeparateProcess,
    HasDarwinId,
    RunAsUser,
    HasExpIcon,
    NoPidAlias,
    RunWithShimLayer,
    ForceNoLinkTrack,
    EnableTargetMetadata,
    DisableLinkPathTracking,
    DisableKnownFolderTracking,
    DisableKnownFolderAlias,
    AllowLinkToLink,
    UnaliasOnSave,
    PreferEnvironmentPath,
    KeepLocalDListForUncTarget,
}

impl LnkHeader {
    /// Parse the `Shortcut` file header. Contains target file size and target file created, modified, accessed timestamps
    pub(crate) fn parse_header(data: &[u8]) -> nom::IResult<&[u8], LnkHeader> {
        let (input, size) = nom_unsigned_four_bytes(data, Endian::Le)?;
        let (input, guid_data) = take(size_of::<u128>())(input)?;
        let (input, data_flags) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, attribute_flags) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let (input, created_filetime) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, access_filetime) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, modified_filetime) = nom_unsigned_eight_bytes(input, Endian::Le)?;

        let (input, file_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, icon_index) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, window_value) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, hot_key) = nom_unsigned_two_bytes(input, Endian::Le)?;

        let (input, unknown) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, unknown2) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, unknown3) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let class_id = format_guid_le_bytes(guid_data);
        let header = LnkHeader {
            _size: size,
            _class_id: class_id,
            data_flags: LnkHeader::get_flags(&data_flags),
            attribute_flags: file_attribute_flags(&attribute_flags),
            created: filetime_to_unixepoch(&created_filetime),
            access: filetime_to_unixepoch(&access_filetime),
            modified: filetime_to_unixepoch(&modified_filetime),
            file_size,
            _icon_index: icon_index,
            _window_value: window_value,
            _hot_key: hot_key,
            _unknown: unknown,
            _unknown2: unknown2,
            _unknown3: unknown3,
        };

        Ok((input, header))
    }

    /// Get data flags from `Shortcut` header control if other structures are available
    fn get_flags(flags: &u32) -> Vec<DataFlags> {
        let mut lnk_flags: Vec<DataFlags> = Vec::new();

        let has_target_id_list = 0x1;
        let has_link_info = 0x2;
        let has_name = 0x4;
        let has_relative_path = 0x8;
        let has_working_directory = 0x10;
        let has_arguements = 0x20;
        let has_icon_location = 0x40;
        let is_unicode = 0x80;
        let force_no_link_info = 0x100;
        let has_exp_string = 0x200;
        let run_in_separate_process = 0x400;
        let has_darwin_id = 0x1000;
        let run_as_user = 0x2000;
        let has_exp_icon = 0x4000;
        let no_pid_alias = 0x8000;
        let run_with_shim_layer = 0x20000;
        let force_no_link_track = 0x40000;
        let enable_target_metadata = 0x80000;
        let disable_link_path_tracking = 0x100000;
        let disable_known_folder_tracking = 0x200000;
        let disable_known_folder_alias = 0x400000;
        let allow_link_to_link = 0x800000;
        let unalias_on_save = 0x1000000;
        let prefer_environment_path = 0x2000000;
        let keep_local_d_list_for_unc_target = 0x4000000;

        // A shortcut file may have multiple flags
        if (flags & has_target_id_list) == has_target_id_list {
            lnk_flags.push(DataFlags::HasTargetIdList);
        }
        if (flags & has_link_info) == has_link_info {
            lnk_flags.push(DataFlags::HasLinkInfo);
        }
        if (flags & has_name) == has_name {
            lnk_flags.push(DataFlags::HasName);
        }
        if (flags & has_relative_path) == has_relative_path {
            lnk_flags.push(DataFlags::HasRelativePath);
        }
        if (flags & has_working_directory) == has_working_directory {
            lnk_flags.push(DataFlags::HasWorkingDirectory);
        }
        if (flags & has_arguements) == has_arguements {
            lnk_flags.push(DataFlags::HasArguements);
        }
        if (flags & has_icon_location) == has_icon_location {
            lnk_flags.push(DataFlags::HasIconLocation);
        }
        if (flags & is_unicode) == is_unicode {
            lnk_flags.push(DataFlags::IsUnicode);
        }
        if (flags & force_no_link_info) == force_no_link_info {
            lnk_flags.push(DataFlags::ForceNoLinkInfo);
        }
        if (flags & has_exp_string) == has_exp_string {
            lnk_flags.push(DataFlags::HasExpString);
        }
        if (flags & run_in_separate_process) == run_in_separate_process {
            lnk_flags.push(DataFlags::RunInSeparateProcess);
        }
        if (flags & has_darwin_id) == has_darwin_id {
            lnk_flags.push(DataFlags::HasDarwinId);
        }
        if (flags & run_as_user) == run_as_user {
            lnk_flags.push(DataFlags::RunAsUser);
        }
        if (flags & has_exp_icon) == has_exp_icon {
            lnk_flags.push(DataFlags::HasExpIcon);
        }
        if (flags & no_pid_alias) == no_pid_alias {
            lnk_flags.push(DataFlags::NoPidAlias);
        }
        if (flags & run_with_shim_layer) == run_with_shim_layer {
            lnk_flags.push(DataFlags::RunWithShimLayer);
        }
        if (flags & force_no_link_track) == force_no_link_track {
            lnk_flags.push(DataFlags::ForceNoLinkTrack);
        }
        if (flags & enable_target_metadata) == enable_target_metadata {
            lnk_flags.push(DataFlags::EnableTargetMetadata);
        }

        if (flags & disable_link_path_tracking) == disable_link_path_tracking {
            lnk_flags.push(DataFlags::DisableLinkPathTracking);
        }
        if (flags & disable_known_folder_tracking) == disable_known_folder_tracking {
            lnk_flags.push(DataFlags::DisableKnownFolderTracking);
        }
        if (flags & disable_known_folder_alias) == disable_known_folder_alias {
            lnk_flags.push(DataFlags::DisableKnownFolderAlias);
        }
        if (flags & allow_link_to_link) == allow_link_to_link {
            lnk_flags.push(DataFlags::AllowLinkToLink);
        }
        if (flags & unalias_on_save) == unalias_on_save {
            lnk_flags.push(DataFlags::UnaliasOnSave);
        }
        if (flags & prefer_environment_path) == prefer_environment_path {
            lnk_flags.push(DataFlags::PreferEnvironmentPath);
        }
        if (flags & keep_local_d_list_for_unc_target) == keep_local_d_list_for_unc_target {
            lnk_flags.push(DataFlags::KeepLocalDListForUncTarget);
        }

        lnk_flags
    }

    /// Verify if provided bytes contain `shortcut` data
    pub(crate) fn check_header(data: &[u8]) -> nom::IResult<&[u8], bool> {
        let (input, size) = nom_unsigned_four_bytes(data, Endian::Le)?;
        let (_, guid_data) = take(size_of::<u128>())(input)?;

        let class_id = format_guid_le_bytes(guid_data);

        let header_size = 76;
        let header_id = "00021401-0000-0000-c000-000000000046";
        if size == header_size && class_id == header_id {
            return Ok((data, true));
        }
        Ok((data, false))
    }
}

#[cfg(test)]
mod tests {
    use super::DataFlags;
    use crate::artifacts::os::windows::shortcuts::header::LnkHeader;
    use crate::filesystem::ntfs::attributes::AttributeFlags;

    #[test]
    fn test_parser_header() {
        let test = [
            76, 0, 0, 0, 1, 20, 2, 0, 0, 0, 0, 0, 192, 0, 0, 0, 0, 0, 0, 70, 139, 0, 32, 0, 16, 0,
            0, 0, 159, 38, 31, 30, 26, 246, 216, 1, 133, 5, 25, 151, 28, 27, 217, 1, 40, 54, 5,
            151, 28, 27, 217, 1, 0, 192, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ];

        let (_, result) = LnkHeader::parse_header(&test).unwrap();
        assert_eq!(result._size, 76);
        assert_eq!(result._class_id, "00021401-0000-0000-c000-000000000046");
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
        assert_eq!(result.created, 1668204504);
        assert_eq!(result.access, 1672273759);
        assert_eq!(result.modified, 1672273759);
        assert_eq!(result.file_size, 49152);
        assert_eq!(result._icon_index, 0);
        assert_eq!(result._window_value, 1);
        assert_eq!(result._hot_key, 0);
        assert_eq!(result._unknown, 0);
        assert_eq!(result._unknown2, 0);
        assert_eq!(result._unknown3, 0);
    }

    #[test]
    fn test_get_flags() {
        let test = 1;
        let result = LnkHeader::get_flags(&test);
        assert_eq!(result[0], DataFlags::HasTargetIdList)
    }

    #[test]
    fn test_check_header() {
        let test = [
            76, 0, 0, 0, 1, 20, 2, 0, 0, 0, 0, 0, 192, 0, 0, 0, 0, 0, 0, 70, 139, 0, 32, 0, 16, 0,
            0, 0, 159, 38, 31, 30, 26, 246, 216, 1, 133, 5, 25, 151, 28, 27, 217, 1, 40, 54, 5,
            151, 28, 27, 217, 1, 0, 192, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ];

        let (_, result) = LnkHeader::check_header(&test).unwrap();
        assert_eq!(result, true);
    }
}
