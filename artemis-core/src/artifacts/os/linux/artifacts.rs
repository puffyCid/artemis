use crate::artifacts::os::files::filelisting::FileInfo;
use crate::artifacts::os::linux::error::LinuxArtifactError;
use crate::artifacts::os::processes::process::Processes;
use crate::artifacts::os::systeminfo::info::SystemInfo;
use crate::filesystem::files::Hashes;
use crate::output::formats::json::json_format;
use crate::output::formats::jsonl::jsonl_format;
use crate::runtime::deno::filter_script;
use crate::structs::artifacts::os::files::FileOptions;
use crate::structs::artifacts::os::processes::ProcessOptions;
use crate::utils::artemis_toml::Output;
use crate::utils::time;
use log::{error, warn};
use serde_json::Value;

use super::{journals::parser::grab_journal, logons::parser::grab_logons};

/// Get Linux `Processes`
pub(crate) fn processes(
    artifact: &ProcessOptions,
    output: &mut Output,
    filter: &bool,
) -> Result<(), LinuxArtifactError> {
    let start_time = time::time_now();

    let hashes = Hashes {
        md5: artifact.md5,
        sha1: artifact.sha1,
        sha256: artifact.sha256,
    };

    let results = Processes::proc_list(&hashes, artifact.metadata);
    let proc_data = match results {
        Ok(data) => data,
        Err(err) => {
            warn!("[artemis-core] Artemis Linux failed to get process list: {err:?}");
            return Err(LinuxArtifactError::Process);
        }
    };

    let serde_data_result = serde_json::to_value(proc_data);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize processes: {err:?}");
            return Err(LinuxArtifactError::Serialize);
        }
    };

    let output_name = "processes";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Get Linux `Systeminfo`
pub(crate) fn systeminfo(output: &mut Output, filter: &bool) -> Result<(), LinuxArtifactError> {
    let start_time = time::time_now();

    let system_data = SystemInfo::get_info();
    let serde_data_result = serde_json::to_value(system_data);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize system data: {err:?}");
            return Err(LinuxArtifactError::Serialize);
        }
    };

    let output_name = "systeminfo";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Get Linux `filelist`
pub(crate) fn files(
    artifact: &FileOptions,
    output: &mut Output,
    filter: &bool,
) -> Result<(), LinuxArtifactError> {
    let hashes = Hashes {
        md5: artifact.md5.unwrap_or(false),
        sha1: artifact.sha1.unwrap_or(false),
        sha256: artifact.sha256.unwrap_or(false),
    };
    let artifact_result = FileInfo::get_filelist(
        &artifact.start_path,
        artifact.depth.unwrap_or(1).into(),
        artifact.metadata.unwrap_or(false),
        &hashes,
        artifact.regex_filter.as_ref().unwrap_or(&String::new()),
        output,
        filter,
    );
    match artifact_result {
        Ok(_) => {}
        Err(err) => {
            error!("[artemis-core] Artemis Linux failed to get file listing: {err:?}");
            return Err(LinuxArtifactError::File);
        }
    };
    Ok(())
}

/// Get Linux `Journals`
pub(crate) fn journals(output: &mut Output, filter: &bool) -> Result<(), LinuxArtifactError> {
    let start_time = time::time_now();

    let artifact_result = grab_journal(output, &start_time, filter);
    match artifact_result {
        Ok(result) => Ok(result),
        Err(err) => {
            error!("[artemis-core] Artemis Linux failed to get journals: {err:?}");
            Err(LinuxArtifactError::Journal)
        }
    }
}

/// Get Linux `Logon` info
pub(crate) fn logons(output: &mut Output, filter: &bool) -> Result<(), LinuxArtifactError> {
    let start_time = time::time_now();

    let result = grab_logons();
    let serde_data_result = serde_json::to_value(result);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize logons: {err:?}");
            return Err(LinuxArtifactError::Serialize);
        }
    };

    let output_name = "logons";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Output Linux artifacts
pub(crate) fn output_data(
    serde_data: &Value,
    output_name: &str,
    output: &mut Output,
    start_time: &u64,
    filter: &bool,
) -> Result<(), LinuxArtifactError> {
    if *filter {
        if let Some(script) = &output.filter_script.clone() {
            let args = vec![serde_data.to_string(), output_name.to_string()];
            if let Some(name) = &output.filter_name.clone() {
                let filter_result = filter_script(output, &args, name, script);
                return match filter_result {
                    Ok(_) => Ok(()),
                    Err(err) => {
                        error!(
                            "[artemis-core] Could not apply filter script to linux data: {err:?}"
                        );
                        Err(LinuxArtifactError::FilterOutput)
                    }
                };
            }
            let filter_result = filter_script(output, &args, "UnknownFilterName", script);
            return match filter_result {
                Ok(_) => Ok(()),
                Err(err) => {
                    error!(
                    "[artemis-core] Could not apply unknown filter script to linux data: {err:?}"
                );
                    Err(LinuxArtifactError::FilterOutput)
                }
            };
        }
    }

    let output_status = if output.format == "json" {
        json_format(serde_data, output_name, output, start_time)
    } else if output.format == "jsonl" {
        jsonl_format(serde_data, output_name, output, start_time)
    } else {
        error!(
            "[artemis-core] Unknown formatter provided: {}",
            output.format
        );
        return Err(LinuxArtifactError::Format);
    };
    match output_status {
        Ok(_) => {}
        Err(err) => {
            error!("[artemis-core] Could not output data: {err:?}");
            return Err(LinuxArtifactError::Output);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::linux::artifacts::{
        files, journals, logons, output_data, processes, systeminfo,
    };
    use crate::structs::artifacts::os::files::FileOptions;
    use crate::structs::artifacts::os::processes::ProcessOptions;
    use crate::utils::artemis_toml::Output;
    use crate::utils::time;

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
            compress,
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
    fn test_output_data() {
        let mut output = output_options("output_test", "local", "./tmp", false);
        let start_time = time::time_now();

        let name = "test";
        let data = serde_json::Value::String(String::from("test"));
        let status = output_data(&data, name, &mut output, &start_time, &&false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_processes() {
        let mut output = output_options("processes_test", "local", "./tmp", false);

        let proc_config = ProcessOptions {
            md5: true,
            sha1: true,
            sha256: true,
            metadata: true,
        };

        let status = processes(&proc_config, &mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_system() {
        let mut output = output_options("system_test", "local", "./tmp", false);

        let status = systeminfo(&mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_journals() {
        let mut output = output_options("journals_test", "local", "./tmp", false);

        let status = journals(&mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_logons() {
        let mut output = output_options("logons_test", "local", "./tmp", false);

        let status = logons(&mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_files() {
        let mut output = output_options("file_test", "local", "./tmp", false);

        let file_config = FileOptions {
            start_path: String::from("/"),
            depth: Some(1),
            metadata: Some(false),
            md5: Some(false),
            sha1: Some(false),
            sha256: Some(false),
            regex_filter: Some(String::new()),
        };
        let status = files(&file_config, &mut output, &false).unwrap();
        assert_eq!(status, ());
    }
}
