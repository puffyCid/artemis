use super::{files::extract_times, meta::check_meta};
use serde_json::Value;

/// Timeline macOS Users
pub(crate) fn users_macos(data: &mut Value) -> Option<()> {
    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };
        entry["datetime"] = entry["account_created"].as_str()?.into();
        entry["message"] = entry["name"]
            .as_array()?
            .first()
            .unwrap_or(&Value::Null)
            .clone();
        entry["artifact"] = Value::String(String::from("macOS User"));
        entry["data_type"] = Value::String(String::from("macos:plist:users:entry"));
        entry["timestamp_desc"] = Value::String(String::from("User Account Created"));
    }

    Some(())
}

/// Timeline macOS groups
pub(crate) fn groups_macos(data: &mut Value) -> Option<()> {
    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };
        entry["datetime"] = Value::String(String::from("1970-01-01T00:00:00.000Z"));
        entry["message"] = entry["name"]
            .as_array()?
            .first()
            .unwrap_or(&Value::Null)
            .clone();
        entry["artifact"] = Value::String(String::from("macOS Group"));
        entry["data_type"] = Value::String(String::from("macos:plist:groups:entry"));
        entry["timestamp_desc"] = Value::String(String::from("N/A"));
    }

    Some(())
}

/// Timeline macOS emond
pub(crate) fn emond(data: &mut Value) -> Option<()> {
    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };
        entry["datetime"] = entry["plist_created"].as_str()?.into();
        entry["message"] = entry["name"].as_str()?.into();
        entry["artifact"] = Value::String(String::from("Emond"));
        entry["data_type"] = Value::String(String::from("macos:plist:emond:entry"));
        entry["timestamp_desc"] = Value::String(String::from("PLIST Created"));
    }

    Some(())
}

/// Timeline macOS `ExecPolicy`
pub(crate) fn execpolicy(data: &mut Value) -> Option<()> {
    let mut entries = Vec::new();

    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };
        entry["datetime"] = entry["executable_timestamp"].as_str()?.into();
        entry["message"] = entry["file_identifier"].as_str()?.into();
        entry["artifact"] = Value::String(String::from("ExecPolicy"));
        entry["data_type"] = Value::String(String::from("macos:sqlite:execpolicy:entry"));
        entry["timestamp_desc"] = Value::String(String::from("Executable Timestamp"));

        entries.push(entry.clone());

        entry["datetime"] = entry["executable_measurements_v2_timestamp"]
            .as_str()?
            .into();
        entry["timestamp_desc"] = Value::String(String::from("Executable Measurements"));
        entries.push(entry.clone());
    }

    check_meta(data, &mut entries)
}

pub(crate) fn fsevents(data: &mut Value) -> Option<()> {
    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };
        entry["datetime"] = entry["source_created"].as_str()?.into();
        entry["message"] = entry["path"].as_str()?.into();
        entry["artifact"] = Value::String(String::from("FsEvents"));
        entry["data_type"] = Value::String(String::from("macos:fsevents:entry"));
        entry["timestamp_desc"] = Value::String(String::from("Source File Created"));
    }

    Some(())
}

pub(crate) fn launchd(data: &mut Value) -> Option<()> {
    let mut entries = Vec::new();

    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };
        entry["message"] = entry["plist_path"].as_str()?.into();
        entry["artifact"] = Value::String(String::from("Launch Daemon"));
        entry["data_type"] = Value::String(String::from("macos:plist:launchd:entry"));

        let temp = entry.clone();
        let times = extract_times(&temp)?;

        for (key, value) in times {
            entry["datetime"] = Value::String(key.into());
            entry["timestamp_desc"] = Value::String(value);
            entries.push(entry.clone());
        }
    }

    check_meta(data, &mut entries)
}

pub(crate) fn loginitems(data: &mut Value) -> Option<()> {
    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };
        entry["datetime"] = entry["created"].as_str()?.into();
        entry["message"] = entry["path"].as_str()?.into();
        entry["artifact"] = Value::String(String::from("LoginItems"));
        entry["data_type"] = Value::String(String::from("macos:plist:loginitems:entry"));
        entry["timestamp_desc"] = Value::String(String::from("Target Created"));
    }

    Some(())
}

pub(crate) fn spotlight(data: &mut Value) -> Option<()> {
    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };
        entry["artifact"] = Value::String(String::from("Spotlight"));
        entry["data_type"] = Value::String(String::from("macos:spotlight:entry"));

        let temp = entry["values"].as_object()?.clone();
        entry.as_object_mut()?.remove("values")?;

        for (key, value) in &temp {
            // Most properties have only one entry
            let prop_value = if value["value"].is_array() && value["value"].as_array()?.len() == 1 {
                value["value"].as_array()?[0].clone()
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
    }

    Some(())
}

pub(crate) fn unifiedlogs(data: &mut Value) -> Option<()> {
    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };
        entry["artifact"] = Value::String(String::from("Unified Logs"));
        entry["data_type"] = Value::String(String::from("macos:unifiedlog:entry"));
        entry["datetime"] = entry["timestamp"].as_str()?.into();
        entry["timestamp_desc"] = Value::String(String::from("Entry Generated"));

        entry.as_object_mut()?.remove("message_entries")?;
        entry.as_object_mut()?.remove("raw_message")?;
    }

    Some(())
}

pub(crate) fn sudo_macos(data: &mut Value) -> Option<()> {
    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };
        entry["artifact"] = Value::String(String::from("Sudo macOS"));
        entry["data_type"] = Value::String(String::from("macos:unifiedlog:sudo:entry"));
        entry["datetime"] = entry["timestamp"].as_str()?.into();
        entry["timestamp_desc"] = Value::String(String::from("Entry Generated"));

        entry.as_object_mut()?.remove("message_entries")?;
        entry.as_object_mut()?.remove("raw_message")?;
    }

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
            "source_created": "2024-01-01T00:00:00.000Z",
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
            "plist_path": "/Library/LaunchDaemons/com.googlecode.munki.logouthelper.plist",
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
