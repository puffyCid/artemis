use super::files::extract_times;
use crate::artifacts::filter::filter_data;
use serde_json::{Map, Value, json};

/// Timeline macOS Users
pub(crate) fn users_macos(data: &mut Value) -> Option<()> {
    data.as_array_mut()?.retain_mut(|entry| {
        if !entry.is_object() {
            // Drop value if its not an object
            return false;
        }
        let created = match entry["account_created"].as_str() {
            Some(result) => result,
            None => return false,
        };
        entry["datetime"] = created.into();
        entry["message"] = entry["name"]
            .as_array()
            .unwrap_or(&Vec::new())
            .first()
            .unwrap_or(&Value::Null)
            .clone();
        entry["artifact"] = Value::String(String::from("macOS User"));
        entry["data_type"] = Value::String(String::from("macos:plist:users:entry"));
        entry["timestamp_desc"] = Value::String(String::from("User Account Created"));
        !filter_data(entry["datetime"].as_str().unwrap(), None, None)
    });

    Some(())
}

/// Timeline macOS groups
pub(crate) fn groups_macos(data: &mut Value) -> Option<()> {
    data.as_array_mut()?.retain_mut(|entry| {
        if !entry.is_object() {
            // Drop value if its not an object
            return false;
        }
        entry["datetime"] = Value::String(String::from("1970-01-01T00:00:00.000Z"));
        entry["message"] = entry["name"]
            .as_array()
            .unwrap_or(&Vec::new())
            .first()
            .unwrap_or(&Value::Null)
            .clone();
        entry["artifact"] = Value::String(String::from("macOS Group"));
        entry["data_type"] = Value::String(String::from("macos:plist:groups:entry"));
        entry["timestamp_desc"] = Value::String(String::from("N/A"));
        !filter_data(entry["datetime"].as_str().unwrap(), None, None)
    });

    Some(())
}

/// Timeline macOS emond
pub(crate) fn emond(data: &mut Value) -> Option<()> {
    data.as_array_mut()?.retain_mut(|entry| {
        if !entry.is_object() {
            // Drop value if its not an object
            return false;
        }
        let created = match entry["plist_created"].as_str() {
            Some(result) => result,
            None => return false,
        };
        entry["datetime"] = created.into();
        entry["message"] = entry["name"].as_str().unwrap_or_default().into();
        entry["artifact"] = Value::String(String::from("Emond"));
        entry["data_type"] = Value::String(String::from("macos:plist:emond:entry"));
        entry["timestamp_desc"] = Value::String(String::from("PLIST Created"));
        !filter_data(entry["datetime"].as_str().unwrap(), None, None)
    });

    Some(())
}

/// Timeline macOS `ExecPolicy`
pub(crate) fn execpolicy(data: &mut Value) -> Option<()> {
    data.as_array_mut()?.retain_mut(|entry| {
        if !entry.is_object() {
            // Drop value if its not an object
            return false;
        }

        let exec_time = match entry["executable_timestamp"].as_str() {
            Some(result) => result,
            None => return false,
        };
        entry["datetime"] = exec_time.into();
        entry["message"] = entry["file_identifier"].as_str().unwrap_or_default().into();
        entry["artifact"] = Value::String(String::from("ExecPolicy"));
        entry["data_type"] = Value::String(String::from("macos:sqlite:execpolicy:entry"));
        entry["timestamp_desc"] = Value::String(String::from("Executable Timestamp"));
        !filter_data(entry["datetime"].as_str().unwrap(), None, None)
    });

    Some(())
}

pub(crate) fn fsevents(data: &mut Value) -> Option<()> {
    data.as_array_mut()?.retain_mut(|entry| {
        if !entry.is_object() {
            // Drop value if its not an object
            return false;
        }
        let created = match entry["evidence_created"].as_str() {
            Some(result) => result,
            None => return false,
        };
        entry["datetime"] = created.into();
        entry["message"] = entry["path"].as_str().unwrap_or_default().into();
        entry["artifact"] = Value::String(String::from("FsEvents"));
        entry["data_type"] = Value::String(String::from("macos:fsevents:entry"));
        entry["timestamp_desc"] = Value::String(String::from("Evidence File Created"));
        !filter_data(entry["datetime"].as_str().unwrap(), None, None)
    });

    Some(())
}

pub(crate) fn launchd(data: &mut Value) -> Option<()> {
    let mut entries = Vec::new();

    for entry in data.as_array_mut()? {
        entry["message"] = entry["evidence"].as_str()?.into();
        entry["artifact"] = Value::String(String::from("Launch Daemon"));
        entry["data_type"] = Value::String(String::from("macos:plist:launchd:entry"));

        let temp = json![{
            "created": entry["created"].as_str()?,
            "modified": entry["modified"].as_str()?,
            "accessed": entry["accessed"].as_str()?,
            "changed": entry["changed"].as_str()?,
        }];
        let times = extract_times(&temp)?;

        for (key, value) in times {
            if filter_data(key, None, None) {
                continue;
            }
            entry["datetime"] = Value::String(key.into());
            entry["timestamp_desc"] = Value::String(value);
            entries.push(entry.clone());
        }
    }
    *data.as_array_mut()? = entries;
    Some(())
}

