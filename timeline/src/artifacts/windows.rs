use super::files::{extract_filename_times, extract_times};
use crate::artifacts::filter::filter_data;
use serde_json::{Map, Value, json};
use std::collections::HashMap;

/// Timeline Windows Users
pub(crate) fn users(data: &mut Value) -> Option<()> {
    data.as_array_mut()?.retain_mut(|entry| {
        if !entry.is_object() {
            // Drop value if its not an object
            return false;
        }

        let last_logon = match entry["last_logon"].as_str() {
            Some(result) => result,
            None => return false,
        };
        entry["datetime"] = last_logon.into();
        entry["message"] = entry["username"].as_str().unwrap_or_default().into();
        entry["artifact"] = Value::String(String::from("Windows User"));
        entry["data_type"] = Value::String(String::from("windows:registry:users:entry"));
        entry["timestamp_desc"] = Value::String(String::from("User Last Logon"));
        !filter_data(entry["datetime"].as_str().unwrap(), None, None)
    });

    Some(())
}

/// Timeline Amcache
pub(crate) fn amcache(data: &mut Value) -> Option<()> {
    data.as_array_mut()?.retain_mut(|entry| {
        if !entry.is_object() {
            // Drop value if its not an object
            return false;
        }

        let last_modified = match entry["last_modified"].as_str() {
            Some(result) => result,
            None => return false,
        };
        entry["datetime"] = last_modified.into();
        entry["message"] = entry["path"].as_str().unwrap_or_default().into();
        entry["artifact"] = Value::String(String::from("Amcache"));
        entry["data_type"] = Value::String(String::from("windows:registry:amcache:entry"));
        entry["timestamp_desc"] = Value::String(String::from("Amcache Registry Last Modified"));
        !filter_data(entry["datetime"].as_str().unwrap(), None, None)
    });

    Some(())
}

