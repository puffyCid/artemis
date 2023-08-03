/**
 * `Schedule Tasks` are a common form of Persistence on Windows systems. There are two (2) types of `Task` files:
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
use super::{error::TaskError, job::TaskJob, xml::TaskXml};
use crate::{
    filesystem::{files::list_files, metadata::glob_paths},
    structs::artifacts::os::windows::TasksOptions,
    utils::environment::get_systemdrive,
};
use log::{error, warn};
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct TaskData {
    tasks: Vec<TaskXml>,
    jobs: Vec<TaskJob>,
}

/// Grab Schedule Tasks based on `TaskOptions`
pub(crate) fn grab_tasks(options: &TasksOptions) -> Result<TaskData, TaskError> {
    if let Some(alt_drive) = options.alt_drive {
        return alt_drive_tasks(&alt_drive);
    }

    default_tasks()
}

/// Grab and parse single Task Job File at provided path
pub(crate) fn grab_task_job(path: &str) -> Result<TaskJob, TaskError> {
    TaskJob::parse_job(path)
}

/// Grab and parse single Task XML File at provided path
pub(crate) fn grab_task_xml(path: &str) -> Result<TaskXml, TaskError> {
    TaskXml::parse_xml(path)
}

/// Grab the default Tasks files. Liekly will be C:
fn default_tasks() -> Result<TaskData, TaskError> {
    let drive_result = get_systemdrive();
    let drive = match drive_result {
        Ok(result) => result,
        Err(err) => {
            error!("[tasks] Could not determine systemdrive: {err:?}");
            return Err(TaskError::DriveLetter);
        }
    };
    alt_drive_tasks(&drive)
}

/// Parse Tasks at an alternative drive
fn alt_drive_tasks(letter: &char) -> Result<TaskData, TaskError> {
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

    let mut tasks_data = TaskData {
        tasks: Vec::new(),
        jobs: Vec::new(),
    };

    for path in xml_paths {
        if !path.is_file {
            continue;
        }

        let xml_result = TaskXml::parse_xml(&path.full_path);
        match xml_result {
            Ok(result) => tasks_data.tasks.push(result),
            Err(err) => {
                warn!(
                    "[tasks] Could not parse Task File at {}: {err:?}",
                    path.full_path
                );
            }
        }
    }

    let job_path = format!("{letter}:\\Windows\\Tasks");
    let job_result = list_files(&job_path);
    let jobs = match job_result {
        Ok(result) => result,
        Err(err) => {
            error!("[tasks] Could not get Task Job files: {err:?}");
            return Err(TaskError::Jobs);
        }
    };

    for job in jobs {
        if !job.ends_with("job") {
            continue;
        }

        let job_result = TaskJob::parse_job(&job);
        match job_result {
            Ok(result) => tasks_data.jobs.push(result),
            Err(err) => {
                warn!("[tasks] Could not parse Task Job {job}: {err:?}");
            }
        }
    }

    Ok(tasks_data)
}

#[cfg(test)]
mod tests {
    use super::grab_tasks;
    use crate::artifacts::os::windows::tasks::parser::{grab_task_job, grab_task_xml};
    use crate::{
        artifacts::os::windows::tasks::parser::{alt_drive_tasks, default_tasks},
        structs::artifacts::os::windows::TasksOptions,
    };
    use std::path::PathBuf;

    #[test]
    fn test_grab_tasks() {
        let options = TasksOptions { alt_drive: None };

        let result = grab_tasks(&options).unwrap();
        assert!(result.tasks.len() > 10);
    }

    #[test]
    fn test_default_tasks() {
        let result = default_tasks().unwrap();
        assert!(result.tasks.len() > 10);
    }

    #[test]
    fn test_alt_drive_tasks() {
        let result = alt_drive_tasks(&'C').unwrap();
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
