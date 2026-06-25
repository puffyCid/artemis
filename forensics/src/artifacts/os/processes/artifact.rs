use super::{error::ProcessError, process::proc_list};
use crate::{output::manager::OutputManager, structs::artifacts::os::processes::ProcessOptions};
use tracing::warn;

/// Collect a process listing from a system
pub(crate) fn processes(
    manager: &mut OutputManager,
    options: &ProcessOptions,
) -> Result<(), ProcessError> {
    if let Err(result) = proc_list(manager, options) {
        warn!("Failed to get process list: {result:?}");
        return Err(ProcessError::ProcessList);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use crate::{
        artifacts::os::processes::artifact::processes, output::manager::OutputManager,
        structs::artifacts::os::processes::ProcessOptions,
    };
    use std::path::PathBuf;

    fn output_options(name: &str, directory: &str, compress: bool) -> OutputManager {
        let config = OutputConfig {
            name: name.to_string(),
            directory: PathBuf::from(directory),
            format: OutputFormat::Csv,
            compress,
            endpoint_id: String::from("abcd"),
            destination: OutputDestination::Local,
            ..Default::default()
        };
        OutputManager::new(config).unwrap()
    }

    #[test]
    fn test_processes() {
        let mut output = output_options("processes_test", "./tmp", false);

        let proc_config = ProcessOptions {
            md5: true,
            sha1: false,
            sha256: false,
            metadata: true,
        };

        let status = processes(&mut output, &proc_config).unwrap();
        assert_eq!(status, ());
    }
}
