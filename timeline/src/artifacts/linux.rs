use serde_json::Value;

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

#[cfg(test)]
mod tests {
    use super::{journal, sudo_linux};
    use crate::artifacts::linux::logons;
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
}
