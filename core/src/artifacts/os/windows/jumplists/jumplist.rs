use super::{automatic::parse_automatic, custom::parse_custom, error::JumplistError};
use crate::filesystem::{files::read_file, metadata::GlobInfo};
use common::windows::JumplistEntry;
use log::error;

/// Get `Jumplists` from an array of globbed paths
pub(crate) fn get_jumplists(paths: &[GlobInfo]) -> Result<Vec<JumplistEntry>, JumplistError> {
    let mut jumps = Vec::new();

    for path in paths {
        let jump_result = get_jumplist_path(&path.full_path);
        let mut jump = match jump_result {
            Ok(result) => result,
            Err(err) => {
                error!(
                    "[jumplist] Could not parse Jumplist file {}: {err:?}",
                    path.full_path
                );
                continue;
            }
        };
        jumps.append(&mut jump);
    }

    Ok(jumps)
}

/// Parse a single `Jumplist` file at provided path. Supports both Custom and Automatic
pub(crate) fn get_jumplist_path(path: &str) -> Result<Vec<JumplistEntry>, JumplistError> {
    let jump_data_result = read_file(path);
    let jump_data = match jump_data_result {
        Ok(result) => result,
        Err(err) => {
            error!("[jumplist] Could not read Jumplist file {path}: {err:?}");
            return Err(JumplistError::ReadFile);
        }
    };

    let jump = if path.ends_with(".automaticDestinations-ms") {
        let jump_result = parse_automatic(&jump_data, path);
        match jump_result {
            Ok((_, result)) => result,
            Err(_err) => {
                error!("[jumplist] Could not parse Automatic Jumplist file {path}");
                return Err(JumplistError::ParseJumplist);
            }
        }
    } else if path.ends_with(".customDestinations-ms") {
        let jump_result = parse_custom(&jump_data, path);
        match jump_result {
            Ok((_, result)) => result,
            Err(_err) => {
                error!("[jumplist] Could not parse Custom Jumplist file {path}");
                return Err(JumplistError::ParseJumplist);
            }
        }
    } else {
        return Err(JumplistError::NotJumplist);
    };

    Ok(jump)
}

#[cfg(test)]
mod tests {
    use super::get_jumplists;
    use crate::{
        artifacts::os::windows::jumplists::jumplist::get_jumplist_path,
        filesystem::metadata::glob_paths,
    };
    use common::windows::ListType;
    use std::path::PathBuf;

    #[test]
    fn test_get_jumplists() {
        let path =
            format!("C:\\Users\\*\\AppData\\Roaming\\Microsoft\\Windows\\Recent\\*Destinations\\*");

        let globs = glob_paths(&path).unwrap();
        let _ = get_jumplists(&globs).unwrap();
    }

    #[test]
    fn test_get_jumplist_path() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push(
            "tests/test_data/windows/jumplists/win10/custom/1ced32d74a95c7bc.customDestinations-ms",
        );

        let result = get_jumplist_path(&test_location.display().to_string()).unwrap();
        assert_eq!(result.len(), 8);
        assert_eq!(result[0].jumplist_type, ListType::Custom);
        assert_eq!(result[0].lnk_info.created, "1571636919");
        assert_eq!(result[0].lnk_info.modified, "1686748880");
        assert_eq!(result[0].lnk_info.accessed, "1691366002");
        assert_eq!(result[0].lnk_info.file_size, 149416368);
    }
}
