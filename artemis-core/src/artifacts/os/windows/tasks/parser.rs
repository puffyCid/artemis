use super::{error::TaskError, xml::TaskXml};
use crate::{
    filesystem::metadata::glob_paths, structs::artifacts::os::windows::TasksOptions,
    utils::environment::get_systemdrive,
};
use log::{error, warn};
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct TaskData {
    tasks: Vec<TaskXml>,
    jobs: Vec<String>,
}

/// Grab Schedule Tasks based on `TaskOptions`
pub(crate) fn grab_tasks(options: &TasksOptions) -> Result<TaskData, TaskError> {
    if let Some(alt_drive) = options.alt_drive {
        return alt_drive_tasks(&alt_drive);
    }

    default_tasks()
}

/// Grab and parse single Task file at provided path
pub(crate) fn grab_custom_tasks(path: &str) -> Result<(), TaskError> {
    Ok(())
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

    Ok(tasks_data)
}

#[cfg(test)]
mod tests {}
