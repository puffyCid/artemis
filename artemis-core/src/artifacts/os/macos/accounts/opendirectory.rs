use crate::{
    artifacts::os::macos::plist::{
        error::PlistError,
        property_list::{get_array, get_float, parse_plist_data, parse_plist_file_dict},
    },
    utils::encoding::{base64_decode_standard, base64_encode_standard},
};
use log::{error, warn};
use plist::Value;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct OpendirectoryUsers {
    pub(crate) uid: Vec<String>,
    pub(crate) gid: Vec<String>,
    pub(crate) name: Vec<String>,
    pub(crate) real_name: Vec<String>,
    pub(crate) account_photo: Vec<String>,
    pub(crate) account_created: f64,
    pub(crate) password_last_set: f64,
    pub(crate) shell: Vec<String>,
    pub(crate) unlock_options: Vec<String>,
    pub(crate) home_path: Vec<String>,
    pub(crate) uuid: Vec<String>,
}

impl OpendirectoryUsers {
    /// Parse User plist files
    pub(crate) fn parse_users_plist(path: &str) -> Result<OpendirectoryUsers, PlistError> {
        let plist_data = parse_plist_file_dict(path)?;
        let mut users_data = OpendirectoryUsers {
            uid: Vec::new(),
            gid: Vec::new(),
            name: Vec::new(),
            real_name: Vec::new(),
            account_photo: Vec::new(),
            account_created: 0.0,
            password_last_set: 0.0,
            shell: Vec::new(),
            unlock_options: Vec::new(),
            home_path: Vec::new(),
            uuid: Vec::new(),
        };
        for (key, value) in plist_data {
            match key.as_str() {
                "shell" => {
                    let plist_results = OpendirectoryUsers::get_array_values(value);
                    users_data.shell = match plist_results {
                        Ok(results) => results,
                        Err(err) => {
                            warn!("[accounts] Failed to get user shell from opendirectoryd file: {path}: {err:?}");
                            continue;
                        }
                    };
                }
                "home" => {
                    let plist_results = OpendirectoryUsers::get_array_values(value);
                    users_data.home_path = match plist_results {
                        Ok(results) => results,
                        Err(err) => {
                            warn!("[accounts] Failed to get user home from opendirectoryd file: {path}: {err:?}");
                            continue;
                        }
                    };
                }
                "name" => {
                    let plist_results = OpendirectoryUsers::get_array_values(value);
                    users_data.name = match plist_results {
                        Ok(results) => results,
                        Err(err) => {
                            warn!("[accounts] Failed to get user name from opendirectoryd file: {path}: {err:?}");
                            continue;
                        }
                    };
                }
                "realname" => {
                    let plist_results = OpendirectoryUsers::get_array_values(value);
                    users_data.real_name = match plist_results {
                        Ok(results) => results,
                        Err(err) => {
                            warn!("[accounts] Failed to get user realname from opendirectoryd file: {path}: {err:?}");
                            continue;
                        }
                    };
                }
                "unlockOptions" => {
                    let plist_results = OpendirectoryUsers::get_array_values(value);
                    users_data.unlock_options = match plist_results {
                        Ok(results) => results,
                        Err(err) => {
                            warn!("[accounts] Failed to get user unlockOptions from opendirectoryd file: {path}: {err:?}");
                            continue;
                        }
                    };
                }
                "generateduid" => {
                    let plist_results = OpendirectoryUsers::get_array_values(value);
                    users_data.uuid = match plist_results {
                        Ok(results) => results,
                        Err(err) => {
                            warn!("[accounts] Failed to get user UUID from opendirectoryd file {path}: {err:?}");
                            continue;
                        }
                    };
                }
                "uid" => {
                    let plist_results = OpendirectoryUsers::get_array_values(value);
                    users_data.uid = match plist_results {
                        Ok(results) => results,
                        Err(err) => {
                            warn!("[accounts] Failed to get user UID from opendirectoryd file: {path}. Error: {err:?}");
                            continue;
                        }
                    };
                }
                "gid" => {
                    let plist_results = OpendirectoryUsers::get_array_values(value);
                    users_data.gid = match plist_results {
                        Ok(results) => results,
                        Err(err) => {
                            warn!("[accounts] Failed to get user GID from opendirectoryd file: {path}: {err:?}");
                            continue;
                        }
                    };
                }
                "jpegphoto" => {
                    let plist_results = OpendirectoryUsers::get_array_values(value);
                    users_data.account_photo = match plist_results {
                        Ok(results) => results,
                        Err(err) => {
                            warn!("[accounts] Failed to get user photo from opendirectoryd file: {path}: {err:?}");
                            continue;
                        }
                    };
                }
                "accountPolicyData" => {
                    let plist_results = OpendirectoryUsers::get_array_values(value);
                    match plist_results {
                        Ok(results) => {
                            for result in results {
                                let data_results = base64_decode_standard(&result);
                                let data = match data_results {
                                    Ok(result) => result,
                                    Err(_err) => {
                                        error!("[accounts] Could not decode policy data: {path}");
                                        continue;
                                    }
                                };
                                let plist_results =
                                    OpendirectoryUsers::get_account_policy(&data, &mut users_data);
                                match plist_results {
                                    Ok(_) => {}
                                    Err(err) => {
                                        warn!("accounts] Failed to get account policy info from opendirectoryd file: {path}: {err:?}");
                                        continue;
                                    }
                                }
                            }
                        }
                        Err(err) => {
                            warn!("[accounts] Failed to get user photo from opendirectoryd file: {path}: {err:?}");
                            continue;
                        }
                    }
                }
                _ => continue,
            }
        }
        Ok(users_data)
    }

