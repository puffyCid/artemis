use super::files::{extract_filename_times, extract_times};
use crate::artifacts::filter::filter_data;
use serde_json::{Map, Value, json};
use std::collections::HashMap;

/// Timeline Windows Users
pub(crate) fn users(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
    if !data.is_object() {
        return false;
    }
    let Some(last_logon) = data["last_logon"].as_str() else {
        return false;
    };

    if filter_data(last_logon, start, end) {
        return false;
    }
    data["datetime"] = last_logon.into();
    data["message"] = data["username"].as_str().unwrap_or_default().into();
    data["artifact"] = Value::String(String::from("Windows User"));
    data["data_type"] = Value::String(String::from("windows:registry:users:entry"));
    data["timestamp_desc"] = Value::String(String::from("User Last Logon"));

    true
}

/// Timeline Amcache
pub(crate) fn amcache(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
    if !data.is_object() {
        return false;
    }
    let Some(last_modified) = data["last_modified"].as_str() else {
        return false;
    };

    if filter_data(last_modified, start, end) {
        return false;
    }

    data["datetime"] = last_modified.into();
    data["message"] = data["path"].as_str().unwrap_or_default().into();
    data["artifact"] = Value::String(String::from("Amcache"));
    data["data_type"] = Value::String(String::from("windows:registry:amcache:entry"));
    data["timestamp_desc"] = Value::String(String::from("Amcache Registry Last Modified"));

    true
}

/// Timeline Windows BITS
pub(crate) fn bits(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
    if !data.is_object() {
        return false;
    }
    let mut entries = Vec::new();
    data["artifact"] = Value::String(String::from("BITS"));
    data["data_type"] = Value::String(String::from("windows:ese:bits:entry"));
    data["message"] = Value::String(format!(
        "Job: {} - Target Path: {}",
        data["job_name"]
            .as_str()
            .unwrap_or(&String::from("Unknown job")),
        data["target_path"]
            .as_str()
            .unwrap_or(&String::from("Unknown target"))
    ));

    let temp = data.clone();
    let times = extract_bits_times(&temp).unwrap_or_default();
    for (key, value) in times {
        if filter_data(key, start, end) {
            continue;
        }
        data["datetime"] = Value::String(key.into());
        data["timestamp_desc"] = Value::String(value);
        entries.push(data.clone());
    }

    if entries.is_empty() {
        return false;
    }
    *data = Value::Array(entries);
    true
}

/// Extract all BITS timestamps into separate timestamps
fn extract_bits_times(data: &Value) -> Option<HashMap<&str, String>> {
    let mut times = HashMap::new();
    times.insert(data["created"].as_str()?, String::from("BITS Created"));

    if let Some(value) = times.get(data["modified"].as_str()?) {
        times.insert(data["modified"].as_str()?, format!("{value} BITS Modified"));
    } else {
        times.insert(data["modified"].as_str()?, String::from("BITS Modified"));
    }

    if let Some(value) = times.get(data["expiration"].as_str()?) {
        times.insert(
            data["expiration"].as_str()?,
            format!("{value} BITS Expired"),
        );
    } else {
        times.insert(data["expiration"].as_str()?, String::from("BITS Expired"));
    }

    if let Some(value) = times.get(data["completed"].as_str()?) {
        times.insert(
            data["completed"].as_str()?,
            format!("{value} BITS Completed"),
        );
    } else {
        times.insert(data["completed"].as_str()?, String::from("BITS Completed"));
    }

    Some(times)
}

