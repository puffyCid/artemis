use super::error::ConnectionsError;
use crate::{artifacts::output::output_artifact, structs::toml::Output, utils::time};
use log::error;
use lumination::connections::connections;
use serde_json::Value;

/// Attempt to get network connections on a system
pub(crate) fn list_connections(output: &mut Output, filter: &bool) -> Result<(), ConnectionsError> {
    let start_time = time::time_now();

    let conns = match connections() {
        Ok(result) => result,
        Err(err) => {
            error!("[connections] Failed to collect network connections: {err:?}");
            return Err(ConnectionsError::ConnectionList);
        }
    };

    let serde_data_result = serde_json::to_value(conns);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[core] Failed to serialize conenctions: {err:?}");
            return Err(ConnectionsError::Serialize);
        }
    };

    let output_name = "connections";
    output_data(&mut serde_data, output_name, output, &start_time, filter)
}

/// Output connections
pub(crate) fn output_data(
    serde_data: &mut Value,
    output_name: &str,
    output: &mut Output,
    start_time: &u64,
    filter: &bool,
) -> Result<(), ConnectionsError> {
    let status = output_artifact(serde_data, output_name, output, start_time, filter);
    if status.is_err() {
        error!("[core] Could not output data: {:?}", status.unwrap_err());
        return Err(ConnectionsError::OutputData);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::list_connections;
    use crate::structs::toml::Output;

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("csv"),
            compress,
            timeline: false,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        }
    }
    #[test]
    fn test_list_connections() {
        let mut output = output_options("connections_test", "local", "./tmp", false);
        list_connections(&mut output, &false).unwrap();
    }
}
