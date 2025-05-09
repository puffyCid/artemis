use super::{error::UserAssistError, registry::UserAssistReg};
use crate::utils::{
    encoding::base64_decode_standard,
    environment::get_folder_descriptions,
    nom_helper::{Endian, nom_unsigned_eight_bytes, nom_unsigned_four_bytes},
    time::{filetime_to_unixepoch, unixepoch_to_iso},
};
use common::windows::UserAssistEntry;
use log::{error, warn};
use nom::bytes::complete::take;
use std::collections::HashMap;

/// Parse the `UserAssist` data obtained from the Registry
pub(crate) fn parse_userassist_data(
    reg_entry: &[UserAssistReg],
    resolve: &bool,
) -> Result<Vec<UserAssistEntry>, UserAssistError> {
    let mut userassist_entries: Vec<UserAssistEntry> = Vec::new();
    let folder_result = if *resolve {
        get_folder_descriptions()
    } else {
        Ok(HashMap::new())
    };

    let descriptions = match folder_result {
        Ok(result) => result,
        Err(err) => {
            warn!(
                "[userassist] Could not get folder descriptions cannot do CLSID lookups: {err:?}"
            );
            HashMap::new()
        }
    };
    for entry in reg_entry {
        get_entries(entry, &mut userassist_entries, &descriptions);
    }
    Ok(userassist_entries)
}

/// Go through all `UserAssist` entries
fn get_entries(
    reg_entries: &UserAssistReg,
    userassist_entries: &mut Vec<UserAssistEntry>,
    folder_descriptions: &HashMap<String, String>,
) {
    for entry in &reg_entries.regs {
        // Loop through UserAssist values
        for value in &entry.values {
            // UserAssist is in a binary format so we need to base64 decode the value string to get the binary data
            let decoded_result = base64_decode_standard(&value.data);
            let assist_data = match decoded_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[userassist] Could not base64 decode data: {err:?}");
                    continue;
                }
            };

            let assist_result = get_userassist_data(&assist_data);
            let (_, mut userassist) = match assist_result {
                Ok(result) => result,
                Err(_err) => {
                    error!("[userassist] Could not parse userassist data");
                    continue;
                }
            };
            userassist.rot_path.clone_from(&value.value);
            userassist.path = rot_decode(&value.value);
            userassist.reg_path.clone_from(&reg_entries.reg_file);

            // Check if we can translate the CLSID values to the folder name
            for (key, value) in folder_descriptions {
                // If there are any case sensitive CLSID entries we will not find them
                // Currently not lowercasing the path and folder description entries
                if !userassist.path.contains(key) {
                    continue;
                }
                userassist.folder_path = userassist.path.replace(key, value);
            }
            userassist_entries.push(userassist);
        }
    }
}

/// Parse out the `UserAssist` data: Execution count and last execution time
fn get_userassist_data(data: &[u8]) -> nom::IResult<&[u8], UserAssistEntry> {
    let mut userassist = UserAssistEntry {
        path: String::new(),
        last_execution: String::new(),
        count: 0,
        reg_path: String::new(),
        rot_path: String::new(),
        folder_path: String::new(),
    };
    let entry_size = 72;
    if data.len() != entry_size {
        return Ok((data, userassist));
    }
    let (input, _unknown) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (input, count) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let unknown_data_size: u8 = 52;
    let (input, _unknown2) = take(unknown_data_size)(input)?;
    let (input, last_execution) = nom_unsigned_eight_bytes(input, Endian::Le)?;

    userassist.count = count;
    userassist.last_execution = unixepoch_to_iso(&filetime_to_unixepoch(&last_execution));

    Ok((input, userassist))
}

/// The `UserAssist` executable path is ROT13 encoded.
/// It is possible to disable the encoding via a Registry setting.
fn rot_decode(rot: &str) -> String {
    let rot_shift = 13;
    rot.chars()
        .map(|c| match c {
            'a'..='m' | 'A'..='M' => ((c as u8) + rot_shift) as char,
            'n'..='z' | 'N'..='Z' => ((c as u8) - rot_shift) as char,
            _ => c,
        })
        .collect()
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use crate::{
        artifacts::os::windows::userassist::{
            assist::{get_entries, get_userassist_data, parse_userassist_data, rot_decode},
            registry::get_userassist_drive,
        },
        utils::{encoding::base64_decode_standard, environment::get_folder_descriptions},
    };

    #[test]
    fn test_parse_userassist_data() {
        let results = get_userassist_drive(&'C').unwrap();
        let results = parse_userassist_data(&results, &false).unwrap();
        assert!(results.len() > 3);
        for entry in results {
            if entry.reg_path == "UEME_CTLSESSION" {
                assert_eq!(entry.count, 0);
                assert_eq!(entry.last_execution, "");
                assert_eq!(entry.rot_path, "HRZR_PGYFRFFVBA");
            }
        }
    }

    #[test]
    fn test_get_entries() {
        let results = get_userassist_drive(&'C').unwrap();
        let mut entries = Vec::new();
        let folder = get_folder_descriptions().unwrap();

        for entry in results {
            get_entries(&entry, &mut entries, &folder);
        }
        assert!(entries.len() > 3);
    }

    #[test]
    fn test_get_userassist_data() {
        let results = get_userassist_drive(&'C').unwrap();
        assert!(results.len() > 0);
        for reg_entries in results {
            for entry in &reg_entries.regs {
                for value in &entry.values {
                    let assist_data = base64_decode_standard(&value.data).unwrap();

                    let (_, _userassist) = get_userassist_data(&assist_data).unwrap();
                }
            }
        }
    }

    #[test]
    fn test_rot_decode() {
        let test_input = "Ehfg vf cerggl pbby nppbeqvat gb Sreevf";
        let result = rot_decode(test_input);
        assert_eq!(result, "Rust is pretty cool according to Ferris");
    }
}