/// Timeline Windows BITS
pub(crate) fn bits(data: &mut Value) -> Option<()> {
    let mut entries = Vec::new();

    for entry in data.as_array_mut()? {
        entry["message"] = Value::String(format!(
            "Job: {} - Target Path: {}",
            entry["job_name"].as_str()?,
            entry["target_path"].as_str()?
        ));
        entry["artifact"] = Value::String(String::from("BITS"));
        entry["data_type"] = Value::String(String::from("windows:ese:bits:entry"));

        let temp = entry.clone();
        let times = extract_bits_times(&temp)?;
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
pub(crate) fn eventlogs(data: &mut Value) -> Option<()> {
    data.as_array_mut()?.retain_mut(|entry| {
        if !entry.is_object() {
            // Drop value if its not an object
            return false;
        }
        if entry["template_message"] == Value::Null {
            // Drop value if we do not have template message
            return false;
        }

        let generated = match entry["generated"].as_str() {
            Some(result) => result,
            None => return false,
        };
        entry["datetime"] = generated.into();

        if entry["message"].is_string() && entry["message"].as_str().unwrap_or_default().is_empty()
        {
            entry["message"] = Value::String(entry["raw_event_data"].to_string());
        }

        // Timesketch cannot handle large amounts of raw event data
        // It maxes out at 1000 total JSON keys per sketch
        // Unwrap safe since we check to make we have an object above
        entry.as_object_mut().unwrap().remove("raw_event_data");
        entry.as_object_mut().unwrap().remove("template_message");

        entry["artifact"] = Value::String(String::from("EventLogs"));
        entry["data_type"] = Value::String(String::from("windows:eventlogs:entry"));
        entry["timestamp_desc"] = Value::String(String::from("EventLog Entry Generated"));
        !filter_data(entry["datetime"].as_str().unwrap(), None, None)
    });

    Some(())
}

pub(crate) fn jumplists(data: &mut Value) -> Option<()> {
    let mut entries = Vec::new();

    for entry in data.as_array_mut()? {
        entry["artifact"] = Value::String(String::from("Jumplist"));
        entry["data_type"] = Value::String(String::from("windows:jumplist:entry"));

        let temp = entry.clone();

        entry.as_object_mut()?.remove("lnk_info");

        // Flatten lnk_info
        for (key, value) in temp["lnk_info"].as_object()? {
            if key == "path" {
                entry["target_path"] = value.clone();
                entry["message"] = value.clone();
                continue;
            }
            entry[key] = value.clone();
        }

        // Flatten jumplist_metadata
        for (key, value) in temp["jumplist_metadata"].as_object()? {
            if key == "path" {
                entry["jumplist_target"] = value.clone();
                if entry["message"] == Value::String(String::new()) {
                    entry["message"] = value.clone();
                }
                continue;
            } else if key == "modified" {
                entry["entry_modified"] = value.clone();
                continue;
            }
            entry[key] = value.clone();
        }
        entry.as_object_mut()?.remove("jumplist_metadata");

        if entry["message"] == Value::String(String::new()) {
            entry["message"] = entry["path"].clone();
        }

        let times = extract_shortcut_times(&temp["lnk_info"])?;

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

pub(crate) fn raw_files(data: &mut Value) -> Option<()> {
    let mut entries = Vec::new();

    for entry in data.as_array_mut()? {
        entry["artifact"] = Value::String(String::from("RawFiles"));
        entry["data_type"] = Value::String(String::from("windows:ntfs:file"));
        entry["message"] = Value::String(entry["full_path"].as_str()?.into());

        let temp = json![{
            "created": entry["created"].as_str()?,
            "modified": entry["modified"].as_str()?,
            "accessed": entry["accessed"].as_str()?,
            "changed": entry["changed"].as_str()?,
            "filename_created": entry["filename_created"].as_str()?,
            "filename_modified": entry["filename_modified"].as_str()?,
            "filename_accessed": entry["filename_accessed"].as_str()?,
            "filename_changed": entry["filename_changed"].as_str()?,
        }];

        let mut times = extract_times(&temp)?;
        extract_filename_times(&temp, &mut times)?;
        for (key, value) in times {
            // If $INDX recovery is enabled. Standard Info timestamps will be empty
            // We will only have FileName timestamps
            // Skip emtpy Standard Info timestamps
            if key.is_empty() || filter_data(key, None, None) {
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

pub(crate) fn outlook(data: &mut Value) -> Option<()> {
    data.as_array_mut()?.retain_mut(|entry| {
        if !entry.is_object() {
            // Drop value if its not an object
            return false;
        }
        let delivered = match entry["delivered"].as_str() {
            Some(result) => result,
            None => return false,
        };
        entry["datetime"] = delivered.into();
        entry["message"] = Value::String(format!(
            "Subject: {} From: {}",
            entry["subject"].as_str().unwrap_or_default(),
            entry["from"].as_str().unwrap_or_default()
        ));
        entry["artifact"] = Value::String(String::from("Outlook"));
        entry["data_type"] = Value::String(String::from("windows:outlook:email"));
        entry["timestamp_desc"] = Value::String(String::from("Email Delivered"));
        !filter_data(entry["datetime"].as_str().unwrap(), None, None)
    });

    Some(())
}

pub(crate) fn prefetch(data: &mut Value) -> Option<()> {
    let mut entries = Vec::new();

    for entry in data.as_array_mut()? {
        entry["artifact"] = Value::String(String::from("Prefetch"));
        entry["data_type"] = Value::String(String::from("windows:prefetch:file"));
        entry["message"] = Value::String(entry["evidence"].as_str()?.into());
        entry["datetime"] = entry["last_run_time"].as_str()?.into();
        entry["timestamp_desc"] = Value::String(String::from("Prefetch Last Execution"));
        if !filter_data(entry["last_run_time"].as_str()?, None, None) {
            entries.push(entry.clone());
        }

        let mut temp = entry.clone();

        for value in entry["all_run_times"].as_array()? {
            if filter_data(value.as_str()?, None, None) {
                continue;
            }
            temp["datetime"] = value.as_str()?.into();
            temp["timestamp_desc"] = Value::String(String::from("Prefetch Execution"));
            entries.push(temp.clone());
        }
    }
    *data.as_array_mut()? = entries;
    Some(())
}

pub(crate) fn recycle_bin(data: &mut Value) -> Option<()> {
    data.as_array_mut()?.retain_mut(|entry| {
        if !entry.is_object() {
            // Drop value if its not an object
            return false;
        }
        let deleted = match entry["deleted"].as_str() {
            Some(result) => result,
            None => return false,
        };
        entry["datetime"] = deleted.into();
        entry["message"] = entry["full_path"].as_str().unwrap_or_default().into();
        entry["artifact"] = Value::String(String::from("RecycleBin"));
        entry["data_type"] = Value::String(String::from("windows:recyclebin:entry"));
        entry["timestamp_desc"] = Value::String(String::from("File Deleted"));
        !filter_data(entry["datetime"].as_str().unwrap(), None, None)
    });

    Some(())
}

pub(crate) fn search(data: &mut Value) -> Option<()> {
    data.as_array_mut()?.retain_mut(|entry| {
        if !entry.is_object() {
            // Drop value if its not an object
            return false;
        }
        let last_modified = match entry["last_modified"].as_str() {
            Some(result) => result,
            None => return false,
        };
        entry["datetime"] = last_modified.into();
        entry["message"] = entry["entry"].as_str().unwrap_or_default().into();
        entry["artifact"] = Value::String(String::from("Search"));
        entry["data_type"] = Value::String(String::from("windows:ese:search:entry"));
        entry["timestamp_desc"] = Value::String(String::from("Entry Last Modified"));

        let temp = entry["properties"]
            .as_object()
            .unwrap_or(&Map::new())
            .clone();
        entry.as_object_mut().unwrap().remove("properties");

        for (key, value) in &temp {
            entry[key] = value.clone();

            if entry["message"].as_str().unwrap_or_default().is_empty()
                && key.contains("System_ItemPathDisplay")
            {
                entry["message"] = value.as_str().unwrap_or_default().into();
            }
        }
        !filter_data(entry["datetime"].as_str().unwrap(), None, None)
    });

    Some(())
}

pub(crate) fn services(data: &mut Value) -> Option<()> {
    data.as_array_mut()?.retain_mut(|entry| {
        if !entry.is_object() {
            // Drop value if its not an object
            return false;
        }
        let modified = match entry["modified"].as_str() {
            Some(result) => result,
            None => return false,
        };
        entry["datetime"] = modified.into();
        entry["message"] = Value::String(format!(
            "Service Name: {} | {}",
            entry["name"].as_str().unwrap_or_default(),
            entry["path"].as_str().unwrap_or_default(),
        ));
        entry["artifact"] = Value::String(String::from("Service"));
        entry["data_type"] = Value::String(String::from("windows:registry:services:entry"));
        entry["timestamp_desc"] = Value::String(String::from("Registry Last Modified"));
        !filter_data(entry["datetime"].as_str().unwrap(), None, None)
    });

    Some(())
}

pub(crate) fn shellbags(data: &mut Value) -> Option<()> {
    data.as_array_mut()?.retain_mut(|entry| {
        if !entry.is_object() {
            // Drop value if its not an object
            return false;
        }
        let reg_modified = match entry["reg_modified"].as_str() {
            Some(result) => result,
            None => return false,
        };
        entry["datetime"] = reg_modified.into();
        entry["message"] = entry["path"].as_str().unwrap_or_default().into();
        entry["artifact"] = Value::String(String::from("Shellbags"));
        entry["data_type"] = Value::String(String::from("windows:registry:shellbags:entry"));
        entry["timestamp_desc"] = Value::String(String::from("Registry Last Modified"));
        !filter_data(entry["datetime"].as_str().unwrap(), None, None)
    });

    Some(())
}

pub(crate) fn shimcache(data: &mut Value) -> Option<()> {
    data.as_array_mut()?.retain_mut(|entry| {
        if !entry.is_object() {
            // Drop value if its not an object
            return false;
        }
        let last_modified = match entry["last_modified"].as_str() {
            Some(result) => result,
            None => return false,
        };
        entry["datetime"] = last_modified.into();
        entry["message"] = entry["path"].as_str().unwrap_or_default().into();
        entry["artifact"] = Value::String(String::from("Shimcache"));
        entry["data_type"] = Value::String(String::from("windows:registry:shimcache:entry"));
        entry["timestamp_desc"] = Value::String(String::from("Shimcache Last Modified"));
        !filter_data(entry["datetime"].as_str().unwrap(), None, None)
    });

    Some(())
}

pub(crate) fn registry(data: &mut Value) -> Option<()> {
    let mut entries = Vec::new();

    for entry in data.as_array_mut()? {
        entry["artifact"] = Value::String(String::from("Registry"));
        entry["data_type"] = Value::String(String::from("windows:registry:entry"));
        entry["datetime"] = entry["last_modified"].as_str()?.into();
        entry["timestamp_desc"] = Value::String(String::from("Registry Last Modified"));
        if filter_data(entry["datetime"].as_str().unwrap(), None, None) {
            continue;
        }

        let temp = entry.clone();
        entry.as_object_mut()?.remove("values")?;

        for value in temp["values"].as_array()? {
            entry["message"] = Value::String(format!(
                "{} | Value: {}",
                entry["path"].as_str()?,
                value["value"].as_str()?
            ));
            entry["value"] = value["value"].clone();
            entry["data"] = value["data"].clone();
            entry["reg_data_type"] = value["data_type"].clone();
            entries.push(entry.clone());
        }

        if temp["values"].as_array()?.is_empty() {
            entry["message"] = entry["path"].clone();
            entries.push(entry.clone());
        }
    }
    *data.as_array_mut()? = entries;
    Some(())
}

pub(crate) fn shimdb(data: &mut Value) -> Option<()> {
    let mut entries = Vec::new();

    for entry in data.as_array_mut()? {
        // If we include Indexes, memory usage will explode (~2GB). Indexes primarily contain base64 binary data. There's nothing parsable in it
        // This likely only affects sysmain.sdb due to the large number of Shims. We could split this into separate entries if needed
        // Custom Shims would likely be unaffected
        entry.as_object_mut()?.remove("indexes");

        entry["artifact"] = Value::String(String::from("Shimdb"));
        entry["data_type"] = Value::String(String::from("windows:shimdb:entry"));
        entry["datetime"] = entry["db_data"].as_object()?["compile_time"]
            .as_str()?
            .into();
        entry["timestamp_desc"] = Value::String(String::from("Shim Compile Time"));
        if filter_data(entry["datetime"].as_str().unwrap(), None, None) {
            continue;
        }

        let temp = entry.clone();
        entry.as_object_mut()?.remove("db_data");
        entry["message"] = Value::String(format!("{} | Shim: None", temp["evidence"].as_str()?,));

        // Flatten db_data
        for (key, value) in temp["db_data"].as_object()? {
            if key == "list_data" {
                // If we parsed the sysmain.sdb file. This will be a ton of data
                for tag in value.as_array().unwrap_or(&Vec::new()) {
                    let mut shim = entry.clone();
                    if let Some(value) = tag["data"].as_object() {
                        for (data_key, data_value) in value {
                            shim[data_key.clone()] = data_value.clone();
                            if data_key == "TAG_NAME" || data_key == "TAG_MODULE" {
                                shim["message"] = Value::String(format!(
                                    "{} | Shim {data_key}: {}",
                                    temp["evidence"].as_str()?,
                                    data_value.as_str()?,
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
    }
    *data.as_array_mut()? = entries;
    Some(())
}

pub(crate) fn shortcuts(data: &mut Value) -> Option<()> {
    let mut entries = Vec::new();

    for entry in data.as_array_mut()? {
        entry["artifact"] = Value::String(String::from("Shortcut"));
        entry["data_type"] = Value::String(String::from("windows:shortcut:lnk"));
        entry["message"] = entry["evidence"].clone();

        let temp = entry.clone();
        let times = extract_shortcut_times(&temp)?;

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

pub(crate) fn srum(data: &mut Value) -> Option<()> {
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
        entry["message"] = entry["app_id"].as_str().unwrap_or_default().into();
        entry["srum_timestamp"] = entry["timestamp"].as_str().unwrap().into();
        // Timestamp is reserved word by Timesketch
        entry.as_object_mut().unwrap().remove("timestamp");
        entry["timestamp_desc"] = Value::String(String::from("SRUM Table Update"));

        if !entry["facetime"].is_null() {
            entry["artifact"] = Value::String(String::from("SRUM Application Info"));
            entry["data_type"] =
                Value::String(String::from("windows:ese:srum:application_info:entry"));
        } else if !entry["cycles_web"].is_null() {
            entry["artifact"] = Value::String(String::from("SRUM Application Timeline"));
            entry["data_type"] =
                Value::String(String::from("windows:ese:srum:application_timeline:entry"));
        } else if !entry["start_time"].is_null() {
            entry["artifact"] = Value::String(String::from("SRUM App VFU"));
            entry["data_type"] = Value::String(String::from("windows:ese:srum:app_vfu:entry"));
        } else if !entry["binary_data"].is_null() {
            entry["artifact"] = Value::String(String::from("SRUM Energy Info"));
            entry["data_type"] = Value::String(String::from("windows:ese:srum:energy_info:entry"));
        } else if !entry["event_timestamp"].is_null() {
            entry["artifact"] = Value::String(String::from("SRUM Energy Usage"));
            entry["data_type"] = Value::String(String::from("windows:ese:srum:energy_usage:entry"));
        } else if !entry["bytes_sent"].is_null() {
            entry["artifact"] = Value::String(String::from("SRUM Network Info"));
            entry["data_type"] = Value::String(String::from("windows:ese:srum:network_info:entry"));
        } else if !entry["connected_time"].is_null() {
            entry["artifact"] = Value::String(String::from("SRUM Network Connectivity"));
            entry["data_type"] =
                Value::String(String::from("windows:ese:srum:network_connectivity:entry"));
        } else if !entry["notification_type"].is_null() {
            entry["artifact"] = Value::String(String::from("SRUM Notification Info"));
            entry["data_type"] = Value::String(String::from(
                "windows:ese:srum:network_connectivity:notification_info:entry",
            ));
        }
        !filter_data(entry["datetime"].as_str().unwrap(), None, None)
    });

    Some(())
}

pub(crate) fn tasks(data: &mut Value) -> Option<()> {
    data.as_array_mut()?.retain_mut(|entry| {
        if !entry.is_object() {
            // Drop value if its not an object
            return false;
        }
        let start_time = match entry["start_time"].as_str() {
            Some(result) => result,
            None => return false,
        };
        entry["datetime"] = start_time.into();
        entry["message"] = entry["evidence"].as_str().unwrap_or_default().into();
        entry["artifact"] = Value::String(String::from("Schedule Task"));
        entry["data_type"] = Value::String(String::from("windows:tasks:xml:entry"));
        entry["timestamp_desc"] = Value::String(String::from("Task Created"));
        !filter_data(entry["datetime"].as_str().unwrap(), None, None)
    });

    Some(())
}

pub(crate) fn userassist(data: &mut Value) -> Option<()> {
    data.as_array_mut()?.retain_mut(|entry| {
        if !entry.is_object() {
            // Drop value if its not an object
            return false;
        }
        let last_execution = match entry["last_execution"].as_str() {
            Some(result) => result,
            None => return false,
        };
        entry["datetime"] = last_execution.into();
        entry["message"] = entry["path"].as_str().unwrap_or_default().into();
        entry["artifact"] = Value::String(String::from("Userassist"));
        entry["data_type"] = Value::String(String::from("windows:registry:userassist:entry"));
        entry["timestamp_desc"] = Value::String(String::from("Userassist Last Execution"));
        !filter_data(entry["datetime"].as_str().unwrap(), None, None)
    });

    Some(())
}

pub(crate) fn usnjrnl(data: &mut Value) -> Option<()> {
    data.as_array_mut()?.retain_mut(|entry| {
        if !entry.is_object() {
            // Drop value if its not an object
            return false;
        }
        let update_time = match entry["update_time"].as_str() {
            Some(result) => result,
            None => return false,
        };
        entry["datetime"] = update_time.into();
        entry["message"] = entry["full_path"].as_str().unwrap_or_default().into();
        entry["artifact"] = Value::String(String::from("UsnJrnl"));
        entry["data_type"] = Value::String(String::from("windows:ntfs:usnjrnl:entry"));
        entry["timestamp_desc"] = Value::String(format!(
            "UsnJrnl {:?}",
            entry["update_reason"].as_array().unwrap_or(&Vec::new())
        ));
        !filter_data(entry["datetime"].as_str().unwrap(), None, None)
    });

    Some(())
}

pub(crate) fn wmi(data: &mut Value) -> Option<()> {
    data.as_array_mut()?.retain_mut(|entry| {
        if !entry.is_object() {
            // Drop value if its not an object
            return false;
        }
        entry["message"] = entry["consumer"].as_str().unwrap_or_default().into();
        entry["datetime"] = Value::String(String::from("1970-01-01T00:00:00.000Z"));
        entry["timestamp_desc"] = Value::String(String::from("N/A"));
        entry["artifact"] = Value::String(String::from("WMI Persist"));
        entry["data_type"] = Value::String(String::from("windows:wmi:persistence:entry"));
        !filter_data(entry["datetime"].as_str().unwrap(), None, None)
    });

    Some(())
}

pub(crate) fn mft(data: &mut Value) -> Option<()> {
    let mut entries = Vec::new();

    for entry in data.as_array_mut()? {
        entry["artifact"] = Value::String(String::from("MFT"));
        entry["data_type"] = Value::String(String::from("windows:ntfs:mft::entry"));
        entry["message"] = Value::String(entry["full_path"].as_str()?.into());

        let temp = json![{
            "created": entry["created"].as_str()?,
            "modified": entry["modified"].as_str()?,
            "accessed": entry["accessed"].as_str()?,
            "changed": entry["changed"].as_str()?,
            "filename_created": entry["filename_created"].as_str()?,
            "filename_modified": entry["filename_modified"].as_str()?,
            "filename_accessed": entry["filename_accessed"].as_str()?,
            "filename_changed": entry["filename_changed"].as_str()?,
        }];
        let mut times = extract_times(&temp)?;
        extract_filename_times(&temp, &mut times)?;
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

#[cfg(test)]
mod tests {
    use crate::artifacts::windows::{
        amcache, bits, eventlogs, jumplists, mft, outlook, prefetch, raw_files, recycle_bin,
        registry, users,
    };
    use serde_json::json;

    #[test]
    fn test_users() {
        let mut test = json!([{
            "last_logon": "2024-01-01T00:00:00.000Z",
            "username":"anything i want"
        }]);

        users(&mut test).unwrap();
        assert_eq!(test[0]["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test[0]["artifact"], "Windows User");
        assert_eq!(test[0]["username"], "anything i want");
    }

    #[test]
    fn test_amcache() {
        let mut test = json!([{
            "last_modified": "2024-01-01T00:00:00.000Z",
            "path":"C:\\Windows\\cmd.exe"
        }]);

        amcache(&mut test).unwrap();
        assert_eq!(test[0]["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test[0]["artifact"], "Amcache");
        assert_eq!(test[0]["message"], "C:\\Windows\\cmd.exe");
    }

    #[test]
    fn test_bits() {
        let mut test = json!([{
            "modified": "2024-01-01T00:00:00.000Z",
            "created": "2024-01-01T00:00:00.000Z",
            "expiration": "2024-01-01T00:00:00.000Z",
            "completed": "2024-01-01T00:00:00.000Z",
            "target_path":"C:\\Windows\\cmd.exe",
            "job_name": "test"
        }]);

        bits(&mut test).unwrap();
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
        let mut test = json!([{
            "generated": "2024-01-01T00:00:00.000Z",
            "message":"C:\\Windows\\cmd.exe",
            "template_message": "%1 data"
        }]);

        eventlogs(&mut test).unwrap();
        assert_eq!(test[0]["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test[0]["artifact"], "EventLogs");
        assert_eq!(test[0]["message"], "C:\\Windows\\cmd.exe");
    }

    #[test]
    fn test_jumplists() {
        let mut test = json!([{
            "lnk_info": {
                "created": "2024-01-01T00:00:00.000Z",
                "modified": "2024-01-01T00:00:00.000Z",
                "accessed": "2024-01-01T00:00:00.000Z",
                "path":"C:\\Windows\\cmd.exe",
            },
            "jumplist_metadata": {},
        }]);

        jumplists(&mut test).unwrap();
        assert_eq!(test[0]["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test[0]["artifact"], "Jumplist");
        assert_eq!(test[0]["message"], "C:\\Windows\\cmd.exe");
    }

    #[test]
    fn test_registry() {
        let mut test = json!([{
            "last_modified": "2024-01-01T00:00:00.000Z",
            "path": "HKEY\\Test\\Run",
            "values": [
                {
                    "value": "test",
                    "data": "base64 encoded",
                    "reg_data_type": "BIN",
                }
            ],
        }]);

        registry(&mut test).unwrap();
        assert_eq!(test[0]["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test[0]["artifact"], "Registry");
        assert_eq!(test[0]["message"], "HKEY\\Test\\Run | Value: test");
    }

    #[test]
    fn test_raw_files() {
        let mut test = json!([{
            "created": "2024-01-01T00:00:00.000Z",
            "full_path": "/usr/bin/ls",
            "modified": "2024-01-01T03:00:00.000Z",
            "changed": "2024-01-01T02:00:00.000Z",
            "accessed": "2024-01-01T01:00:00.000Z",
            "filename_changed": "2024-01-01T03:00:00.000Z",
            "filename_created": "2024-01-01T03:00:00.000Z",
            "filename_modified": "2024-01-01T03:00:00.000Z",
            "filename_accessed": "2024-01-01T03:00:00.000Z",
        }]);

        raw_files(&mut test).unwrap();
        assert_eq!(test[0]["accessed"], "2024-01-01T01:00:00.000Z");
        assert_eq!(test[0]["artifact"], "RawFiles");
        assert_eq!(test[0]["message"], "/usr/bin/ls");
    }

    #[test]
    fn test_raw_files_empty() {
        let mut test = json!([{
            "created": "",
            "full_path": "/usr/bin/ls",
            "modified": "",
            "changed": "",
            "accessed": "",
            "filename_changed": "2024-01-01T03:00:00.001Z",
            "filename_created": "2024-01-01T03:00:00.002Z",
            "filename_modified": "2024-01-01T03:00:00.030Z",
            "filename_accessed": "2024-01-01T03:00:00.040Z",
        }]);

        raw_files(&mut test).unwrap();
        assert_eq!(test.as_array().unwrap().len(), 4);
    }

    #[test]
    fn test_mft() {
        let mut test = json!([{
            "created": "2024-01-01T00:00:00.000Z",
            "full_path": "/usr/bin/ls",
            "modified": "2024-01-01T03:00:00.000Z",
            "changed": "2024-01-01T02:00:00.000Z",
            "accessed": "2024-01-01T01:00:00.000Z",
            "filename_changed": "2024-01-01T03:00:00.000Z",
            "filename_created": "2024-01-01T03:00:00.000Z",
            "filename_modified": "2024-01-01T03:00:00.000Z",
            "filename_accessed": "2024-01-01T03:00:00.000Z",
        }]);

        mft(&mut test).unwrap();
        assert_eq!(test[0]["accessed"], "2024-01-01T01:00:00.000Z");
        assert_eq!(test[0]["artifact"], "MFT");
        assert_eq!(test[0]["message"], "/usr/bin/ls");
    }

    #[test]
    fn test_outlook() {
        let mut test = json!([{
            "delivered": "2024-01-01T00:00:00.000Z",
            "subject": "testing timelines",
            "from": "me!"
        }]);

        outlook(&mut test).unwrap();
        assert_eq!(test[0]["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test[0]["artifact"], "Outlook");
        assert_eq!(test[0]["message"], "Subject: testing timelines From: me!");
    }

    #[test]
    fn test_prefetch() {
        let mut test = json!([{
            "last_run_time": "2024-01-01T00:00:00.000Z",
            "evidence": "test.pf",
            "all_run_times": [],
        }]);

        prefetch(&mut test).unwrap();
        assert_eq!(test[0]["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test[0]["artifact"], "Prefetch");
        assert_eq!(test[0]["message"], "test.pf");
    }

    #[test]
    fn test_recycle_bin() {
        let mut test = json!([{
            "deleted": "2024-01-01T00:00:00.000Z",
            "full_path": "test.pf",
        }]);

        recycle_bin(&mut test).unwrap();
        assert_eq!(test[0]["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test[0]["artifact"], "RecycleBin");
        assert_eq!(test[0]["message"], "test.pf");
    }
}
