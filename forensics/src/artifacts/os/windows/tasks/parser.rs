/**
 * `Schedule Tasks` are a common form of persistence on Windows systems. There are two (2) types of `Task` files:
 *   - XML based `Task` files
 *   - Job based `Task` files
 *
 * Starting on Windows Vista and higher XML files are used for `Schedule Tasks`.
 *
 * References:
 * `https://github.com/libyal/dtformats/blob/main/documentation/Job%20file%20format.asciidoc`
 * `https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-tsch/0d6383e4-de92-43e7-b0bb-a60cfa36379f`
 *
 * Other Parsers:
 *  Any XML reader
 * `https://github.com/Velocidex/velociraptor`
 */
use super::{error::TaskError, job::parse_job, xml::parse_xml};
use crate::{
    artifacts::os::windows::{artifacts::output_data, tasks::registry::cache_info},
    filesystem::{
        files::{get_filename, list_files},
        metadata::glob_paths,
    },
    structs::{artifacts::os::windows::TasksOptions, toml::Output},
    utils::{environment::get_systemdrive, time},
};
use common::windows::{TaskFormat, TaskInfo, TaskJob, TaskXml};
use log::{error, warn};
use serde_json::Value;

/// Grab Schedule Tasks based on `TaskOptions`
pub(crate) fn grab_tasks(
    options: &TasksOptions,
    output: &mut Output,
    filter: bool,
) -> Result<(), TaskError> {
    let start_time = time::time_now();
    if let Some(file) = &options.alt_file {
        if file.ends_with(".job") {
            let result = grab_task_job(file)?;
            let mut serde_data = match serde_json::to_value(&result) {
                Ok(result) => result,
                Err(err) => {
                    error!("[tasks] Failed to serialize job: {err:?}");
                    return Err(TaskError::Serialize);
                }
            };
            output_tasks(&mut serde_data, output, filter, start_time);
            return Ok(());
        }
        let result = grab_task_xml(file)?;
        let mut serde_data = match serde_json::to_value(&result) {
            Ok(result) => result,
            Err(err) => {
                error!("[tasks] Failed to serialize task: {err:?}");
                return Err(TaskError::Serialize);
            }
        };
        output_tasks(&mut serde_data, output, filter, start_time);

        return Ok(());
    }

    let drive_result = get_systemdrive();
    let drive = match drive_result {
        Ok(result) => result,
        Err(err) => {
            error!("[tasks] Could not determine systemdrive: {err:?}");
            return Err(TaskError::DriveLetter);
        }
    };

    drive_tasks(drive, output, filter, start_time)
}

/// Grab and parse single Task Job File at provided path
fn grab_task_job(path: &str) -> Result<TaskJob, TaskError> {
    parse_job(path)
}

/// Grab and parse single Task XML File at provided path
pub(crate) fn grab_task_xml(path: &str) -> Result<TaskXml, TaskError> {
    parse_xml(path)
}