pub(crate) fn loginitems(data: &mut Value) -> Option<()> {
    data.as_array_mut()?.retain_mut(|entry| {
        if !entry.is_object() {
            // Drop value if its not an object
            return false;
        }
        let created = match entry["created"].as_str() {
            Some(result) => result,
            None => return false,
        };

        entry["datetime"] = created.into();
        entry["message"] = entry["path"].as_str().unwrap_or_default().into();
        entry["artifact"] = Value::String(String::from("LoginItems"));
        entry["data_type"] = Value::String(String::from("macos:plist:loginitems:entry"));
        entry["timestamp_desc"] = Value::String(String::from("Target Created"));

        if entry["message"].as_str().unwrap_or_default().is_empty() {
            entry["message"] = entry["app_id"].as_str().unwrap_or_default().into();
        }
        !filter_data(entry["datetime"].as_str().unwrap(), None, None)
    });

    Some(())
}

pub(crate) fn spotlight(data: &mut Value) -> Option<()> {
    data.as_array_mut()?.retain_mut(|entry| {
        if !entry.is_object() {
            // Drop value if its not an object
            return false;
        }
        entry["artifact"] = Value::String(String::from("Spotlight"));
        entry["data_type"] = Value::String(String::from("macos:spotlight:entry"));
        // Default value
        entry["datetime"] = Value::String(String::from("1970-01-01T00:00:00.000Z"));
        // Default value
        entry["timestamp_desc"] = Value::String(String::from("N/A"));

        // Deal with nested json metadata
        let temp = match entry["values"].as_object() {
            Some(result) => result.clone(),
            None => Map::new(),
        };

        // Entry is always an object since we check above
        entry.as_object_mut().unwrap().remove("values");

        for (key, value) in &temp {
            // Most properties have only one entry
            let prop_value = if value["value"].is_array()
                && value["value"].as_array().unwrap_or(&Vec::new()).len() == 1
            {
                // unwrap is safe since we check above
                value["value"].as_array().unwrap()[0].clone()
            } else {
                value["value"].clone()
            };

            entry[key] = prop_value.clone();

            if key.contains("kMDItemDisplayName") {
                entry["message"] = prop_value;
            } else if key.contains("kMDItemDateAdded") {
                entry["datetime"] = prop_value;
                entry["timestamp_desc"] = Value::String(String::from("Item Added"));
            }
        }
        !filter_data(entry["datetime"].as_str().unwrap(), None, None)
    });

    Some(())
}

pub(crate) fn unifiedlogs(data: &mut Value) -> Option<()> {
    data.as_array_mut()?.retain_mut(|entry| {
        if !entry.is_object() {
            // Drop value if its not an object
            return false;
        }
        let timestamp = match entry["timestamp"].as_str() {
            Some(result) => result,
            None => return false,
        };
        entry["datetime"] = timestamp.into();
        entry["artifact"] = Value::String(String::from("Unified Logs"));
        entry["data_type"] = Value::String(String::from("macos:unifiedlog:entry"));
        entry["timestamp_desc"] = Value::String(String::from("Entry Generated"));

        entry["log_timestamp"] = entry["timestamp"].as_str().unwrap().into();
        // Timestamp is reserved word by Timesketch
        entry.as_object_mut().unwrap().remove("timestamp");
        // Always an object since we check above
        entry.as_object_mut().unwrap().remove("message_entries");
        entry.as_object_mut().unwrap().remove("raw_message");

        !filter_data(entry["datetime"].as_str().unwrap(), None, None)
    });

    Some(())
}

pub(crate) fn sudo_macos(data: &mut Value) -> Option<()> {
    data.as_array_mut()?.retain_mut(|entry| {
        if !entry.is_object() {
            // Drop value if its not an object
            return false;
        }
        let timestamp = match entry["timestamp"].as_str() {
            Some(result) => result,
            None => return false,
        };
        entry["datetime"] = timestamp.into();
        entry["artifact"] = Value::String(String::from("Sudo macOS"));
        entry["data_type"] = Value::String(String::from("macos:unifiedlog:sudo:entry"));
        entry["timestamp_desc"] = Value::String(String::from("Entry Generated"));

        // Always an object since we check above
        entry.as_object_mut().unwrap().remove("message_entries");
        entry.as_object_mut().unwrap().remove("raw_message");

        !filter_data(entry["datetime"].as_str().unwrap(), None, None)
    });

    Some(())
}

#[cfg(test)]
mod tests {
    use crate::artifacts::macos::{
        emond, execpolicy, fsevents, groups_macos, launchd, loginitems, spotlight, unifiedlogs,
        users_macos,
    };
    use serde_json::json;

