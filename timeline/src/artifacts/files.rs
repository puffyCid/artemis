use serde_json::Value;
use std::collections::HashMap;

/// Timeline filelisting info
pub(crate) fn files(data: &mut Value) -> Option<()> {
    let mut entries = Vec::new();
    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };

        entry["artifact"] = Value::String(String::from("Files"));
        entry["data_type"] = Value::String(String::from("system:fs:file"));
        entry["message"] = Value::String(entry["full_path"].as_str()?.into());

        let temp = entry.clone();
        let times = extract_times(&temp)?;
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
        let mut test = json!([{
            "created": "2024-01-01T00:00:00.000Z",
            "full_path": "/usr/bin/ls",
            "modified": "2024-01-01T03:00:00.000Z",
            "changed": "2024-01-01T02:00:00.000Z",
            "accessed": "2024-01-01T01:00:00.000Z",

        }]);

        files(&mut test).unwrap();
        assert_eq!(test.as_array().unwrap().len(), 4);
        assert_eq!(test[0]["created"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test[0]["artifact"], "Files");
        assert_eq!(test[0]["message"], "/usr/bin/ls");
    }
}