/// Timeline Eventlogs. Only Eventlog entries with template strings are supported
pub(crate) fn eventlogs(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
    if !data.is_object() || data["template_message"] == Value::Null {
        return false;
    }
    let Some(generated) = data["generated"].as_str() else {
        return false;
    };

    if filter_data(generated, start, end) {
        return false;
    }
    data["datetime"] = generated.into();

    if data["message"].is_string() && data["message"].as_str().unwrap_or_default().is_empty() {
        data["message"] = Value::String(data["raw_event_data"].to_string());
    }

    // Timesketch cannot handle large amounts of raw event data
    // It maxes out at 1000 total JSON keys per sketch
    // Unwrap safe since we check to make we have an object above
    data.as_object_mut().unwrap().remove("raw_event_data");
    data.as_object_mut().unwrap().remove("template_message");

    data["artifact"] = Value::String(String::from("EventLogs"));
    data["data_type"] = Value::String(String::from("windows:eventlogs:entry"));
    data["timestamp_desc"] = Value::String(String::from("EventLog Entry Generated"));

    true
}

pub(crate) fn jumplists(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
    if !data.is_object() {
        return false;
    }
    let mut entries = Vec::new();
    data["artifact"] = Value::String(String::from("Jumplist"));
    data["data_type"] = Value::String(String::from("windows:jumplist:entry"));

    let temp = data.clone();
    data.as_object_mut().unwrap().remove("lnk_info");
    // Flatten lnk_info
    for (key, value) in temp["lnk_info"].as_object().unwrap_or(&Map::new()) {
        if key == "path" {
            data["target_path"] = value.clone();
            data["message"] = value.clone();
            continue;
        }
        data[key] = value.clone();
    }

    // Flatten jumplist_metadata
    for (key, value) in temp["jumplist_metadata"].as_object().unwrap_or(&Map::new()) {
        if key == "path" {
            data["jumplist_target"] = value.clone();
            if data["message"] == Value::String(String::new()) {
                data["message"] = value.clone();
            }
            continue;
        } else if key == "modified" {
            data["entry_modified"] = value.clone();
            continue;
        }
        data[key] = value.clone();
    }
    data.as_object_mut().unwrap().remove("jumplist_metadata");

    if data["message"] == Value::String(String::new()) {
        data["message"] = data["path"].clone();
    }

    let times = extract_shortcut_times(&temp["lnk_info"]).unwrap_or_default();

    for (key, value) in times {
        if filter_data(key, start, end) {
            continue;
        }
        data["datetime"] = Value::String(key.into());
        data["timestamp_desc"] = Value::String(value);
        entries.push(data.clone());
    }
    if entries.is_empty() {
        return false;
    }

    *data = Value::Array(entries);
    true
}

fn extract_shortcut_times(data: &Value) -> Option<HashMap<&str, String>> {
    let mut times = HashMap::new();
    times.insert(
        data["created"].as_str()?,
        String::from("Shortcut Target Created"),
    );

    if let Some(value) = times.get(data["modified"].as_str()?) {
        times.insert(
            data["modified"].as_str()?,
            format!("{value} Shortcut Target Modified"),
        );
    } else {
        times.insert(
            data["modified"].as_str()?,
            String::from("Shortcut Target Modified"),
        );
    }

    if let Some(value) = times.get(data["accessed"].as_str()?) {
        times.insert(
            data["accessed"].as_str()?,
            format!("{value} Shortcut Target Accessed"),
        );
    } else {
        times.insert(
            data["accessed"].as_str()?,
            String::from("Shortcut Target Accessed"),
        );
    }

    Some(times)
}

pub(crate) fn raw_files(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
    if !data.is_object() {
        return false;
    }
    let mut entries = Vec::new();
    data["artifact"] = Value::String(String::from("RawFiles"));
    data["data_type"] = Value::String(String::from("windows:ntfs:file"));
    data["message"] = Value::String(data["full_path"].as_str().unwrap_or_default().into());
    let temp = json![{
        "created": &data["created"].as_str().unwrap_or_default(),
        "modified": data["modified"].as_str().unwrap_or_default(),
        "accessed": data["accessed"].as_str().unwrap_or_default(),
        "changed": data["changed"].as_str().unwrap_or_default(),
        "filename_created": data["filename_created"].as_str().unwrap_or_default(),
        "filename_modified": data["filename_modified"].as_str().unwrap_or_default(),
        "filename_accessed": data["filename_accessed"].as_str().unwrap_or_default(),
        "filename_changed": data["filename_changed"].as_str().unwrap_or_default(),
    }];

    let mut times = extract_times(&temp).unwrap_or_default();
    extract_filename_times(&temp, &mut times).unwrap_or_default();
    for (key, value) in times {
        // If $INDX recovery is enabled. Standard Info timestamps will be empty
        // We will only have FileName timestamps
        // Skip emtpy Standard Info timestamps
        if key.is_empty() || filter_data(key, start, end) {
            continue;
        }

        data["datetime"] = Value::String(key.into());
        data["timestamp_desc"] = Value::String(value);
        entries.push(data.clone());
    }

    *data = Value::Array(entries);
    true
}

