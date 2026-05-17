use super::{error::ProcessError, process::proc_list};
use crate::{
    output2::manager::OutputManager,
    structs::{artifacts::os::processes::ProcessOptions, toml::Output},
};
use common::files::Hashes;
use log::warn;

/// Collect a process listing from a system
pub(crate) fn processes(
    output: &mut OutputManager,
    filter: bool,
    options: &ProcessOptions,
) -> Result<(), ProcessError> {
    let hashes = Hashes {
        md5: options.md5,
        sha1: options.sha1,
        sha256: options.sha256,
    };

    if let Err(result) = proc_list(&hashes, options.metadata, filter, output) {
        warn!("[forensics] Failed to get process list: {result:?}");
        return Err(ProcessError::ProcessList);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::processes::artifact::processes,
        output2::{config::OutputConfig, manager::OutputManager},
        structs::{artifacts::os::processes::ProcessOptions, toml::Output},
        utils::time::time_now,
    };

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
            compress,
            endpoint_id: String::from("abcd"),
            output: output.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_processes() {
        let mut output = output_options("processes_test", "local", "./tmp", false);

        let proc_config = ProcessOptions {
            md5: true,
            sha1: false,
            sha256: false,
            metadata: true,
        };
        let config = OutputConfig::try_from(output).unwrap();
        let mut manage = OutputManager::new(config).unwrap();

        let status = processes(&mut manage, false, &proc_config).unwrap();
        assert_eq!(status, ());
    }
}
