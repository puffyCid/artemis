use super::files::{extract_filename_times, extract_times};
use serde_json::Value;
use std::collections::HashMap;

/// Timeline Windows Users
pub(crate) fn users(data: &mut Value) -> Option<()> {
    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };
        entry["datetime"] = entry["last_logon"].as_str()?.into();
        entry["message"] = entry["username"].as_str()?.into();
        entry["artifact"] = Value::String(String::from("Windows User"));
        entry["data_type"] = Value::String(String::from("windows:registry:users:entry"));
        entry["timestamp_desc"] = Value::String(String::from("User Last Logon"));
    }

    Some(())
}

/// Timeline Amcache
pub(crate) fn amcache(data: &mut Value) -> Option<()> {
    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };
        entry["datetime"] = entry["last_modified"].as_str()?.into();
        entry["message"] = entry["path"].as_str()?.into();
        entry["artifact"] = Value::String(String::from("Amcache"));
        entry["data_type"] = Value::String(String::from("windows:registry:amcache:entry"));
        entry["timestamp_desc"] = Value::String(String::from("Amcache Registry Last Modified"));
    }

    Some(())
}

/// Timeline Windows BITS
pub(crate) fn bits(data: &mut Value) -> Option<()> {
    let mut entries = Vec::new();

    for values in data.as_array_mut()? {
        let mut temp = Value::Null;
        let bits = if let Some(value) = values.get_mut("data").unwrap_or(&mut temp).get_mut("bits")
        {
            value
        } else {
            values.get_mut("bits")?
        };
        // First get full BITS jobs
        for entry in bits.as_array_mut()? {
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
                entry["datetime"] = Value::String(key.into());
                entry["timestamp_desc"] = Value::String(value);
                entries.push(entry.clone());
            }
        }

        // Now get carved jobs
        let mut jobs = if let Some(value) = values
            .get_mut("data")
            .unwrap_or(&mut temp)
            .get_mut("carved_jobs")
        {
            value.clone()
        } else {
            values.get_mut("carved_jobs")?.clone()
        };
        // Get carved Jobs
        for entry in jobs.as_array_mut()? {
            entry["message"] = Value::String(format!(
                "Job: {} - Target Path: {}",
                entry["job_name"].as_str()?,
                entry["target_path"].as_str()?
            ));
            entry["artifact"] = Value::String(String::from("BITS Carved Job"));
            entry["data_type"] = Value::String(String::from("windows:ese:bits:carve:job"));

            let temp = entry.clone();
            let times = extract_bits_times(&temp)?;
            for (key, value) in times {
                entry["datetime"] = Value::String(key.into());
                entry["timestamp_desc"] = Value::String(format!("Carved {value}"));
                entries.push(entry.clone());
            }
        }

        // Now get carved files
        let mut files = if let Some(value) = values
            .get_mut("data")
            .unwrap_or(&mut temp)
            .get_mut("carved_files")
        {
            value.clone()
        } else {
            values.get_mut("carved_files")?.clone()
        };
        for entry in files.as_array_mut()? {
            entry["message"] = Value::String(format!(
                "File: {} - URL: {}",
                entry["target_path"].as_str()?,
                entry["url"].as_str()?
            ));
            entry["artifact"] = Value::String(String::from("Carved BITS File"));
            entry["data_type"] = Value::String(String::from("windows:ese:bits:carve:file"));
            entry["datetime"] = Value::String(String::from("1601-01-01T00:00:00.000Z"));
            entry["timestamp_desc"] = Value::String(String::from("BITS Carved File"));
            entries.push(entry.clone());
        }
    }

    let mut has_meta = Value::Null;
    if let Some(values) = (data.as_array()?).iter().next() {
        if let Some(value) = values.get("metadata") {
            has_meta = value.clone();
        }
    }

    if !has_meta.is_null() {
        for entry in entries.iter_mut() {
            entry["metadata"] = has_meta.clone();
        }
    }

    data.as_array_mut()?.clear();
    data.as_array_mut()?.append(&mut entries);

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
    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };

        if entry["template_message"] == Value::Null {
            continue;
        }

        entry["datetime"] = entry["generated"].as_str()?.into();
        entry["artifact"] = Value::String(String::from("EventLogs"));
        entry["data_type"] = Value::String(String::from("windows:eventlogs:entry"));
        entry["timestamp_desc"] = Value::String(String::from("EventLog Entry Generated"));
    }

    Some(())
}