pub(crate) fn outlook(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
    if !data.is_object() {
        return false;
    }
    let Some(delivered) = data["delivered"].as_str() else {
        return false;
    };

    if filter_data(delivered, start, end) {
        return false;
    }
    data["datetime"] = delivered.into();
    data["message"] = Value::String(format!(
        "Subject: {} From: {}",
        data["subject"].as_str().unwrap_or_default(),
        data["from"].as_str().unwrap_or_default()
    ));
    data["artifact"] = Value::String(String::from("Outlook"));
    data["data_type"] = Value::String(String::from("windows:outlook:email"));
    data["timestamp_desc"] = Value::String(String::from("Email Delivered"));

    true
}

pub(crate) fn prefetch(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
    if !data.is_object() {
        return false;
    }
    let mut entries = Vec::new();
    data["artifact"] = Value::String(String::from("Prefetch"));
    data["data_type"] = Value::String(String::from("windows:prefetch:file"));
    data["message"] = Value::String(data["evidence"].as_str().unwrap_or_default().into());
    data["datetime"] = data["last_run_time"].as_str().unwrap_or_default().into();
    data["timestamp_desc"] = Value::String(String::from("Prefetch Last Execution"));
    if !filter_data(
        data["last_run_time"].as_str().unwrap_or_default(),
        start,
        end,
    ) {
        entries.push(data.clone());
    }
    let mut temp = data.clone();

    for value in data["all_run_times"].as_array().unwrap_or(&Vec::new()) {
        if filter_data(value.as_str().unwrap_or_default(), start, end) {
            continue;
        }
        temp["datetime"] = value.as_str().unwrap_or_default().into();
        temp["timestamp_desc"] = Value::String(String::from("Prefetch Execution"));
        entries.push(temp.clone());
    }

    if entries.is_empty() {
        return false;
    }
    *data = Value::Array(entries);
    true
}

pub(crate) fn recycle_bin(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
    if !data.is_object() {
        return false;
    }
    let Some(deleted) = data["deleted"].as_str() else {
        return false;
    };

    if filter_data(deleted, start, end) {
        return false;
    }

    data["datetime"] = deleted.into();
    data["message"] = data["full_path"].as_str().unwrap_or_default().into();
    data["artifact"] = Value::String(String::from("RecycleBin"));
    data["data_type"] = Value::String(String::from("windows:recyclebin:entry"));
    data["timestamp_desc"] = Value::String(String::from("File Deleted"));

    true
}

pub(crate) fn search(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
    if !data.is_object() {
        return false;
    }
    let Some(last_modified) = data["last_modified"].as_str() else {
        return false;
    };

    if filter_data(last_modified, start, end) {
        return false;
    }
    data["datetime"] = last_modified.into();
    data["message"] = data["entry"].as_str().unwrap_or_default().into();
    data["artifact"] = Value::String(String::from("Search"));
    data["data_type"] = Value::String(String::from("windows:ese:search:entry"));
    data["timestamp_desc"] = Value::String(String::from("Entry Last Modified"));
    let temp = data["properties"]
        .as_object()
        .unwrap_or(&Map::new())
        .clone();
    data.as_object_mut().unwrap().remove("properties");
    for (key, value) in &temp {
        data[key] = value.clone();

        if data["message"].as_str().unwrap_or_default().is_empty()
            && key.contains("System_ItemPathDisplay")
        {
            data["message"] = value.as_str().unwrap_or_default().into();
        }
    }

    true
}

