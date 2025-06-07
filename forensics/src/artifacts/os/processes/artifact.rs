use super::{error::ProcessError, process::proc_list};
use crate::structs::{artifacts::os::processes::ProcessOptions, toml::Output};
use common::files::Hashes;
use log::warn;

/// Collect a process listing from a system
pub(crate) fn processes(
    output: &mut Output,
    filter: &bool,
    options: &ProcessOptions,
) -> Result<(), ProcessError> {
    let hashes = Hashes {
        md5: options.md5,
        sha1: options.sha1,
        sha256: options.sha256,
    };

    let results = proc_list(&hashes, &options.metadata, filter, output);
    if results.is_err() {
        warn!(
            "[core] Failed to get process list: {:?}",
            results.unwrap_err()
        );
        return Err(ProcessError::ProcessList);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::processes::artifact::processes,
        structs::{artifacts::os::processes::ProcessOptions, toml::Output},
    };

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
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
    fn test_processes() {
        let mut output = output_options("processes_test", "local", "./tmp", false);

        let proc_config = ProcessOptions {
            md5: true,
            sha1: false,
            sha256: false,
            metadata: true,
        };

        let status = processes(&mut output, &false, &proc_config).unwrap();
        assert_eq!(status, ());
    }
}
