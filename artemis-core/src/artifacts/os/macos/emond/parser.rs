/**
 * macOS `Emond` (Event Monitor) can be used as persistence on a system  
 * A user can create `Emond` rules to execute commands on macOS  
 *
 * Starting on Ventura Emond was removed
 *
 * References:  
 *   `https://www.xorrior.com/emond-persistence/`
 */
use super::eventmonitor::EmondData;
use crate::{
    artifacts::os::macos::plist::{
        error::PlistError,
        property_list::{get_string, parse_plist_file_dict},
    },
    filesystem::files::is_file,
};
use log::{error, warn};
use plist::Value;

/// Parse Emond rules on macOS
pub(crate) fn grab_emond() -> Result<Vec<EmondData>, PlistError> {
    let paths = get_emond_rules_paths()?;
    let mut emond_data: Vec<EmondData> = Vec::new();
    for path in paths {
        let mut data = EmondData::parse_emond_rules(&path)?;
        emond_data.append(&mut data);
    }
    Ok(emond_data)
}

/// Parse the Emond Config PLIST to get any additional Emond Rules directories besides the default path
fn get_emond_rules_paths() -> Result<Vec<String>, PlistError> {
    let emond_plist_path = "/etc/emond.d/emond.plist";
    if !is_file(emond_plist_path) {
        warn!("[emond] No emond.plist file found. Emond removed starting on macOS Ventura");
        return Ok(Vec::new());
    }

    let emond_plist_result = parse_plist_file_dict(&emond_plist_path);
    let emond_plist = match emond_plist_result {
        Ok(results) => results,
        Err(err) => {
            error!("[emond] Failed to parse Emond Config PLIST file: {err:?}");
            return Ok(Vec::new());
        }
    };

    let mut emond_rules_paths: Vec<String> = Vec::new();
    let default_path = String::from("/etc/emond.d/rules");
    emond_rules_paths.push(default_path);

    for (key, value) in emond_plist {
        if key != "config" {
            continue;
        }
        // Parse the config dictionary and get all the additional paths at additionalRulesPaths
        let value_dictionary = match value {
            Value::Dictionary(value_dictionary) => value_dictionary,
            _ => continue,
        };
        for (subkey, subvalue) in value_dictionary {
            if subkey != "additionalRulesPaths" {
                continue;
            }

            // Additional paths are stored as an array. Loop and get all the paths (if any)
            let value_array = match subvalue {
                Value::Array(value_array) => value_array,
                _ => continue,
            };

            for additional_path in value_array {
                let path_string = get_string(&additional_path)?;
                emond_rules_paths.push(path_string);
            }
        }
    }
    Ok(emond_rules_paths)
}

#[cfg(test)]
mod tests {
    use super::{get_emond_rules_paths, grab_emond};

    #[test]
    fn test_get_emond_rules_paths() {
        let _ = get_emond_rules_paths().unwrap();
    }

    #[test]
    fn test_grab_emond() {
        let _ = grab_emond().unwrap();
    }
}