pub(crate) fn jumplists(data: &mut Value) -> Option<()> {
    let mut entries = Vec::new();

    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };
        entry["artifact"] = Value::String(String::from("Jumplist"));
        entry["data_type"] = Value::String(String::from("windows:jumplist:entry"));

        let temp = entry.clone();
        let times = extract_shortcut_times(&temp["lnk_info"])?;

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
                continue;
            } else if key == "modified" {
                entry["entry_modified"] = value.clone();
            }
            entry[key] = value.clone();
        }
        entry.as_object_mut()?.remove("jumplist_metadata");

        for (key, value) in times {
            entry["datetime"] = Value::String(key.into());
            entry["timestamp_desc"] = Value::String(value);
            entries.push(entry.clone());
        }
    }

    let mut has_meta = Value::Null;
    if let Some(values) = (data.as_array()?).iter().next() {
        if let Some(value) = values.get("metadata") {
            has_meta = value.clone();
        }
    }

    if !has_meta.is_null() {
        for entry in entries.iter_mut() {
            entry["metadata"] = has_meta.clone();
        }
    }

    data.as_array_mut()?.clear();
    data.as_array_mut()?.append(&mut entries);

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

    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };

        entry["artifact"] = Value::String(String::from("RawFiles"));
        entry["data_type"] = Value::String(String::from("windows:ntfs:file"));
        entry["message"] = Value::String(entry["full_path"].as_str()?.into());

        let temp = entry.clone();
        let mut times = extract_times(&temp)?;
        extract_filename_times(&temp, &mut times)?;
        for (key, value) in times {
            entry["datetime"] = Value::String(key.into());
            entry["timestamp_desc"] = Value::String(value);
            entries.push(entry.clone());
        }
    }

    let mut has_meta = Value::Null;
    if let Some(values) = (data.as_array()?).iter().next() {
        if let Some(value) = values.get("metadata") {
            has_meta = value.clone();
        }
    }

    if !has_meta.is_null() {
        for entry in entries.iter_mut() {
            entry["metadata"] = has_meta.clone();
        }
    }

    data.as_array_mut()?.clear();
    data.as_array_mut()?.append(&mut entries);

    Some(())
}

pub(crate) fn outlook(data: &mut Value) -> Option<()> {
    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };

        entry["message"] = Value::String(format!(
            "Subject: {} From: {}",
            entry["subject"].as_str()?,
            entry["from"].as_str()?
        ));
        entry["datetime"] = entry["delivered"].as_str()?.into();
        entry["artifact"] = Value::String(String::from("Outlook"));
        entry["data_type"] = Value::String(String::from("windows:outlook:email"));
        entry["timestamp_desc"] = Value::String(String::from("Email Delivered"));
    }

    Some(())
}

