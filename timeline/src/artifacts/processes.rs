use crate::artifacts::filter::filter_data;
use serde_json::Value;

/// Timeline process info
pub(crate) fn processes(data: &mut Value, start: &Option<String>, end: &Option<String>) -> bool {
    if !data.is_object() {
        return false;
    }
    let Some(start_time) = data["start_time"].as_str() else {
        return false;
    };

    if filter_data(start_time, start, end) {
        return false;
    }

    data["datetime"] = start_time.into();
    data["artifact"] = Value::String(String::from("Processes"));
    data["data_type"] = Value::String(String::from("system:processes:process"));
    data["timestamp_desc"] = Value::String(String::from("Process Start Time"));
    data["message"] = Value::String(format!(
        "{} {}",
        data["full_path"].as_str().unwrap_or_default(),
        data["arguments"].as_str().unwrap_or_default()
    ));

    true
}

/// Timeline network connections. We will not have timestamps but we can still include other stuff
pub(crate) fn network(data: &mut Value) -> bool {
    if !data.is_object() {
        return false;
    }
    data["datetime"] = Value::String(String::from("1970-01-01T00:00:00.000Z"));
    data["artifact"] = Value::String(String::from("Connections"));
    data["data_type"] = Value::String(String::from("system:network:connection"));
    data["timestamp_desc"] = Value::String(String::from("N/A"));
    data["message"] = Value::String(format!(
        "Local: {}:{} Remote: {}:{} State: {}",
        data["local_address"].as_str().unwrap_or_default(),
        data["local_port"].as_i64().unwrap_or_default(),
        data["remote_address"].as_str().unwrap_or_default(),
        data["remote_port"].as_i64().unwrap_or_default(),
        data["state"].as_str().unwrap_or_default(),
    ));

    true
}

#[cfg(test)]
mod tests {
    use super::processes;
    use crate::artifacts::processes::network;
    use serde_json::json;

    #[test]
    fn test_processes() {
        let mut test = json!({
            "start_time": "2024-01-01T00:00:00.000Z",
            "full_path": "/usr/bin/ls",
            "arguments":" stuff",
            "binary_info": [{"data":"data1"}]
        });

        let write_timeline = processes(&mut test, &None, &None);
        assert!(write_timeline);
        assert_eq!(test["datetime"], "2024-01-01T00:00:00.000Z");
        assert_eq!(test["artifact"], "Processes");
        assert_eq!(test["message"], "/usr/bin/ls  stuff");
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

        let write_timeline = network(&mut test);
        assert!(write_timeline);
        assert_eq!(test["artifact"], "Connections");
        assert_eq!(
            test["message"].as_str().unwrap(),
            "Local: :::9600 Remote: :::0 State: Listen"
        );
    }
}