/// Parse Tasks at provided drive
fn drive_tasks(
    letter: char,
    output: &mut Output,
    filter: bool,
    start_time: u64,
) -> Result<(), TaskError> {
    let path = format!("{letter}:\\Windows\\System32\\Tasks");
    // Tasks may be under nested directories. Glob everything at path
    let paths_result = glob_paths(&format!("{path}\\**\\*"));
    let xml_paths = match paths_result {
        Ok(result) => result,
        Err(err) => {
            error!("[tasks] Could not glob Task XML files: {err:?}");
            return Err(TaskError::Glob);
        }
    };

    let mut xml_tasks = Vec::new();
    let cache = cache_info(letter)?;

    for path in xml_paths {
        if !path.is_file {
            continue;
        }

        let task_data = match parse_xml(&path.full_path) {
            Ok(result) => result,
            Err(err) => {
                warn!(
                    "[tasks] Could not parse Task File at {}: {err:?}",
                    path.full_path
                );
                continue;
            }
        };

        let mut info = task_info(&task_data);
        info.evidence = path.full_path;
        if let Ok(result) = serde_json::to_value(&task_data) {
            info.details = result;
        }
        if let Some(value) = cache.get(&info.path.to_lowercase()) {
            info.id = value.id.clone();
            info.last_error_code = value.last_error_code;
            info.last_run = value.last_run.clone();
            info.created = value.created.clone();
            info.last_successful_run = value.last_successful_run.clone();
            info.registry_file = value.registry_file.clone();
            info.registry_task_path = value.registry_task_path.clone();
            info.registry_tree_path = value.registry_tree_path.clone();
            info.security_descriptor = value.security_description.clone();
        }

        xml_tasks.push(info);
    }

    let mut serde_data = match serde_json::to_value(&xml_tasks) {
        Ok(result) => result,
        Err(err) => {
            error!("[tasks] Failed to serialize tasks: {err:?}");
            return Err(TaskError::Serialize);
        }
    };
    output_tasks(&mut serde_data, output, filter, start_time);

    // Legacy Task path associated with Job files
    let job_path = format!("{letter}:\\Windows\\Tasks");
    let job_result = list_files(&job_path);
    let jobs = match job_result {
        Ok(result) => result,
        Err(err) => {
            error!("[tasks] Could not get Task Job files: {err:?}");
            return Err(TaskError::Jobs);
        }
    };

    let mut job_tasks = Vec::new();

    for job in jobs {
        if !job.ends_with("job") {
            continue;
        }

        let job_result = match parse_job(&job) {
            Ok(result) => result,
            Err(err) => {
                warn!("[tasks] Could not parse Task Job {job}: {err:?}");
                continue;
            }
        };

        job_tasks.push(job_result);
    }

    // Job schedule tasks are legacy format. May not exist
    if job_tasks.is_empty() {
        return Ok(());
    }

    let mut serde_data = match serde_json::to_value(&job_tasks) {
        Ok(result) => result,
        Err(err) => {
            error!("[tasks] Failed to serialize jobs: {err:?}");
            return Err(TaskError::Serialize);
        }
    };
    output_tasks(&mut serde_data, output, filter, start_time);

    Ok(())
}

/// Output Schedule tasks artifacts
fn output_tasks(result: &mut Value, output: &mut Output, filter: bool, start_time: u64) {
    if let Err(err) = output_data(result, "tasks", output, start_time, filter) {
        error!("[tasks] Could not output Schedule Tasks data: {err:?}");
    }
}

fn task_info(xml: &TaskXml) -> TaskInfo {
    let mut info = TaskInfo::default();
    if let Some(value) = &xml.registration_info {
        info.path = value.uri.as_ref().unwrap_or(&String::new()).clone();
        if !info.path.starts_with("\\") {
            info.path = format!("\\{}", info.path);
        }
        info.description = value.description.as_ref().unwrap_or(&String::new()).clone();
        info.name = get_filename(&info.path);
    }

    if let Some(value) = xml.actions.exec.first() {
        let args = value.arguments.as_ref().unwrap_or(&String::new()).clone();
        info.action = format!("{} {args}", value.command.replace('"', ""),);
    }
    if let Some(value) = &xml.settings {
        info.hidden = value.hidden.unwrap_or_default();
        info.enabled = value.enabled.unwrap_or_default();
    }

    info.format = TaskFormat::Xml;

    info
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::grab_tasks;
    use crate::artifacts::os::windows::tasks::parser::{grab_task_job, grab_task_xml};
    use crate::structs::toml::Output;
    use crate::{
        artifacts::os::windows::tasks::parser::drive_tasks,
        structs::artifacts::os::windows::TasksOptions,
    };
    use std::path::PathBuf;

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
    fn test_grab_tasks() {
        let options = TasksOptions { alt_file: None };
        let mut output = output_options("tasks_temp", "local", "./tmp", false);

        grab_tasks(&options, &mut output, false).unwrap();
    }

    #[test]
    fn test_drive_tasks() {
        let mut output = output_options("tasks_temp", "local", "./tmp", false);

        drive_tasks('C', &mut output, false, 0).unwrap();
    }

    #[test]
    fn test_grab_task_job() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/tasks/win10/At1.job");

        let result = grab_task_job(&test_location.display().to_string()).unwrap();
        assert_eq!(result.parameters, "");
    }

    #[test]
    fn test_grab_task_xml() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/tasks/win10/VSIX Auto Update");

        let result = grab_task_xml(&test_location.display().to_string()).unwrap();
        assert_eq!(result.actions.exec.len(), 1);
    }
}
