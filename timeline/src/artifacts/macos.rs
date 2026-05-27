use super::files::extract_times;
use crate::artifacts::filter::filter_data;
use serde_json::{Map, Value, json};

/// Timeline macOS Users
pub(crate) fn users_macos(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
    if !data.is_object() {
        return false;
    }
    let Some(account_created) = data["account_created"].as_str() else {
        return false;
    };

    if filter_data(account_created, start, end) {
        return false;
    }

    data["datetime"] = account_created.into();
    data["message"] = data["name"]
        .as_array()
        .unwrap_or(&Vec::new())
        .first()
        .unwrap_or(&String::from("Unknown username").into())
        .clone();
    data["artifact"] = Value::String(String::from("macOS User"));
    data["data_type"] = Value::String(String::from("macos:plist:users:entry"));
    data["timestamp_desc"] = Value::String(String::from("User Account Created"));

    true
}

/// Timeline macOS groups
pub(crate) fn groups_macos(data: &mut Value) -> bool {
    data["datetime"] = Value::String(String::from("1970-01-01T00:00:00.000Z"));
    data["message"] = data["name"]
        .as_array()
        .unwrap_or(&Vec::new())
        .first()
        .unwrap_or(&String::from("Unknown group name").into())
        .clone();
    data["artifact"] = "macOS Group".into();
    data["data_type"] = "macos:plist:groups:entry".into();
    data["timestamp_desc"] = "N/A".into();

    true
}

/// Timeline macOS emond
pub(crate) fn emond(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
    if !data.is_object() {
        return false;
    }
    let Some(plist_created) = data["plist_created"].as_str() else {
        return false;
    };

    if filter_data(plist_created, start, end) {
        return false;
    }
    data["datetime"] = plist_created.into();
    data["message"] = data["name"].as_str().unwrap_or_default().into();
    data["artifact"] = Value::String(String::from("Emond"));
    data["data_type"] = Value::String(String::from("macos:plist:emond:entry"));
    data["timestamp_desc"] = Value::String(String::from("PLIST Created"));

    true
}

/// Timeline macOS `ExecPolicy`
pub(crate) fn execpolicy(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
    if !data.is_object() {
        return false;
    }
    let Some(exec_time) = data["executable_timestamp"].as_str() else {
        return false;
    };

    if filter_data(exec_time, start, end) {
        return false;
    }
    data["datetime"] = exec_time.into();
    data["message"] = data["file_identifier"].as_str().unwrap_or_default().into();
    data["artifact"] = "ExecPolicy".into();
    data["data_type"] = "macos:sqlite:execpolicy:entry".into();
    data["timestamp_desc"] = "Executable Timestamp".into();

    true
}

pub(crate) fn fsevents(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
    if !data.is_object() {
        return false;
    }
    let Some(evidence_created) = data["evidence_created"].as_str() else {
        return false;
    };

    if filter_data(evidence_created, start, end) {
        return false;
    }
    data["datetime"] = evidence_created.into();
    data["message"] = data["path"].as_str().unwrap_or_default().into();
    data["artifact"] = Value::String(String::from("FsEvents"));
    data["data_type"] = Value::String(String::from("macos:fsevents:entry"));
    data["timestamp_desc"] = Value::String(String::from("Evidence File Created"));

    true
}

pub(crate) fn launchd(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
    if !data.is_object() {
        return false;
    }
    let Some(created) = data["created"].as_str() else {
        return false;
    };

    if filter_data(created, start, end) {
        return false;
    }

    data["datetime"] = created.into();
    data["message"] = data["evidence"].as_str().unwrap_or_default().into();
    data["artifact"] = Value::String(String::from("Launch Daemon"));
    data["data_type"] = Value::String(String::from("macos:plist:launchd:entry"));
    data["timestamp_desc"] = Value::String(String::from("Launch Daemon Created"));

    true
}

pub(crate) fn loginitems(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
    if !data.is_object() {
        return false;
    }
    let Some(created) = data["created"].as_str() else {
        return false;
    };

    if filter_data(created, start, end) {
        return false;
    }
    data["datetime"] = created.into();
    data["message"] = data["path"].as_str().unwrap_or_default().into();
    data["artifact"] = Value::String(String::from("LoginItems"));
    data["data_type"] = Value::String(String::from("macos:plist:loginitems:entry"));
    data["timestamp_desc"] = Value::String(String::from("Target Created"));

    if data["message"].as_str().unwrap_or_default().is_empty() {
        data["message"] = data["app_id"].as_str().unwrap_or_default().into();
    }

    true
}

pub(crate) fn spotlight(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
    if !data.is_object() {
        return false;
    }

    data["artifact"] = Value::String(String::from("Spotlight"));
    data["data_type"] = Value::String(String::from("macos:spotlight:entry"));
    // Default value
    data["datetime"] = Value::String(String::from("1970-01-01T00:00:00.000Z"));
    // Default value
    data["timestamp_desc"] = Value::String(String::from("N/A"));

    // Deal with nested json metadata
    let temp = match data["values"].as_object() {
        Some(result) => result.clone(),
        None => Map::new(),
    };

    // Entry is always an object since we check above
    data.as_object_mut().unwrap().remove("values");

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

        data[key] = prop_value.clone();

        if key.contains("kMDItemDisplayName") {
            data["message"] = prop_value;
        } else if key.contains("kMDItemDateAdded") {
            // This should always be string. But if not then continue loop
            if !prop_value.is_string() {
                continue;
            }
            // unwrap is safe since we check for string type
            if filter_data(prop_value.as_str().unwrap(), start, end) {
                return false;
            }
            data["datetime"] = prop_value;
            data["timestamp_desc"] = Value::String(String::from("Item Added"));
        }
    }

    true
}