pub(crate) fn services(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
    if !data.is_object() {
        return false;
    }
    let Some(modified) = data["modified"].as_str() else {
        return false;
    };

    if filter_data(modified, start, end) {
        return false;
    }

    data["datetime"] = modified.into();
    data["message"] = Value::String(format!(
        "Service Name: {} | {}",
        data["name"].as_str().unwrap_or_default(),
        data["path"].as_str().unwrap_or_default(),
    ));
    data["artifact"] = Value::String(String::from("Service"));
    data["data_type"] = Value::String(String::from("windows:registry:services:entry"));
    data["timestamp_desc"] = Value::String(String::from("Registry Last Modified"));

    true
}

pub(crate) fn shellbags(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
    if !data.is_object() {
        return false;
    }
    let Some(reg_modified) = data["reg_modified"].as_str() else {
        return false;
    };

    if filter_data(reg_modified, start, end) {
        return false;
    }

    data["datetime"] = reg_modified.into();
    data["message"] = data["path"].as_str().unwrap_or_default().into();
    data["artifact"] = Value::String(String::from("Shellbags"));
    data["data_type"] = Value::String(String::from("windows:registry:shellbags:entry"));
    data["timestamp_desc"] = Value::String(String::from("Registry Last Modified"));

    true
}

pub(crate) fn shimcache(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
    if !data.is_object() {
        return false;
    }
    let Some(last_modified) = data["last_modified"].as_str() else {
        return false;
    };

    if filter_data(last_modified, start, end) {
        return false;
    }
    data["datetime"] = last_modified.into();
    data["message"] = data["path"].as_str().unwrap_or_default().into();
    data["artifact"] = Value::String(String::from("Shimcache"));
    data["data_type"] = Value::String(String::from("windows:registry:shimcache:entry"));
    data["timestamp_desc"] = Value::String(String::from("Shimcache Last Modified"));

    true
}

pub(crate) fn registry(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
    if !data.is_object() {
        return false;
    }
    let Some(last_modified) = data["last_modified"].as_str() else {
        return false;
    };

    if filter_data(last_modified, start, end) {
        return false;
    }

    data["datetime"] = last_modified.into();
    data["artifact"] = Value::String(String::from("Registry"));
    data["data_type"] = Value::String(String::from("windows:registry:entry"));
    data["timestamp_desc"] = Value::String(String::from("Registry Last Modified"));
    let mut entries = Vec::new();

    let temp = data.clone();
    data.as_object_mut().unwrap().remove("values");

    for value in temp["values"].as_array().unwrap_or(&Vec::new()) {
        data["message"] = Value::String(format!(
            "{} | Value: {}",
            data["path"].as_str().unwrap_or_default(),
            value["value"].as_str().unwrap_or_default()
        ));
        data["value"] = value["value"].clone();
        data["data"] = value["data"].clone();
        data["reg_data_type"] = value["data_type"].clone();
        entries.push(data.clone());
    }

    if temp["values"].as_array().is_some_and(|v| v.is_empty()) {
        data["message"] = data["path"].clone();
        entries.push(data.clone());
    }

    *data = Value::Array(entries);
    true
}

