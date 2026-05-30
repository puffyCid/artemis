use super::error::ConnectionsError;
use crate::output2::{manager::OutputManager, record::serialize_records_to_stream};
use log::error;
use lumination::connections::connections;

/// Attempt to get network connections on a system
pub(crate) fn list_connections(manager: &mut OutputManager) -> Result<(), ConnectionsError> {
    let entries = match connections() {
        Ok(result) => result,
        Err(err) => {
            error!("[connections] Failed to collect network connections: {err:?}");
            return Err(ConnectionsError::ConnectionList);
        }
    };

    let mut records = match serialize_records_to_stream(entries) {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Failed to serialize connections: {err:?}");
            return Err(ConnectionsError::Serialize);
        }
    };

    let artifact_name = "connections";
    if let Err(err) = manager.write_artifact(artifact_name, &"", &mut records) {
        error!("[forensics] Failed to output connections: {err:?}");
        return Err(ConnectionsError::Output);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::list_connections;
    use crate::output2::{
        config::{OutputConfig, OutputDestination, OutputFormat},
        manager::OutputManager,
    };
    use std::path::PathBuf;

    fn output_options(name: &str, directory: &str, compress: bool) -> OutputConfig {
        OutputConfig {
            name: name.to_string(),
            directory: PathBuf::from(directory),
            format: OutputFormat::Csv,
            compress,
            endpoint_id: String::from("abcd"),
            destination: OutputDestination::Local,
            ..Default::default()
        }
    }
    #[test]
    fn test_list_connections() {
        let output = output_options("connections_test", "./tmp", false);
        let mut manage = OutputManager::new(output).unwrap();

        list_connections(&mut manage).unwrap();
    }
}
