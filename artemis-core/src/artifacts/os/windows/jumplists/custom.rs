use super::{
    destlist::{DestEntries, PinStatus},
    jumplist::JumplistEntry,
};
use crate::{
    artifacts::os::windows::{jumplists::jumplist::ListType, shortcuts::shortcut::ShortcutInfo},
    filesystem::files::get_filename,
    utils::nom_helper::{nom_unsigned_four_bytes, nom_unsigned_sixteen_bytes, Endian},
};
use nom::{
    branch::alt,
    bytes::complete::{take, take_until},
};

impl JumplistEntry {
    /// Parse Custom `Jumplist` file. It contains an array of `Shortcut` (LNK) structures
    pub(crate) fn parse_custom<'a>(
        data: &'a [u8],
        path: &str,
    ) -> nom::IResult<&'a [u8], Vec<JumplistEntry>> {
        let (input, _version) = nom_unsigned_four_bytes(data, Endian::Le)?;
        let (input, _unknown) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, _unknown) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, _unknown) = nom_unsigned_four_bytes(input, Endian::Le)?;
        // Last part of header seems to be LNK GUID?
        let (mut input, _lnk_guid) = nom_unsigned_sixteen_bytes(input, Endian::Le)?;

        let mut lists = Vec::new();

        let lnk_start = [
            76, 0, 0, 0, 1, 20, 2, 0, 0, 0, 0, 0, 192, 0, 0, 0, 0, 0, 0, 70,
        ];
        let footer = [171, 251, 191, 186];

        // Loop through all the LNK structure. Since LNK structures are variable size, we rely on two LNK headers to determine sizes
        while !input.is_empty() {
            let (lnk_input, start_lnk_data) = take_until(lnk_start.as_slice())(input)?;

            // Only start parsing if we arrived at the next LNK header or footer. Immediately after the JumpList header is the first LNK header
            if !start_lnk_data.is_empty() {
                input = lnk_input;
                continue;
            }
            let (lnk_input, header) = take(lnk_start.len())(lnk_input)?;
            // Now nom until the next header or if not found the footer. The last LNK file we nom until the Jumplist footer
            let (next_lnk_data, lnk_data) = alt((
                take_until(lnk_start.as_slice()),
                take_until(footer.as_slice()),
            ))(lnk_input)?;

            // Now take the header size and lnk_data size and nom them together
            let (_, lnk_data) = take(header.len() + lnk_data.len())(input)?;

            let (_, lnk_info) = ShortcutInfo::get_shortcut_data(lnk_data)?;

            let list = JumplistEntry {
                lnk_info,
                path: path.to_string(),
                jumplist_type: ListType::Custom,
                app_id: get_filename(path)
                    .split(".")
                    .next()
                    .unwrap_or_default()
                    .to_string(),
                jumplist_metadata: DestEntries {
                    droid_volume_id: String::new(),
                    droid_file_id: String::new(),
                    birth_droid_volume_id: String::new(),
                    birth_droid_file_id: String::new(),
                    hostname: String::new(),
                    entry: 0,
                    modified: 0,
                    pin_status: PinStatus::None,
                    path: String::new(),
                },
            };

            lists.push(list);

            // Go to next LNK structure
            input = next_lnk_data;
            // If we arrived at the JumpList footer then there is no more LNK files
            if !input.starts_with(&lnk_start) {
                break;
            }
        }

        Ok((data, lists))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::jumplists::jumplist::{JumplistEntry, ListType},
        filesystem::files::read_file,
    };
    use std::path::PathBuf;

    #[test]
    fn test_parse_custom() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push(
            "tests/test_data/windows/jumplists/win10/custom/1ced32d74a95c7bc.customDestinations-ms",
        );
        let data = read_file(&test_location.display().to_string()).unwrap();
        let (_, result) =
            JumplistEntry::parse_custom(&data, &test_location.display().to_string()).unwrap();

        assert_eq!(result.len(), 8);
        assert_eq!(result[0].jumplist_type, ListType::Custom);
        assert_eq!(result[0].lnk_info.created, 1571636919);
        assert_eq!(result[0].lnk_info.modified, 1686748880);
        assert_eq!(result[0].lnk_info.accessed, 1691366002);
        assert_eq!(result[0].lnk_info.file_size, 149416368);

        assert_eq!(
            result[0].lnk_info.path,
            "C:\\Program Files\\Microsoft VS Code\\Code.exe"
        );
        assert_eq!(result[0].lnk_info.description, "Opens a new window");
        assert_eq!(result[0].lnk_info.command_line_args, "-n");
        assert_eq!(
            result[0].lnk_info.birth_droid_file_id,
            "004c7ebf-f3c6-11e9-a0cc-0800276eb45e"
        );
        assert_eq!(
            result[0].lnk_info.property_guid,
            "f29f85e0-4ff9-1068-ab91-08002b27b3d9"
        );
        assert_eq!(result[0].lnk_info.shellitems.len(), 5);

        assert_eq!(result[7].lnk_info.created, 1571636919);
        assert_eq!(result[7].lnk_info.modified, 1686748880);
        assert_eq!(result[7].lnk_info.accessed, 1691366002);
        assert_eq!(result[7].lnk_info.file_size, 149416368);

        assert_eq!(
            result[7].lnk_info.path,
            "C:\\Program Files\\Microsoft VS Code\\Code.exe"
        );
        assert_eq!(
            result[7].lnk_info.description,
            "C:\\Users\\bob\\Projects\\Rust\\artemis-core\\artemis-core.code-workspace"
        );
        assert_eq!(result[7].lnk_info.command_line_args, "--file-uri \"file:///c%3A/Users/bob/Projects/Rust/artemis-core/artemis-core.code-workspace\"");
        assert_eq!(
            result[7].lnk_info.birth_droid_file_id,
            "004c7ebf-f3c6-11e9-a0cc-0800276eb45e"
        );
        assert_eq!(
            result[7].lnk_info.property_guid,
            "f29f85e0-4ff9-1068-ab91-08002b27b3d9"
        );
        assert_eq!(result[7].lnk_info.shellitems.len(), 5);
        assert_eq!(result[7].lnk_info.drive_serial, "D49D126F");
    }
}
