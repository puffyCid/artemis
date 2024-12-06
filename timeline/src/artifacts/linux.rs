use serde_json::Value;

/// Timeline Journal files
pub(crate) fn journal(mut data: Value) -> Option<Value> {
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

    Some(data)
}

/// Timeline sudo entries in Journal files
pub(crate) fn sudo_linux(mut data: Value) -> Option<Value> {
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

    Some(data)
}

/// Timeline Linux logons
pub(crate) fn logons(mut data: Value) -> Option<Value> {
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

    Some(data)
}

#[cfg(test)]
mod tests {
    use super::{journal, sudo_linux};
    use crate::artifacts::linux::logons;
    use serde_json::json;

    #[test]
    fn test_journal() {
        let test = json!([{
            "realtime": "2024-01-01T00:00:00.000Z",
            "message": "my log",
            "data1":"anything i want"
        }]);

        let result = journal(test).unwrap();
        assert_eq!(result[0]["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(result[0]["artifact"], "Journals");
        assert_eq!(result[0]["data1"], "anything i want");
    }

    #[test]
    fn test_sudo_linux() {
        let test = json!([{
            "realtime": "2024-01-01T00:00:00.000Z",
            "message": "my log",
            "data1":"anything i want"
        }]);

        let result = sudo_linux(test).unwrap();
        assert_eq!(result[0]["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(result[0]["artifact"], "Sudo Linux");
        assert_eq!(result[0]["data1"], "anything i want");
    }

    #[test]
    fn test_logons() {
        let test = json!([{
            "timestamp": "2024-01-01T00:00:00.000Z",
            "message": "my log",
            "data1":"anything i want",
            "username": "bob",
            "status": "Success",
        }]);

        let result = logons(test).unwrap();
        assert_eq!(result[0]["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(result[0]["artifact"], "Logon Linux");
        assert_eq!(result[0]["message"], "User: bob - Logon: Success");
    }
}
