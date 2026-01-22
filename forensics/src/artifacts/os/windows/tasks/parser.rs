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
    artifacts::os::windows::artifacts::output_data,
    filesystem::{files::list_files, metadata::glob_paths},
    structs::{artifacts::os::windows::TasksOptions, toml::Output},
    utils::{environment::get_systemdrive, time},
};
use common::windows::{TaskJob, TaskXml};
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
            return Ok(())
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
        xml_tasks.push(task_data);
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

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::grab_tasks;
    use crate::artifacts::os::windows::tasks::parser::{grab_task_job, grab_task_xml};
    use crate::{
        artifacts::os::windows::tasks::parser::drive_tasks,
        structs::artifacts::os::windows::TasksOptions,
    };
    use std::path::PathBuf;

    #[test]
    fn test_grab_tasks() {
        let options = TasksOptions { alt_file: None };

        let result = grab_tasks(&options).unwrap();
        assert!(result.tasks.len() > 10);
    }

    #[test]
    fn test_drive_tasks() {
        let result = drive_tasks('C').unwrap();
        assert!(result.tasks.len() > 10);
    }

    #[test]
    fn test_grab_task_job() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/tasks/win10/At1.job");

        let _ = grab_task_job(&test_location.display().to_string()).unwrap();
    }

    #[test]
    fn test_grab_task_xml() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/tasks/win10/VSIX Auto Update");

        let _ = grab_task_xml(&test_location.display().to_string()).unwrap();
    }
}