pub(crate) fn prefetch(data: &mut Value) -> Option<()> {
    let mut entries = Vec::new();

    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };

        entry["artifact"] = Value::String(String::from("Prefetch"));
        entry["data_type"] = Value::String(String::from("windows:prefetch:file"));
        entry["message"] = Value::String(entry["path"].as_str()?.into());
        entry["datetime"] = entry["last_runtime"].as_str()?.into();
        entry["timestamp_desc"] = Value::String(String::from("Prefetch Last Execution"));
        entries.push(entry.clone());

        let mut temp = entry.clone();

        for value in entry["all_run_times"].as_array()? {
            temp["datetime"] = value["last_runtime"].as_str()?.into();
            temp["timestamp_desc"] = Value::String(String::from("Prefetch Execution"));
            entries.push(temp.clone());
        }
    }

    let mut has_meta = Value::Null;
    if let Some(values) = (data.as_array()?).iter().next() {
        if let Some(value) = values.get("metadata") {
            has_meta = value.clone();
        }
    }

    if !has_meta.is_null() {
        for entry in entries.iter_mut() {
            entry["metadata"] = has_meta.clone();
        }
    }

    data.as_array_mut()?.clear();
    data.as_array_mut()?.append(&mut entries);
    Some(())
}

pub(crate) fn recycle_bin(data: &mut Value) -> Option<()> {
    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };

        entry["message"] = entry["full_path"].as_str()?.into();
        entry["datetime"] = entry["deleted"].as_str()?.into();
        entry["artifact"] = Value::String(String::from("RecycleBin"));
        entry["data_type"] = Value::String(String::from("windows:recyclebin:entry"));
        entry["timestamp_desc"] = Value::String(String::from("File Deleted"));
    }
    Some(())
}

pub(crate) fn search(data: &mut Value) -> Option<()> {
    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };

        entry["message"] = entry["entry"].as_str()?.into();
        entry["datetime"] = entry["last_modified"].as_str()?.into();
        entry["artifact"] = Value::String(String::from("Search"));
        entry["data_type"] = Value::String(String::from("windows:ese:search:entry"));
        entry["timestamp_desc"] = Value::String(String::from("Entry Last Modified"));
    }
    Some(())
}

pub(crate) fn searvices(data: &mut Value) -> Option<()> {
    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };

        entry["message"] = Value::String(format!(
            "Service Name: {} | {}",
            entry["name"].as_str()?,
            entry["path"].as_str()?,
        ));
        entry["datetime"] = entry["modified"].as_str()?.into();
        entry["artifact"] = Value::String(String::from("Service"));
        entry["data_type"] = Value::String(String::from("windows:registry:services:entry"));
        entry["timestamp_desc"] = Value::String(String::from("Registry Last Modified"));
    }
    Some(())
}

pub(crate) fn shellbags(data: &mut Value) -> Option<()> {
    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };

        entry["message"] = entry["path"].as_str()?.into();
        entry["datetime"] = entry["reg_modified"].as_str()?.into();
        entry["artifact"] = Value::String(String::from("Shellbags"));
        entry["data_type"] = Value::String(String::from("windows:registry:shellbags:entry"));
        entry["timestamp_desc"] = Value::String(String::from("Registry Last Modified"));
    }
    Some(())
}

pub(crate) fn shimcache(data: &mut Value) -> Option<()> {
    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };

        entry["message"] = entry["path"].as_str()?.into();
        entry["datetime"] = entry["last_modified"].as_str()?.into();
        entry["artifact"] = Value::String(String::from("Shimcache"));
        entry["data_type"] = Value::String(String::from("windows:registry:shimcache:entry"));
        entry["timestamp_desc"] = Value::String(String::from("Shimcache Last Modified"));
    }
    Some(())
}

pub(crate) fn registry(data: &mut Value) -> Option<()> {
    let mut entries = Vec::new();

    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };

        entry["artifact"] = Value::String(String::from("Registry"));
        entry["data_type"] = Value::String(String::from("windows:registry:entry"));
        entry["datetime"] = entry["last_modified"].as_str()?.into();
        entry["timestamp_desc"] = Value::String(String::from("Registry Last Modified"));

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
            entry["reg_data_type"] = value["reg_data_type"].clone();
            entries.push(entry.clone());
        }
    }

    let mut has_meta = Value::Null;
    if let Some(values) = (data.as_array()?).iter().next() {
        if let Some(value) = values.get("metadata") {
            has_meta = value.clone();
        }
    }

    if !has_meta.is_null() {
        for entry in entries.iter_mut() {
            entry["metadata"] = has_meta.clone();
        }
    }

    data.as_array_mut()?.clear();
    data.as_array_mut()?.append(&mut entries);

    Some(())
}

