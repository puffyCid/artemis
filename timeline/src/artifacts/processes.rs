use serde_json::Value;

/// Timeline process info
pub(crate) fn processes(data: &mut Value) -> Option<()> {
    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };
        entry["datetime"] = entry["start_time"].as_str()?.into();
        entry["artifact"] = Value::String(String::from("Processes"));
        entry["data_type"] = Value::String(String::from("system:processes:process"));
        entry["timestamp_desc"] = Value::String(String::from("Process Start Time"));
        entry["message"] = Value::String(format!(
            "{} {}",
            entry["full_path"].as_str()?,
            entry["arguments"].as_str()?
        ));
    }

    Some(())
}

/// Timneline network connections. We will not have timestamps but we can still include other stuff
pub(crate) fn network(data: &mut Value) -> Option<()> {
    for values in data.as_array_mut()? {
        let entry = if let Some(value) = values.get_mut("data") {
            value
        } else {
            values
        };
        entry["datetime"] = Value::String(String::from("1970-01-01T00:00:00.000Z"));
        entry["artifact"] = Value::String(String::from("Connections"));
        entry["data_type"] = Value::String(String::from("system:network:connection"));
        entry["timestamp_desc"] = Value::String(String::from("N/A"));
        entry["message"] = Value::String(format!(
            "Local: {}:{} Remote: {}:{} State: {}",
            entry["local_address"].as_str()?,
            entry["local_port"].as_i64()?,
            entry["remote_address"].as_str()?,
            entry["remote_port"].as_i64()?,
            entry["state"].as_str()?,
        ));
    }

    Some(())
}

#[cfg(test)]
mod tests {
    use crate::artifacts::processes::network;

    use super::processes;
    use serde_json::json;

    #[test]
    fn test_processes() {
        let mut test = json!([{
            "start_time": "2024-01-01T00:00:00.000Z",
            "full_path": "/usr/bin/ls",
            "arguments":" stuff",
            "binary_info": [{"data":"data1"}]
        }]);

        processes(&mut test).unwrap();
        assert_eq!(test[0]["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test[0]["artifact"], "Processes");
        assert_eq!(test[0]["message"], "/usr/bin/ls  stuff");
    }

    #[test]
    fn test_network() {
        let mut test = json!([{
            "protocol": "Tcp",
            "local_address": "::",
            "local_port": 9600,
            "remote_address": "::",
            "remote_port": 0,
            "state": "Listen",
            "pid": 1529358,
            "process_name": "pasta.avx2",
        }]);

        network(&mut test).unwrap();
        assert_eq!(test[0]["artifact"], "Connections");
        assert_eq!(
            test[0]["message"].as_str().unwrap(),
            "Local: :::9600 Remote: :::0 State: Listen"
        );
    }
}
