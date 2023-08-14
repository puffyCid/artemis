use super::{
    destlist::parse_destlist,
    jumplist::{JumplistEntry, ListType},
};
use crate::{
    artifacts::os::windows::{
        jumplists::destlist::{DestList, DestVersion},
        ole::olecf::{DirectoryType, OleData},
        shortcuts::shortcut::ShortcutInfo,
    },
    filesystem::files::get_filename,
};
use log::error;
use nom::error::ErrorKind;

impl JumplistEntry {
    /// Parse Automatic `Jumplists`
    pub(crate) fn parse_automatic<'a>(
        data: &'a [u8],
        path: &str,
    ) -> nom::IResult<&'a [u8], Vec<JumplistEntry>> {
        let (_, jump_ole) = OleData::parse_ole(data)?;

        let mut dest_info = DestList {
            version: DestVersion::Unknown,
            number_entries: 0,
            _number_pinned_entries: 0,
            _last_entry: 0,
            _last_revision: 0,
            entries: Vec::new(),
        };
        for entry in jump_ole.iter() {
            if entry.name != "DestList" || entry.directory_type == DirectoryType::Root {
                continue;
            }

            // Parse DestList directory to get metadata about the Shortcut (LNK) info
            let info_result = parse_destlist(&entry.data);
            dest_info = match info_result {
                Ok((_, result)) => result,
                Err(_err) => {
                    error!("[jumplist] Could not parse Automatic DestList info!");
                    return Err(nom::Err::Failure(nom::error::Error::new(
                        &[],
                        ErrorKind::Fail,
                    )));
                }
            }
        }

        let mut jump_entries = Vec::new();

        // Now have everything needed to parse Automatic Jumplists!
        for entry in jump_ole {
            for info in dest_info.entries.iter() {
                // Need to compare hex values
                if entry.name != format!("{:x?}", info.entry) {
                    continue;
                }

                let lnk_result = ShortcutInfo::get_shortcut_data(&entry.data);
                let lnk_info = match lnk_result {
                    Ok((_, result)) => result,
                    Err(_err) => {
                        error!("[jumplist] Could not parse Shortcut info in Automatic Jumplist!");
                        continue;
                    }
                };

                let jump = JumplistEntry {
                    lnk_info,
                    path: path.to_string(),
                    jumplist_type: ListType::Automatic,
                    app_id: get_filename(path)
                        .split('.')
                        .next()
                        .unwrap_or_default()
                        .to_string(),
                    jumplist_metadata: info.clone(),
                };
                jump_entries.push(jump);
            }
        }
        Ok((data, jump_entries))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::jumplists::jumplist::JumplistEntry, filesystem::files::read_file,
    };
    use std::path::PathBuf;

    #[test]
    fn test_parse_automatic() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push(
            "tests/test_data/dfir/windows/jumplists/win7/1b4dd67f29cb1962.automaticDestinations-ms",
        );
        let data = read_file(&test_location.display().to_string()).unwrap();

        let (_, result) =
            JumplistEntry::parse_automatic(&data, &test_location.display().to_string()).unwrap();

        assert_eq!(result.len(), 4);
        assert_eq!(result[3].app_id, "1b4dd67f29cb1962");
        assert_eq!(result[3].lnk_info.created, 1452975745);
        assert_eq!(result[3].lnk_info.drive_serial, "88008C2F");
        assert_eq!(
            result[3].jumplist_metadata.path,
            "::{031E4825-7B94-4DC3-B131-E946B44C8DD5}\\Videos.library-ms"
        );
    }

    #[test]
    fn test_parse_automatic_large() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push(
            "tests/test_data/windows/jumplists/win11/automatic/3d2110c4a0cb6d15.automaticDestinations-ms",
        );
        let data = read_file(&test_location.display().to_string()).unwrap();

        let (_, result) =
            JumplistEntry::parse_automatic(&data, &test_location.display().to_string()).unwrap();

        assert_eq!(result.len(), 41);
        assert_eq!(result[3].app_id, "3d2110c4a0cb6d15");
        assert_eq!(result[3].lnk_info.created, 1668879141);
        assert_eq!(result[3].lnk_info.drive_serial, "4290933E");
        assert_eq!(
            result[3].jumplist_metadata.path,
            "C:\\Users\\bob\\Projects\\artemis-core\\tests\\test_data\\browser\\firefoxwin.toml"
        );
    }
}