pub(crate) fn shimdb(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
    if !data.is_object() {
        return false;
    }

    // If we include Indexes, memory usage will explode (~2GB). Indexes primarily contain base64 binary data. There's nothing parsable in it
    // This likely only affects sysmain.sdb due to the large number of Shims. We could split this into separate entries if needed
    // Custom Shims would likely be unaffected
    data.as_object_mut().unwrap().remove("indexes");
    let mut entries = Vec::new();

    data["artifact"] = Value::String(String::from("Shimdb"));
    data["data_type"] = Value::String(String::from("windows:shimdb:entry"));
    let Some(db_data) = data["db_data"].as_object() else {
        return false;
    };
    data["datetime"] = db_data["compile_time"].as_str().unwrap_or_default().into();
    if filter_data(data["datetime"].as_str().unwrap(), start, end) {
        return false;
    }
    data["timestamp_desc"] = Value::String(String::from("Shim Compile Time"));

    let temp = data.clone();
    data.as_object_mut().unwrap().remove("db_data");
    data["message"] = Value::String(format!(
        "{} | Shim: None",
        temp["evidence"].as_str().unwrap_or_default()
    ));

    // Flatten db_data
    for (key, value) in temp["db_data"].as_object().unwrap_or(&Map::new()) {
        if key == "list_data" {
            // If we parsed the sysmain.sdb file. This will be a ton of data
            for tag in value.as_array().unwrap_or(&Vec::new()) {
                let mut shim = data.clone();
                if let Some(value) = tag["data"].as_object() {
                    for (data_key, data_value) in value {
                        shim[data_key.clone()] = data_value.clone();
                        if data_key == "TAG_NAME" || data_key == "TAG_MODULE" {
                            shim["message"] = Value::String(format!(
                                "{} | Shim {data_key}: {}",
                                temp["evidence"].as_str().unwrap_or_default(),
                                data_value.as_str().unwrap_or_default(),
                            ));
                        }
                    }
                }

                if let Some(value) = tag["list_data"].as_array() {
                    for list_entry in value {
                        if let Some(list_value) = list_entry.as_object() {
                            for (list_key, list_data) in list_value {
                                if let Some(existing_data) =
                                    shim.get_mut(format!("list_{list_key}"))
                                {
                                    if existing_data.is_string() {
                                        shim[format!("list_{list_key}")] = Value::Array(vec![
                                            existing_data.clone(),
                                            list_data.clone(),
                                        ]);
                                        continue;
                                    }
                                    let _ = existing_data.is_array().then(|| {
                                        existing_data
                                            .as_array_mut()
                                            .unwrap_or(&mut Vec::new())
                                            .push(list_data.clone());
                                    });

                                    continue;
                                }
                                shim[format!("list_{list_key}")] = list_data.clone();
                            }
                        }
                    }
                }
                entries.push(shim.clone());
            }
        }
    }

    if entries.is_empty() {
        return false;
    }

    *data = Value::Array(entries);
    true
}

pub(crate) fn shortcuts(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
    if !data.is_object() {
        return false;
    }
    data["artifact"] = Value::String(String::from("Shortcut"));
    data["data_type"] = Value::String(String::from("windows:shortcut:lnk"));
    data["message"] = data["evidence"].clone();
    let mut entries = Vec::new();

    let temp = data.clone();
    let times = extract_shortcut_times(&temp).unwrap_or_default();

    for (key, value) in times {
        if filter_data(key, start, end) {
            continue;
        }
        data["datetime"] = Value::String(key.into());
        data["timestamp_desc"] = Value::String(value);
        entries.push(data.clone());
    }

    if entries.is_empty() {
        return false;
    }

    *data = Value::Array(entries);
    true
}