pub(crate) fn unifiedlogs(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
    if !data.is_object() {
        return false;
    }
    let Some(timestamp) = data["timestamp"].as_str() else {
        return false;
    };

    if filter_data(timestamp, start, end) {
        return false;
    }
    data["datetime"] = timestamp.into();
    data["artifact"] = Value::String(String::from("Unified Logs"));
    data["data_type"] = Value::String(String::from("macos:unifiedlog:entry"));
    data["timestamp_desc"] = Value::String(String::from("Entry Generated"));

    data["log_timestamp"] = data["timestamp"].as_str().unwrap().into();
    // Timestamp is reserved word by Timesketch
    data.as_object_mut().unwrap().remove("timestamp");
    // Always an object since we check above
    data.as_object_mut().unwrap().remove("message_entries");
    data.as_object_mut().unwrap().remove("raw_message");

    true
}

pub(crate) fn sudo_macos(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
    if !data.is_object() {
        return false;
    }
    let Some(timestamp) = data["timestamp"].as_str() else {
        return false;
    };

    if filter_data(timestamp, start, end) {
        return false;
    }
    data["datetime"] = timestamp.into();
    data["artifact"] = Value::String(String::from("Sudo macOS"));
    data["data_type"] = Value::String(String::from("macos:unifiedlog:sudo:entry"));
    data["timestamp_desc"] = Value::String(String::from("Entry Generated"));

    // Always an object since we check above
    data.as_object_mut().unwrap().remove("message_entries");
    data.as_object_mut().unwrap().remove("raw_message");

    true
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
        let mut test = json!({
            "account_created": "2024-01-01T00:00:00.000Z",
            "name": ["bob"],
        });

        assert!(users_macos(&mut test, &None, &None));
        assert_eq!(test["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test["artifact"], "macOS User");
        assert_eq!(test["message"], "bob");
    }

    #[test]
    fn test_groups_macos() {
        let mut test = json!({
            "name": ["bob"],
        });

        assert!(groups_macos(&mut test));
        assert_eq!(test["artifact"], "macOS Group");
        assert_eq!(test["message"], "bob");
    }

    #[test]
    fn test_emond() {
        let mut test = json!({
            "plist_created": "2024-01-01T00:00:00.000Z",
            "name": "bob rule",
        });

        assert!(emond(&mut test, &None, &None));
        assert_eq!(test["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test["artifact"], "Emond");
        assert_eq!(test["message"], "bob rule");
    }

    #[test]
    fn test_execpolicy() {
        let mut test = json!({
            "executable_timestamp": "2024-01-01T00:00:00.000Z",
            "file_identifier": "git",
            "executable_measurements_v2_timestamp": "2024-02-01T00:00:00.000Z",
        });

        assert!(execpolicy(&mut test, &None, &None));
        assert_eq!(test["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test["artifact"], "ExecPolicy");
        assert_eq!(test["message"], "git");
    }

    #[test]
    fn test_fsevents() {
        let mut test = json!({
            "evidence_created": "2024-01-01T00:00:00.000Z",
            "path": "git",
        });

        assert!(fsevents(&mut test, &None, &None));
        assert_eq!(test["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test["artifact"], "FsEvents");
        assert_eq!(test["message"], "git");
    }

    #[test]
    fn test_launchd() {
        let mut test = json!({
            "created": "2024-01-01T00:00:00.000Z",
            "modified": "2024-02-01T00:00:00.000Z",
            "changed": "2024-03-01T00:00:00.000Z",
            "accessed": "2024-04-01T00:00:00.000Z",
            "evidence": "/Library/LaunchDaemons/com.googlecode.munki.logouthelper.plist",
        });

        assert!(launchd(&mut test, &None, &None));
        assert_eq!(test["artifact"], "Launch Daemon");
        assert_eq!(
            test["message"],
            "/Library/LaunchDaemons/com.googlecode.munki.logouthelper.plist"
        );
    }

    #[test]
    fn test_loginitems() {
        let mut test = json!({
            "created": "2024-01-01T00:00:00.000Z",
            "path": "/Applications/Docker.app",
        });

        assert!(loginitems(&mut test, &None, &None));
        assert_eq!(test["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test["artifact"], "LoginItems");
        assert_eq!(test["message"], "/Applications/Docker.app");

        let mut missing_path = json!({
            "created": "2024-01-01T00:00:00.000Z",
            "path": "",
            "app_id": "docker"
        });

        assert!(loginitems(&mut missing_path, &None, &None));
        assert_eq!(missing_path["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(missing_path["artifact"], "LoginItems");
        assert_eq!(missing_path["message"], "docker");
    }

    #[test]
    fn test_spotlight() {
        let mut test = json!({
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
        });

        assert!(spotlight(&mut test, &None, &None));
        assert_eq!(test["datetime"], "2022-08-14T00:00:00.000Z");
        assert_eq!(test["artifact"], "Spotlight");
        assert_eq!(test["message"], "proxy_delta.rb");
    }

    #[test]
    fn test_unifiedlogs() {
        let mut test = json!({
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
        });

        assert!(unifiedlogs(&mut test, &None, &None));
        assert_eq!(test["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test["artifact"], "Unified Logs");
        assert_eq!(
            test["message"],
            "ANE0: newUserClient :H11ANEIn::newUserClient type=1\n"
        );
    }
}
