use crate::artifacts::filter::filter_data;
use serde_json::{Value, json};
use std::collections::HashMap;

/// Timeline filelisting info
pub(crate) fn files(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
    if !data.is_object() {
        return false;
    }
    let mut entries = Vec::new();
    data["artifact"] = Value::String(String::from("Files"));
    data["data_type"] = Value::String(String::from("system:fs:file"));
    data["message"] = Value::String(data["full_path"].as_str().unwrap_or_default().into());
    let temp = json![{
        "created": data["created"].as_str().unwrap_or_default(),
        "modified": data["modified"].as_str().unwrap_or_default(),
        "accessed": data["accessed"].as_str().unwrap_or_default(),
        "changed": data["changed"].as_str().unwrap_or_default(),
    }];
    let times = extract_times(&temp).unwrap_or_default();
    for (key, value) in times {
        if filter_data(key, start, end) {
            continue;
        }
        data["datetime"] = Value::String(key.into());
        data["timestamp_desc"] = Value::String(value);
        entries.push(data.clone());
    }
    *data = Value::Array(entries);

    true
}

/// Extract each timestamp into its own separate file if required
pub(crate) fn extract_times(data: &Value) -> Option<HashMap<&str, String>> {
    let mut times = HashMap::new();
    times.insert(data["created"].as_str()?, String::from("Created"));

    if let Some(value) = times.get(data["modified"].as_str()?) {
        times.insert(data["modified"].as_str()?, format!("{value} Modified"));
    } else {
        times.insert(data["modified"].as_str()?, String::from("Modified"));
    }

    if let Some(value) = times.get(data["accessed"].as_str()?) {
        times.insert(data["accessed"].as_str()?, format!("{value} Accessed"));
    } else {
        times.insert(data["accessed"].as_str()?, String::from("Accessed"));
    }

    if let Some(value) = times.get(data["changed"].as_str()?) {
        times.insert(data["changed"].as_str()?, format!("{value} Changed"));
    } else if let Some(value) = data["changed"].as_str() {
        // Skip default empty timestamps. Mainly on Windows for Changed timestamps
        if !value.starts_with("1970") {
            times.insert(data["changed"].as_str()?, String::from("Changed"));
        }
    }

    Some(times)
}

pub(crate) fn extract_filename_times<'a>(
    data: &'a Value,
    times: &mut HashMap<&'a str, String>,
) -> Option<()> {
    if let Some(value) = times.get(data["filename_created"].as_str()?) {
        times.insert(
            data["filename_created"].as_str()?,
            format!("{value} FilenameCreated"),
        );
    } else {
        times.insert(
            data["filename_created"].as_str()?,
            String::from("FilenameCreated"),
        );
    }
    if let Some(value) = times.get(data["filename_modified"].as_str()?) {
        times.insert(
            data["filename_modified"].as_str()?,
            format!("{value} FilenameModified"),
        );
    } else {
        times.insert(
            data["filename_modified"].as_str()?,
            String::from("FilenameModified"),
        );
    }

    if let Some(value) = times.get(data["filename_accessed"].as_str()?) {
        times.insert(
            data["filename_accessed"].as_str()?,
            format!("{value} FilenameAccessed"),
        );
    } else {
        times.insert(
            data["filename_accessed"].as_str()?,
            String::from("FilenameAccessed"),
        );
    }

    if let Some(value) = times.get(data["filename_changed"].as_str()?) {
        times.insert(
            data["filename_changed"].as_str()?,
            format!("{value} FilenameChanged"),
        );
    } else {
        times.insert(
            data["filename_changed"].as_str()?,
            String::from("FilenameChanged"),
        );
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::files;
    use serde_json::json;

    #[test]
    fn test_files() {
        let mut test = json!({
            "created": "2024-01-01T00:00:00.000Z",
            "full_path": "/usr/bin/ls",
            "modified": "2024-01-01T03:00:00.000Z",
            "changed": "2024-01-01T02:00:00.000Z",
            "accessed": "2024-01-01T01:00:00.000Z",

        });

        let write_timeline = files(&mut test, &None, &None);
        assert!(write_timeline);
        assert_eq!(test.as_array().unwrap().len(), 4);
        assert_eq!(test[0]["created"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test[0]["artifact"], "Files");
        assert_eq!(test[0]["message"], "/usr/bin/ls");
    }
}