pub(crate) fn srum(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
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
    data["message"] = data["app_id"].as_str().unwrap_or_default().into();
    data["srum_timestamp"] = data["timestamp"].as_str().unwrap().into();
    // Timestamp is reserved word by Timesketch
    data.as_object_mut().unwrap().remove("timestamp");
    data["timestamp_desc"] = Value::String(String::from("SRUM Table Update"));
    if !data["facetime"].is_null() {
        data["artifact"] = Value::String(String::from("SRUM Application Info"));
        data["data_type"] = Value::String(String::from("windows:ese:srum:application_info:entry"));
    } else if !data["cycles_web"].is_null() {
        data["artifact"] = Value::String(String::from("SRUM Application Timeline"));
        data["data_type"] =
            Value::String(String::from("windows:ese:srum:application_timeline:entry"));
    } else if !data["start_time"].is_null() {
        data["artifact"] = Value::String(String::from("SRUM App VFU"));
        data["data_type"] = Value::String(String::from("windows:ese:srum:app_vfu:entry"));
    } else if !data["binary_data"].is_null() {
        data["artifact"] = Value::String(String::from("SRUM Energy Info"));
        data["data_type"] = Value::String(String::from("windows:ese:srum:energy_info:entry"));
    } else if !data["event_timestamp"].is_null() {
        data["artifact"] = Value::String(String::from("SRUM Energy Usage"));
        data["data_type"] = Value::String(String::from("windows:ese:srum:energy_usage:entry"));
    } else if !data["bytes_sent"].is_null() {
        data["artifact"] = Value::String(String::from("SRUM Network Info"));
        data["data_type"] = Value::String(String::from("windows:ese:srum:network_info:entry"));
    } else if !data["connected_time"].is_null() {
        data["artifact"] = Value::String(String::from("SRUM Network Connectivity"));
        data["data_type"] =
            Value::String(String::from("windows:ese:srum:network_connectivity:entry"));
    } else if !data["notification_type"].is_null() {
        data["artifact"] = Value::String(String::from("SRUM Notification Info"));
        data["data_type"] = Value::String(String::from(
            "windows:ese:srum:network_connectivity:notification_info:entry",
        ));
    }

    true
}

pub(crate) fn tasks(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
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
    data["artifact"] = Value::String(String::from("Schedule Task"));
    data["data_type"] = Value::String(String::from("windows:tasks:xml:entry"));
    data["timestamp_desc"] = Value::String(String::from("Task Created"));

    true
}

pub(crate) fn userassist(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
    if !data.is_object() {
        return false;
    }
    let Some(last_execution) = data["last_execution"].as_str() else {
        return false;
    };

    if filter_data(last_execution, start, end) {
        return false;
    }
    data["datetime"] = last_execution.into();
    data["message"] = data["path"].as_str().unwrap_or_default().into();
    data["artifact"] = Value::String(String::from("Userassist"));
    data["data_type"] = Value::String(String::from("windows:registry:userassist:entry"));
    data["timestamp_desc"] = Value::String(String::from("Userassist Last Execution"));

    true
}

pub(crate) fn usnjrnl(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
    if !data.is_object() {
        return false;
    }
    let Some(update_time) = data["update_time"].as_str() else {
        return false;
    };

    if filter_data(update_time, start, end) {
        return false;
    }

    data["datetime"] = update_time.into();
    data["message"] = data["full_path"].as_str().unwrap_or_default().into();
    data["artifact"] = Value::String(String::from("UsnJrnl"));
    data["data_type"] = Value::String(String::from("windows:ntfs:usnjrnl:entry"));
    data["timestamp_desc"] = Value::String(format!(
        "UsnJrnl {:?}",
        data["update_reason"].as_array().unwrap_or(&Vec::new())
    ));

    true
}

pub(crate) fn wmi(data: &mut Value) -> bool {
    if !data.is_object() {
        return false;
    }
    data["message"] = data["consumer"].as_str().unwrap_or_default().into();
    data["datetime"] = Value::String(String::from("1970-01-01T00:00:00.000Z"));
    data["timestamp_desc"] = Value::String(String::from("N/A"));
    data["artifact"] = Value::String(String::from("WMI Persist"));
    data["data_type"] = Value::String(String::from("windows:wmi:persistence:entry"));

    true
}