    /// Loop through Array values and get the plist data
    fn get_array_values(value: Value) -> Result<Vec<String>, PlistError> {
        let array_value = get_array(&value)?;
        let mut array_results: Vec<String> = Vec::new();
        for data in array_value {
            match data {
                // All opendirectoryd data should be a string or raw bytes
                Value::String(value) => {
                    array_results.push(value);
                }
                Value::Data(value) => {
                    array_results.push(base64_encode_standard(&value));
                }
                _ => continue,
            }
        }

        Ok(array_results)
    }

    // Get account policy data from plist data
    fn get_account_policy(
        results: &[u8],
        users_data: &mut OpendirectoryUsers,
    ) -> Result<(), PlistError> {
        let account_data = parse_plist_data(results)?;
        let account_dict = account_data.as_dictionary();
        let account_info = match account_dict {
            Some(result) => result,
            _ => return Err(PlistError::Dictionary),
        };

        for (key, value) in account_info {
            match key.as_str() {
                "creationTime" => {
                    users_data.account_created = get_float(value)?;
                }

                "passwordLastSetTime" => {
                    users_data.password_last_set = get_float(value)?;
                }
                _ => continue,
            }
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct OpendirectoryGroups {
    pub(crate) gid: Vec<String>,
    pub(crate) name: Vec<String>,
    pub(crate) real_name: Vec<String>,
    pub(crate) users: Vec<String>,
    pub(crate) groupmembers: Vec<String>,
    pub(crate) uuid: Vec<String>,
}
impl OpendirectoryGroups {
    /// Parse Group plist files
    pub(crate) fn parse_groups_plist(path: &str) -> Result<OpendirectoryGroups, PlistError> {
        let plist_data = parse_plist_file_dict(path)?;
        let mut group_data = OpendirectoryGroups {
            gid: Vec::new(),
            name: Vec::new(),
            real_name: Vec::new(),
            uuid: Vec::new(),
            users: Vec::new(),
            groupmembers: Vec::new(),
        };
        for (key, value) in plist_data {
            match key.as_str() {
                "gid" => {
                    let plist_results = OpendirectoryUsers::get_array_values(value);
                    group_data.gid = match plist_results {
                        Ok(results) => results,
                        Err(err) => {
                            warn!("[accounts] Failed to get group GID from opendirectoryd file: {path}: {err:?}");
                            continue;
                        }
                    };
                }
                "name" => {
                    let plist_results = OpendirectoryUsers::get_array_values(value);
                    group_data.name = match plist_results {
                        Ok(results) => results,
                        Err(err) => {
                            warn!("[accounts] Failed to get group name from opendirectoryd file: {path}: {err:?}");
                            continue;
                        }
                    };
                }
                "realname" => {
                    let plist_results = OpendirectoryUsers::get_array_values(value);
                    group_data.real_name = match plist_results {
                        Ok(results) => results,
                        Err(err) => {
                            warn!("[accounts] Failed to get group realname from opendirectoryd file: {path}: {err:?}");
                            continue;
                        }
                    };
                }
                "generateduid" => {
                    let plist_results = OpendirectoryUsers::get_array_values(value);
                    group_data.uuid = match plist_results {
                        Ok(results) => results,
                        Err(err) => {
                            warn!("[accounts] Failed to get group UUID from opendirectoryd file: {path}: {err:?}");
                            continue;
                        }
                    };
                }
                "users" => {
                    let plist_results = OpendirectoryUsers::get_array_values(value);
                    group_data.users = match plist_results {
                        Ok(results) => results,
                        Err(err) => {
                            warn!("[accounts] Failed to get group UUID from opendirectoryd file: {path}: {err:?}");
                            continue;
                        }
                    };
                }
                "groupmembers" => {
                    let plist_results = OpendirectoryUsers::get_array_values(value);
                    group_data.groupmembers = match plist_results {
                        Ok(results) => results,
                        Err(err) => {
                            warn!("[accounts] Failed to get group UUID from opendirectoryd file: {path}: {err:?}");
                            continue;
                        }
                    };
                }
                _ => continue,
            }
        }
        Ok(group_data)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::macos::{
            accounts::opendirectory::OpendirectoryUsers,
            plist::property_list::parse_plist_file_dict,
        },
        utils::encoding::base64_decode_standard,
    };
    use plist::{Dictionary, Value};
    use std::path::PathBuf;

    #[test]
    fn test_get_array_value() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/users/nobody.plist");

        let downloads: Dictionary =
            parse_plist_file_dict(&test_location.display().to_string()).unwrap();
        let mut shell: Vec<String> = Vec::new();
        for (key, value) in downloads {
            if key != "shell" {
                continue;
            }

            if let Value::Array(_) = value {
                // Parse the array of dictionaries
                shell = OpendirectoryUsers::get_array_values(value).unwrap();
            }
        }
        assert_eq!(shell[0], "/usr/bin/false");
    }

    #[test]
    fn test_get_account_policy() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/users/nobody.plist");

        let users: Dictionary = plist::from_file(test_location.display().to_string()).unwrap();
        let mut users_data = OpendirectoryUsers {
            uid: Vec::new(),
            gid: Vec::new(),
            name: Vec::new(),
            real_name: Vec::new(),
            account_photo: Vec::new(),
            account_created: 0.0,
            password_last_set: 0.0,
            shell: Vec::new(),
            unlock_options: Vec::new(),
            home_path: Vec::new(),
            uuid: Vec::new(),
        };

        for (key, value) in users {
            if key != "accountPolicyData" {
                continue;
            }
            let plist_results = OpendirectoryUsers::get_array_values(value).unwrap();
            for result in plist_results {
                let data = base64_decode_standard(&result).unwrap();
                OpendirectoryUsers::get_account_policy(&data, &mut users_data).unwrap();
            }
        }
        assert_eq!(users_data.account_created, 1595003382.687535);
    }

    #[test]
    fn test_parse_users_plist() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/users/nobody.plist");

        let results =
            OpendirectoryUsers::parse_users_plist(&test_location.display().to_string()).unwrap();
        assert_eq!(results.shell[0], "/usr/bin/false");
        assert_eq!(results.uid[0], "-2");
        assert_eq!(results.home_path[0], "/var/empty");
        assert_eq!(results.real_name[0], "Unprivileged User");
        assert_eq!(results.account_created, 1595003382.687535);
        assert_eq!(results.account_photo.len(), 0);
        assert_eq!(results.gid[0], "-2");
        assert_eq!(results.name[0], "nobody");
        assert_eq!(results.uuid[0], "FFFFEEEE-DDDD-CCCC-BBBB-AAAAFFFFFFFE");
        assert_eq!(results.unlock_options.len(), 0);
        assert_eq!(results.password_last_set, 1595003387.901966);
    }
}
