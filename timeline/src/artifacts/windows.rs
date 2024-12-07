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
    for values in data.as_array()? {
        if let Some(value) = values.get("metadata") {
            has_meta = value.clone();
        }
        break;
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
        entry["message"] = entry["path"].as_str()?.into();
        entry["artifact"] = Value::String(String::from("Jumplist"));
        entry["data_type"] = Value::String(String::from("windows:jumplist:entry"));

        let temp = entry.clone();
        let times = extract_shortcut_times(&temp["lnk_info"])?;
        for (key, value) in times {
            entry["datetime"] = Value::String(key.into());
            entry["timestamp_desc"] = Value::String(value);
            entries.push(entry.clone());
        }
    }

    let mut has_meta = Value::Null;
    for values in data.as_array()? {
        if let Some(value) = values.get("metadata") {
            has_meta = value.clone();
        }
        break;
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

#[cfg(test)]
mod tests {
    use crate::artifacts::windows::{amcache, bits, eventlogs, jumplists, users};
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
            "created": "2024-01-01T00:00:00.000Z",
            "modified": "2024-01-01T00:00:00.000Z",
            "accessed": "2024-01-01T00:00:00.000Z",
            "path":"C:\\Windows\\cmd.exe",
        }]);

        jumplists(&mut test).unwrap();
        assert_eq!(test[0]["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test[0]["artifact"], "Jumplist");
        assert_eq!(test[0]["message"], "C:\\Windows\\cmd.exe");
    }
}