pub(crate) fn shimdb(data: &mut Value) -> Option<()> {
    let mut entries = Vec::new();

    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };

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

        let temp = entry.clone();
        entry.as_object_mut()?.remove("db_data");

        // Flatten db_data
        for (key, value) in temp["db_data"].as_object()? {
            if key == "list_data" {
                // If we parsed the sysmain.sdb file. This will be a lot of data
                for tag in value.as_array().unwrap_or(&Vec::new()) {
                    entry["message"] = Value::String(format!(
                        "{} | Shim Tag Name: {}",
                        temp["sdb_path"].as_str()?,
                        tag["data"].as_object()?["TAG_NAME"].as_str().unwrap_or("")
                    ));
                    entry["list_data"] = tag.clone();
                }
                continue;
            }
            entry[key] = value.clone();
        }
        entries.push(entry.clone());
    }

    let mut has_meta = Value::Null;
    if let Some(values) = (data.as_array()?).iter().next() {
        if let Some(value) = values.get("metadata") {
            has_meta = value.clone();
        }
    }

    if !has_meta.is_null() {
        for entry in entries.iter_mut() {
            entry["metadata"] = has_meta.clone();
        }
    }

    data.as_array_mut()?.clear();
    data.as_array_mut()?.append(&mut entries);
    Some(())
}

pub(crate) fn shortcuts(data: &mut Value) -> Option<()> {
    let mut entries = Vec::new();

    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };
        entry["artifact"] = Value::String(String::from("Shortcut"));
        entry["data_type"] = Value::String(String::from("windows:shortcut:lnk"));
        entry["message"] = entry["source_path"].clone();

        let temp = entry.clone();
        let times = extract_shortcut_times(&temp)?;

        for (key, value) in times {
            entry["datetime"] = Value::String(key.into());
            entry["timestamp_desc"] = Value::String(value);
            entries.push(entry.clone());
        }
    }

    let mut has_meta = Value::Null;
    if let Some(values) = (data.as_array()?).iter().next() {
        if let Some(value) = values.get("metadata") {
            has_meta = value.clone();
        }
    }

    if !has_meta.is_null() {
        for entry in entries.iter_mut() {
            entry["metadata"] = has_meta.clone();
        }
    }

    data.as_array_mut()?.clear();
    data.as_array_mut()?.append(&mut entries);

    Some(())
}

pub(crate) fn srum(data: &mut Value) -> Option<()> {
    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };

        entry["message"] = entry["app_id"].as_str()?.into();
        entry["datetime"] = entry["timestamp"].as_str()?.into();
        entry["srum_timestamp"] = entry["timestamp"].as_str()?.into();
        // Timestamp is reserved word by Timesketch
        entry.as_object_mut()?.remove("timestamp");
        entry["timestamp_desc"] = Value::String(String::from("SRUM Table Update"));

        if !entry["facetime"].is_null() {
            entry["artifact"] = Value::String(String::from("SRUM Application Info"));
            entry["data_type"] =
                Value::String(String::from("windows:ese:srum:application_info:entry"));
        } else if !entry["cycles_web"].is_null() {
            entry["artifact"] = Value::String(String::from("SRUM ASRUM Application Timeline"));
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
    }
    Some(())
}