pub(crate) fn mft(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
    if !data.is_object() {
        return false;
    }
    let mut entries = Vec::new();
    data["artifact"] = Value::String(String::from("MFT"));
    data["data_type"] = Value::String(String::from("windows:ntfs:mft::entry"));
    data["message"] = Value::String(data["full_path"].as_str().unwrap_or_default().into());

    let temp = json![{
        "created": data["created"].as_str().unwrap_or_default(),
        "modified": data["modified"].as_str().unwrap_or_default(),
        "accessed": data["accessed"].as_str().unwrap_or_default(),
        "changed": data["changed"].as_str().unwrap_or_default(),
        "filename_created": data["filename_created"].as_str().unwrap_or_default(),
        "filename_modified": data["filename_modified"].as_str().unwrap_or_default(),
        "filename_accessed": data["filename_accessed"].as_str().unwrap_or_default(),
        "filename_changed": data["filename_changed"].as_str().unwrap_or_default(),
    }];
    let mut times = extract_times(&temp).unwrap_or_default();
    extract_filename_times(&temp, &mut times).unwrap_or_default();
    for (key, value) in times {
        if filter_data(key, start, end) {
            continue;
        }
        data["datetime"] = Value::String(key.into());
        data["timestamp_desc"] = Value::String(value);
        entries.push(data.clone());
    }

    if entries.is_empty() {
        return false;
    }

    *data = Value::Array(entries);
    true
}

#[cfg(test)]
mod tests {
    use crate::artifacts::windows::{
        amcache, bits, eventlogs, jumplists, mft, outlook, prefetch, raw_files, recycle_bin,
        registry, users,
    };
    use serde_json::json;