    #[test]
    fn test_users_macos() {
        let mut test = json!([{
            "account_created": "2024-01-01T00:00:00.000Z",
            "name": ["bob"],
        }]);

        users_macos(&mut test).unwrap();
        assert_eq!(test[0]["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test[0]["artifact"], "macOS User");
        assert_eq!(test[0]["message"], "bob");
    }

    #[test]
    fn test_groups_macos() {
        let mut test = json!([{
            "name": ["bob"],
        }]);

        groups_macos(&mut test).unwrap();
        assert_eq!(test[0]["artifact"], "macOS Group");
        assert_eq!(test[0]["message"], "bob");
    }

    #[test]
    fn test_emond() {
        let mut test = json!([{
            "plist_created": "2024-01-01T00:00:00.000Z",
            "name": "bob rule",
        }]);

        emond(&mut test).unwrap();
        assert_eq!(test[0]["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test[0]["artifact"], "Emond");
        assert_eq!(test[0]["message"], "bob rule");
    }

    #[test]
    fn test_execpolicy() {
        let mut test = json!([{
            "executable_timestamp": "2024-01-01T00:00:00.000Z",
            "file_identifier": "git",
            "executable_measurements_v2_timestamp": "2024-02-01T00:00:00.000Z",
        }]);

        execpolicy(&mut test).unwrap();
        assert_eq!(test[0]["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test[0]["artifact"], "ExecPolicy");
        assert_eq!(test[0]["message"], "git");
    }

    #[test]
    fn test_fsevents() {
        let mut test = json!([{
            "evidence_created": "2024-01-01T00:00:00.000Z",
            "path": "git",
        }]);

        fsevents(&mut test).unwrap();
        assert_eq!(test[0]["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test[0]["artifact"], "FsEvents");
        assert_eq!(test[0]["message"], "git");
    }

    #[test]
    fn test_launchd() {
        let mut test = json!([{
            "created": "2024-01-01T00:00:00.000Z",
            "modified": "2024-02-01T00:00:00.000Z",
            "changed": "2024-03-01T00:00:00.000Z",
            "accessed": "2024-04-01T00:00:00.000Z",
            "evidence": "/Library/LaunchDaemons/com.googlecode.munki.logouthelper.plist",
        }]);

        launchd(&mut test).unwrap();
        assert_eq!(test[0]["artifact"], "Launch Daemon");
        assert_eq!(
            test[0]["message"],
            "/Library/LaunchDaemons/com.googlecode.munki.logouthelper.plist"
        );
    }

    #[test]
    fn test_loginitems() {
        let mut test = json!([{
            "created": "2024-01-01T00:00:00.000Z",
            "path": "/Applications/Docker.app",
        }]);

        loginitems(&mut test).unwrap();
        assert_eq!(test[0]["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test[0]["artifact"], "LoginItems");
        assert_eq!(test[0]["message"], "/Applications/Docker.app");

        let mut missing_path = json!([{
            "created": "2024-01-01T00:00:00.000Z",
            "path": "",
            "app_id": "docker"
        }]);

        loginitems(&mut missing_path).unwrap();
        assert_eq!(missing_path[0]["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(missing_path[0]["artifact"], "LoginItems");
        assert_eq!(missing_path[0]["message"], "docker");
    }

    #[test]
    fn test_spotlight() {
        let mut test = json!([{
            "directory": "/System/Volumes/Data/.Spotlight-V100/Store-V2/1037649B-DB77-4E4E-8265-0ECC829B4813/store.db",
            "values": {
                "kMDItemDisplayName": {
                    "attribute": "AttrString",
                    "value": "proxy_delta.rb",
                },
                "kMDItemDateAdded_Ranking": {
                    "attribute": "AttrDate",
                    "value": [
                        "2022-08-14T00:00:00.000Z"
                    ]
                },
            }
        }]);

        spotlight(&mut test).unwrap();
        assert_eq!(test[0]["datetime"], "2022-08-14T00:00:00.000Z");
        assert_eq!(test[0]["artifact"], "Spotlight");
        assert_eq!(test[0]["message"], "proxy_delta.rb");
    }

    #[test]
    fn test_unifiedlogs() {
        let mut test = json!([{
            "timestamp": "2024-01-01T00:00:00.000Z",
            "message": "ANE0: newUserClient :H11ANEIn::newUserClient type=1\n",
            "raw_message": "ANE%d: %s :H11ANEIn::newUserClient type=%u\n",
            "message_entries": [
                {
                    "message_strings": "0",
                    "item_type": 2,
                    "item_size": 0
                },
                {
                    "message_strings": "newUserClient",
                    "item_type": 34,
                    "item_size": 14
                },
                {
                    "message_strings": "1",
                    "item_type": 2,
                    "item_size": 0
                }
            ],
        }]);

        unifiedlogs(&mut test).unwrap();
        assert_eq!(test[0]["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test[0]["artifact"], "Unified Logs");
        assert_eq!(
            test[0]["message"],
            "ANE0: newUserClient :H11ANEIn::newUserClient type=1\n"
        );
    }
}
