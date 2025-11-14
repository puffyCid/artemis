use serde_json::{Value, json};

use crate::artifacts::{files::extract_times, meta::check_meta};

/// Timeline Journal files
pub(crate) fn journal(data: &mut Value) -> Option<()> {
    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };
        entry["datetime"] = entry["realtime"].as_str()?.into();
        entry["artifact"] = Value::String(String::from("Journals"));
        entry["data_type"] = Value::String(String::from("linux:journals:entry"));
        entry["timestamp_desc"] = Value::String(String::from("Journal Entry Generated"));
    }

    Some(())
}

/// Timeline sudo entries in Journal files
pub(crate) fn sudo_linux(data: &mut Value) -> Option<()> {
    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };
        entry["datetime"] = entry["realtime"].as_str()?.into();
        entry["artifact"] = Value::String(String::from("Sudo Linux"));
        entry["data_type"] = Value::String(String::from("linux:journals:sudo:entry"));
        entry["timestamp_desc"] = Value::String(String::from("Sudo Journal Entry Generated"));
    }

    Some(())
}

/// Timeline Linux logons
pub(crate) fn logons(data: &mut Value) -> Option<()> {
    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };
        entry["datetime"] = entry["timestamp"].as_str()?.into();
        entry["artifact"] = Value::String(String::from("Logon Linux"));
        entry["data_type"] = Value::String(String::from("linux:logons:entry"));
        entry["timestamp_desc"] = Value::String(String::from("Logon Event"));
        entry["message"] = Value::String(format!(
            "User: {} - Logon: {}",
            entry["username"].as_str()?,
            entry["status"].as_str()?
        ));
    }

    Some(())
}

pub(crate) fn ext4_filelisting(data: &mut Value) -> Option<()> {
    let mut entries = Vec::new();
    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };

        entry["artifact"] = Value::String(String::from("RawFilesExt4"));
        entry["data_type"] = Value::String(String::from("linux:ext4:file"));
        entry["message"] = Value::String(entry["full_path"].as_str()?.into());

        let temp = json![{
            "created": entry["created"].as_str()?,
            "modified": entry["modified"].as_str()?,
            "accessed": entry["accessed"].as_str()?,
            "changed": entry["changed"].as_str()?,
        }];
        let times = extract_times(&temp)?;
        for (key, value) in times {
            entry["datetime"] = Value::String(key.into());
            entry["timestamp_desc"] = Value::String(value);
            entries.push(entry.clone());
        }
    }

    check_meta(data, &mut entries)
}

#[cfg(test)]
mod tests {
    use super::{journal, sudo_linux};
    use crate::artifacts::linux::{ext4_filelisting, logons};
    use serde_json::json;

    #[test]
    fn test_journal() {
        let mut test = json!([{
            "realtime": "2024-01-01T00:00:00.000Z",
            "message": "my log",
            "data1":"anything i want"
        }]);

        journal(&mut test).unwrap();
        assert_eq!(test[0]["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test[0]["artifact"], "Journals");
        assert_eq!(test[0]["data1"], "anything i want");
    }

    #[test]
    fn test_sudo_linux() {
        let mut test = json!([{
            "realtime": "2024-01-01T00:00:00.000Z",
            "message": "my log",
            "data1":"anything i want"
        }]);

        sudo_linux(&mut test).unwrap();
        assert_eq!(test[0]["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test[0]["artifact"], "Sudo Linux");
        assert_eq!(test[0]["data1"], "anything i want");
    }

    #[test]
    fn test_logons() {
        let mut test = json!([{
            "timestamp": "2024-01-01T00:00:00.000Z",
            "message": "my log",
            "data1":"anything i want",
            "username": "bob",
            "status": "Success",
        }]);

        logons(&mut test).unwrap();
        assert_eq!(test[0]["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test[0]["artifact"], "Logon Linux");
        assert_eq!(test[0]["message"], "User: bob - Logon: Success");
    }

    #[test]
    fn test_ext4_filelisting() {
        let mut test = json!([{
            "full_path": "/boot/.vmlinuz-6.17.5-200.fc42.x86_64.hmac",
            "directory": "/boot",
            "filename": ".vmlinuz-6.17.5-200.fc42.x86_64.hmac",
            "extension": "hmac",
            "created": "2025-11-01T17:46:20.402127476Z",
            "modified": "2025-10-23T00:00:00.000000000Z",
            "changed": "2025-11-01T17:46:20.402893753Z",
            "accessed": "2025-10-23T00:00:00.000000000Z",
        }]);

        ext4_filelisting(&mut test).unwrap();
        assert_eq!(test.as_array().unwrap().len(), 3);
        assert_eq!(
            test[0]["message"].as_str().unwrap(),
            "/boot/.vmlinuz-6.17.5-200.fc42.x86_64.hmac"
        );
    }
}
