use crate::artifacts::filter::filter_data;
use serde_json::Value;

/// Timeline process info
pub(crate) fn processes(
    data: &mut Value,
    start: &Option<String>,
    end: &Option<String>,
) -> Option<()> {
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
        entry["artifact"] = Value::String(String::from("Processes"));
        entry["data_type"] = Value::String(String::from("system:processes:process"));
        entry["timestamp_desc"] = Value::String(String::from("Process Start Time"));
        entry["message"] = Value::String(format!(
            "{} {}",
            entry["full_path"].as_str().unwrap_or_default(),
            entry["arguments"].as_str().unwrap_or_default()
        ));
        !filter_data(entry["datetime"].as_str().unwrap(), start, end)
    });

    Some(())
}

/// Timeline network connections. We will not have timestamps but we can still include other stuff
pub(crate) fn network(
    data: &mut Value,
    start: &Option<String>,
    end: &Option<String>,
) -> Option<()> {
    data.as_array_mut()?.retain_mut(|entry| {
        if !entry.is_object() {
            // Drop value if its not an object
            return false;
        }
        entry["datetime"] = Value::String(String::from("1970-01-01T00:00:00.000Z"));
        entry["artifact"] = Value::String(String::from("Connections"));
        entry["data_type"] = Value::String(String::from("system:network:connection"));
        entry["timestamp_desc"] = Value::String(String::from("N/A"));
        entry["message"] = Value::String(format!(
            "Local: {}:{} Remote: {}:{} State: {}",
            entry["local_address"].as_str().unwrap_or_default(),
            entry["local_port"].as_i64().unwrap_or_default(),
            entry["remote_address"].as_str().unwrap_or_default(),
            entry["remote_port"].as_i64().unwrap_or_default(),
            entry["state"].as_str().unwrap_or_default(),
        ));
        !filter_data(entry["datetime"].as_str().unwrap(), start, end)
    });

    Some(())
}

#[cfg(test)]
mod tests {
    use super::processes;
    use crate::artifacts::processes::network;
    use serde_json::json;

    #[test]
    fn test_processes() {
        let mut test = json!([{
            "start_time": "2024-01-01T00:00:00.000Z",
            "full_path": "/usr/bin/ls",
            "arguments":" stuff",
            "binary_info": [{"data":"data1"}]
        }]);

        processes(&mut test, &None, &None).unwrap();
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

        network(&mut test, &None, &None).unwrap();
        assert_eq!(test[0]["artifact"], "Connections");
        assert_eq!(
            test[0]["message"].as_str().unwrap(),
            "Local: :::9600 Remote: :::0 State: Listen"
        );
    }
}
