use super::info::get_info;
use crate::{
    artifacts::os::systeminfo::error::SystemInfoError,
    output::{
        manager::OutputManager,
        record::{SingleRecordStream, serialize_to_record},
    },
};
use log::error;

/// Get basic sysinfo for a system
pub(crate) fn systeminfo(manager: &mut OutputManager) -> Result<(), SystemInfoError> {
    let entries = get_info();

    let records = match serialize_to_record(entries) {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Failed to serialize systeminfo: {err:?}");
            return Err(SystemInfoError::Serialize);
        }
    };

    let artifact_name = "systeminfo";
    if let Err(err) =
        manager.write_artifact(artifact_name, &"", &mut SingleRecordStream::new(records))
    {
        error!("[forensics] Failed to output systeminfo: {err:?}");
        return Err(SystemInfoError::Output);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use crate::{artifacts::os::systeminfo::artifact::systeminfo, output::manager::OutputManager};
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
    fn test_systeminfo() {
        let output = output_options("system_test", "./tmp", false);
        let mut manage = OutputManager::new(output).unwrap();

        let status = systeminfo(&mut manage).unwrap();
        assert_eq!(status, ());
    }
}