    #[test]
    fn test_users() {
        let mut test = json!({
            "last_logon": "2024-01-01T00:00:00.000Z",
            "username":"anything i want"
        });

        assert!(users(&mut test, &None, &None));
        assert_eq!(test["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test["artifact"], "Windows User");
        assert_eq!(test["username"], "anything i want");
    }

    #[test]
    fn test_amcache() {
        let mut test = json!({
            "last_modified": "2024-01-01T00:00:00.000Z",
            "path":"C:\\Windows\\cmd.exe"
        });

        assert!(amcache(&mut test, &None, &None));
        assert_eq!(test["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test["artifact"], "Amcache");
        assert_eq!(test["message"], "C:\\Windows\\cmd.exe");
    }

    #[test]
    fn test_amcache_missing_path() {
        let mut test = json!({
            "last_modified": "2024-01-01T00:00:00.000Z",
        });

        assert!(amcache(&mut test, &None, &None));
        assert_eq!(test["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test["artifact"], "Amcache");
        assert_eq!(test["message"], "");
    }

    #[test]
    fn test_bits() {
        let mut test = json!({
            "modified": "2024-01-01T00:00:00.000Z",
            "created": "2024-01-01T00:00:00.000Z",
            "expiration": "2024-01-01T00:00:00.000Z",
            "completed": "2024-01-01T00:00:00.000Z",
            "target_path":"C:\\Windows\\cmd.exe",
            "job_name": "test"
        });

        assert!(bits(&mut test, &None, &None));
        assert_eq!(test.as_array().unwrap().len(), 1);
        assert_eq!(test[0]["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test[0]["artifact"], "BITS");
        assert_eq!(
            test[0]["message"],
            "Job: test - Target Path: C:\\Windows\\cmd.exe"
        );
    }

    #[test]
    fn test_eventlogs() {
        let mut test = json!({
            "generated": "2024-01-01T00:00:00.000Z",
            "message":"C:\\Windows\\cmd.exe",
            "template_message": "%1 data"
        });

        assert!(eventlogs(&mut test, &None, &None));
        assert_eq!(test["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test["artifact"], "EventLogs");
        assert_eq!(test["message"], "C:\\Windows\\cmd.exe");
    }

    #[test]
    fn test_jumplists() {
        let mut test = json!({
            "lnk_info": {
                "created": "2024-01-01T00:00:00.000Z",
                "modified": "2024-01-01T00:00:00.000Z",
                "accessed": "2024-01-01T00:00:00.000Z",
                "path":"C:\\Windows\\cmd.exe",
            },
            "jumplist_metadata": {},
        });

        assert!(jumplists(&mut test, &None, &None));
        assert_eq!(test[0]["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test[0]["artifact"], "Jumplist");
        assert_eq!(test[0]["message"], "C:\\Windows\\cmd.exe");
    }

    #[test]
    fn test_registry() {
        let mut test = json!({
            "last_modified": "2024-01-01T00:00:00.000Z",
            "path": "HKEY\\Test\\Run",
            "values": [
                {
                    "value": "test",
                    "data": "base64 encoded",
                    "reg_data_type": "BIN",
                }
            ],
        });

        assert!(registry(&mut test, &None, &None));
        assert_eq!(test[0]["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test[0]["artifact"], "Registry");
        assert_eq!(test[0]["message"], "HKEY\\Test\\Run | Value: test");
    }

    #[test]
    fn test_raw_files() {
        let mut test = json!({
            "created": "2024-01-01T00:00:00.000Z",
            "full_path": "/usr/bin/ls",
            "modified": "2024-01-01T03:00:00.000Z",
            "changed": "2024-01-01T02:00:00.000Z",
            "accessed": "2024-01-01T01:00:00.000Z",
            "filename_changed": "2024-01-01T03:00:00.000Z",
            "filename_created": "2024-01-01T03:00:00.000Z",
            "filename_modified": "2024-01-01T03:00:00.000Z",
            "filename_accessed": "2024-01-01T03:00:00.000Z",
        });

        assert!(raw_files(&mut test, &None, &None));
        assert_eq!(test[0]["accessed"], "2024-01-01T01:00:00.000Z");
        assert_eq!(test[0]["artifact"], "RawFiles");
        assert_eq!(test[0]["message"], "/usr/bin/ls");
    }

    #[test]
    fn test_raw_files_empty() {
        let mut test = json!({
            "created": "",
            "full_path": "/usr/bin/ls",
            "modified": "",
            "changed": "",
            "accessed": "",
            "filename_changed": "2024-01-01T03:00:00.001Z",
            "filename_created": "2024-01-01T03:00:00.002Z",
            "filename_modified": "2024-01-01T03:00:00.030Z",
            "filename_accessed": "2024-01-01T03:00:00.040Z",
        });

        assert!(raw_files(&mut test, &None, &None));
        assert_eq!(test.as_array().unwrap().len(), 4);
    }

    #[test]
    fn test_mft() {
        let mut test = json!({
            "created": "2024-01-01T00:00:00.000Z",
            "full_path": "/usr/bin/ls",
            "modified": "2024-01-01T03:00:00.000Z",
            "changed": "2024-01-01T02:00:00.000Z",
            "accessed": "2024-01-01T01:00:00.000Z",
            "filename_changed": "2024-01-01T03:00:00.000Z",
            "filename_created": "2024-01-01T03:00:00.000Z",
            "filename_modified": "2024-01-01T03:00:00.000Z",
            "filename_accessed": "2024-01-01T03:00:00.000Z",
        });

        assert!(mft(&mut test, &None, &None));
        assert_eq!(test[0]["accessed"], "2024-01-01T01:00:00.000Z");
        assert_eq!(test[0]["artifact"], "MFT");
        assert_eq!(test[0]["message"], "/usr/bin/ls");
    }

    #[test]
    fn test_outlook() {
        let mut test = json!({
            "delivered": "2024-01-01T00:00:00.000Z",
            "subject": "testing timelines",
            "from": "me!"
        });

        assert!(outlook(&mut test, &None, &None));
        assert_eq!(test["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test["artifact"], "Outlook");
        assert_eq!(test["message"], "Subject: testing timelines From: me!");
    }

    #[test]
    fn test_prefetch() {
        let mut test = json!({
            "last_run_time": "2024-01-01T00:00:00.000Z",
            "evidence": "test.pf",
            "all_run_times": [],
        });

        assert!(prefetch(&mut test, &None, &None));
        assert_eq!(test[0]["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test[0]["artifact"], "Prefetch");
        assert_eq!(test[0]["message"], "test.pf");
    }

    #[test]
    fn test_recycle_bin() {
        let mut test = json!({
            "deleted": "2024-01-01T00:00:00.000Z",
            "full_path": "test.pf",
        });

        assert!(recycle_bin(&mut test, &None, &None));
        assert_eq!(test["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test["artifact"], "RecycleBin");
        assert_eq!(test["message"], "test.pf");
    }
}