pub(crate) fn tasks(data: &mut Value) -> Option<()> {
    let mut entries = Vec::new();

    for values in data.as_array_mut()? {
        let mut temp = Value::Null;
        let bits = if let Some(value) = values.get_mut("data").unwrap_or(&mut temp).get_mut("tasks")
        {
            value
        } else {
            values.get_mut("tasks")?
        };
        // First get full modern tasks
        for entry in bits.as_array_mut()? {
            entry["message"] = entry["path"].as_str().into();
            entry["artifact"] = Value::String(String::from("Schedule Task"));
            entry["data_type"] = Value::String(String::from("windows:tasks:xml:entry"));
            entry["datetime"] = Value::String(String::from("1970-01-01T00:00:00.000Z"));
            entry["timestamp_desc"] = Value::String(String::from("N/A"));
            entries.push(entry.clone());
        }

        // Now get legacy jobs
        let mut jobs =
            if let Some(value) = values.get_mut("data").unwrap_or(&mut temp).get_mut("jobs") {
                value.clone()
            } else {
                values.get_mut("jobs")?.clone()
            };
        // Get legacy Jobs
        for entry in jobs.as_array_mut()? {
            entry["message"] = entry["path"].as_str().into();
            entry["artifact"] = Value::String(String::from("Schedule Task"));
            entry["data_type"] = Value::String(String::from("windows:tasks:jobs:entry"));
            entry["datetime"] = Value::String(String::from("1970-01-01T00:00:00.000Z"));
            entry["timestamp_desc"] = Value::String(String::from("N/A"));
            entries.push(entry.clone());
        }
    }

    let mut has_meta = Value::Null;
    if let Some(values) = (data.as_array()?).iter().next() {
        if let Some(value) = values.get("metadata") {
            has_meta = value.clone();
        }
    }

    if !has_meta.is_null() {
        for entry in entries.iter_mut() {
            entry["metadata"] = has_meta.clone();
        }
    }

    data.as_array_mut()?.clear();
    data.as_array_mut()?.append(&mut entries);

    Some(())
}

pub(crate) fn userassist(data: &mut Value) -> Option<()> {
    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };

        entry["message"] = entry["path"].as_str()?.into();
        entry["datetime"] = entry["last_execution"].as_str()?.into();
        entry["artifact"] = Value::String(String::from("Userassist"));
        entry["data_type"] = Value::String(String::from("windows:registry:userassist:entry"));
        entry["timestamp_desc"] = Value::String(String::from("Userassist Last Execution"));
    }
    Some(())
}

pub(crate) fn usnjrnl(data: &mut Value) -> Option<()> {
    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };

        entry["message"] = entry["full_path"].as_str()?.into();
        entry["datetime"] = entry["update_time"].as_str()?.into();
        entry["artifact"] = Value::String(String::from("Userassist"));
        entry["data_type"] = Value::String(String::from("windows:ntfs:usnjrnl:entry"));
        entry["timestamp_desc"] =
            Value::String(format!("UsnJrnl {}", entry["update_reason"].as_str()?));
    }
    Some(())
}

pub(crate) fn wmi(data: &mut Value) -> Option<()> {
    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };

        entry["message"] = entry["consumer"].as_str()?.into();
        entry["datetime"] = Value::String(String::from("1970-01-01T00:00:00.000Z"));
        entry["timestamp_desc"] = Value::String(String::from("N/A"));
        entry["artifact"] = Value::String(String::from("WMI Persist"));
        entry["data_type"] = Value::String(String::from("windows:wmi:persistence:entry"));
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use crate::artifacts::windows::{amcache, bits, eventlogs, jumplists, registry, users};
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
            "bits": [{
                "modified": "2024-01-01T00:00:00.000Z",
                "created": "2024-01-01T00:00:00.000Z",
                "expiration": "2024-01-01T00:00:00.000Z",
                "completed": "2024-01-01T00:00:00.000Z",
                "target_path":"C:\\Windows\\cmd.exe",
                "job_name": "test"
            }],
            "carved_files": [],
            "carved_jobs": [],

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
            "path":"C:\\Windows\\cmd.exe",
            "lnk_info": {
                "created": "2024-01-01T00:00:00.000Z",
                "modified": "2024-01-01T00:00:00.000Z",
                "accessed": "2024-01-01T00:00:00.000Z",
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
}
