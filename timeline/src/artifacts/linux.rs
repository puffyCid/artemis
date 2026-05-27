use crate::artifacts::{files::extract_times, filter::filter_data};
use serde_json::{Value, json};

/// Timeline Journal files
pub(crate) fn journal(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
    if !data.is_object() {
        return false;
    }
    let Some(realtime) = data["realtime"].as_str() else {
        return false;
    };

    if filter_data(realtime, start, end) {
        return false;
    }
    data["datetime"] = realtime.into();
    data["artifact"] = "Journals".into();
    data["data_type"] = "linux:journals:entry".into();
    data["timestamp_desc"] = "Journal Entry Generated".into();

    true
}

/// Timeline sudo entries in Journal files
pub(crate) fn sudo_linux(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
    if !data.is_object() {
        return false;
    }
    let Some(realtime) = data["realtime"].as_str() else {
        return false;
    };

    if filter_data(realtime, start, end) {
        return false;
    }
    data["datetime"] = realtime.into();
    data["artifact"] = "Sudo Linux".into();
    data["data_type"] = "linux:journals:sudo:entry".into();
    data["timestamp_desc"] = "Sudo Journal Entry Generated".into();

    true
}

/// Timeline Linux logons
pub(crate) fn logons(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
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
    data["artifact"] = "Logon Linux".into();
    data["data_type"] = "linux:logons:entry".into();
    data["timestamp_desc"] = "Logon Event".into();
    data["message"] = Value::String(format!(
        "User: {} - Logon: {}",
        data["username"].as_str().unwrap_or_default(),
        data["status"].as_str().unwrap_or_default()
    ));

    true
}

pub(crate) fn ext4_filelisting(
    data: &mut Value,
    start: &Option<String>,
    end: &Option<String>,
) -> bool {
    if !data.is_object() {
        return false;
    }
    let mut entries = Vec::new();
    data["artifact"] = "RawFilesExt4".into();
    data["data_type"] = "linux:ext4:file".into();
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
    if entries.is_empty() {
        return false;
    }

    *data = Value::Array(entries);

    true
}

#[cfg(test)]
mod tests {
    use super::{journal, sudo_linux};
    use crate::artifacts::linux::{ext4_filelisting, logons};
    use serde_json::json;

    #[test]
    fn test_journal() {
        let mut test = json!({
            "realtime": "2024-01-01T00:00:00.000Z",
            "message": "my log",
            "data1":"anything i want"
        });

        assert!(journal(&mut test, &None, &None));
        assert_eq!(test["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test["artifact"], "Journals");
        assert_eq!(test["data1"], "anything i want");
    }

    #[test]
    fn test_sudo_linux() {
        let mut test = json!({
            "realtime": "2024-01-01T00:00:00.000Z",
            "message": "my log",
            "data1":"anything i want"
        });

        assert!(sudo_linux(&mut test, &None, &None));
        assert_eq!(test["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test["artifact"], "Sudo Linux");
        assert_eq!(test["data1"], "anything i want");
    }

    #[test]
    fn test_logons() {
        let mut test = json!({
            "timestamp": "2024-01-01T00:00:00.000Z",
            "message": "my log",
            "data1":"anything i want",
            "username": "bob",
            "status": "Success",
        });

        assert!(logons(&mut test, &None, &None));
        assert_eq!(test["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test["artifact"], "Logon Linux");
        assert_eq!(test["message"], "User: bob - Logon: Success");
    }

    #[test]
    fn test_ext4_filelisting() {
        let mut test = json!({
            "full_path": "/boot/.vmlinuz-6.17.5-200.fc42.x86_64.hmac",
            "directory": "/boot",
            "filename": ".vmlinuz-6.17.5-200.fc42.x86_64.hmac",
            "extension": "hmac",
            "created": "2025-11-01T17:46:20.402127476Z",
            "modified": "2025-10-23T00:00:00.000000000Z",
            "changed": "2025-11-01T17:46:20.402893753Z",
            "accessed": "2025-10-23T00:00:00.000000000Z",
        });

        assert!(ext4_filelisting(&mut test, &None, &None));
        assert_eq!(test.as_array().unwrap().len(), 3);
        assert_eq!(
            test[0]["message"].as_str().unwrap(),
            "/boot/.vmlinuz-6.17.5-200.fc42.x86_64.hmac"
        );
    }
}
