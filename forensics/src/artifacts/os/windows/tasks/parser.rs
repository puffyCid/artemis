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
        metadata::{get_timestamps, glob_paths},
    },
    structs::{artifacts::os::windows::TasksOptions, toml::Output},
    utils::{environment::get_systemdrive, time},
};
use common::windows::{Flags, TaskFormat, TaskInfo, TaskJob, TaskXml};
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
            let mut serde_data = match serde_json::to_value(vec![result]) {
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
        let mut serde_data = match serde_json::to_value(vec![result]) {
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
fn grab_task_job(path: &str) -> Result<TaskInfo, TaskError> {
    let job = parse_job(path)?;
    Ok(job_info(&job))
}

/// Grab and parse single Task XML File at provided path
pub(crate) fn grab_task_xml(path: &str) -> Result<TaskInfo, TaskError> {
    let xml = parse_xml(path)?;
    Ok(xml_info(&xml))
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

        let mut info = xml_info(&task_data);

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

        let info = job_info(&job_result);
        job_tasks.push(info);
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

/// Convert `TaskXml` to `TaskInfo`
fn xml_info(xml: &TaskXml) -> TaskInfo {
    let mut info = TaskInfo {
        format: TaskFormat::Xml,
        evidence: xml.evidence.clone(),
        ..Default::default()
    };
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
        info.action = format!("{} {args}", value.command.replace('"', ""))
            .trim()
            .to_string();
        info.action_count = xml.actions.exec.len() as u8;
    }
    if info.action.is_empty()
        && let Some(value) = xml.actions.com_handler.first()
    {
        let data = value.data.as_ref().unwrap_or(&String::new()).clone();
        info.action = format!("{} {data}", value.class_id.replace('"', ""))
            .trim()
            .to_string();
        info.action_count = xml.actions.com_handler.len() as u8;
    }
    if let Some(value) = &xml.settings {
        info.hidden = value.hidden.unwrap_or_default();
        info.enabled = value.enabled.unwrap_or_default();
    }

    if let Ok(result) = serde_json::to_value(xml) {
        info.details = result;
    }

    info
}

/// Convert `TaskJob` to `TaskInfo`
fn job_info(job: &TaskJob) -> TaskInfo {
    let command = format!("{} {}", job.application_name, job.parameters)
        .trim()
        .to_string();
    let mut info = TaskInfo {
        format: TaskFormat::Job,
        id: job.job_id.clone(),
        action: command,
        enabled: !job.flags.contains(&Flags::Disabled),
        hidden: job.flags.contains(&Flags::Hidden),
        description: job.comments.clone(),
        name: get_filename(&job.evidence),
        // Job file format does not have a URI path
        // But for consistency we will use the path to the Job file
        path: job.evidence.clone(),
        evidence: job.evidence.clone(),
        ..Default::default()
    };

    // Disadvantage of this is that if we parse an Job file that was copied to another system
    // The timestamp will be not helpful
    // But there are many scenarios where a user will be parsing a Job file on the original system
    if let Ok(value) = get_timestamps(&job.evidence) {
        info.created = value.created;
    }
    if let Ok(value) = serde_json::to_value(job) {
        info.details = value;
    }

    info
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::grab_tasks;
    use crate::artifacts::os::windows::tasks::parser::{
        grab_task_job, grab_task_xml, job_info, xml_info,
    };
    use crate::structs::toml::Output;
    use crate::{
        artifacts::os::windows::tasks::parser::drive_tasks,
        structs::artifacts::os::windows::TasksOptions,
    };
    use common::windows::{Actions, Priority, Status, TaskJob, TaskXml};
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
        assert_eq!(result.action, "cmd.exe");
        assert!(result.enabled);
        assert!(!result.details.to_string().contains(&"Disabled"))
    }

    #[test]
    fn test_grab_task_xml() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/tasks/win10/VSIX Auto Update");

        let result = grab_task_xml(&test_location.display().to_string()).unwrap();
        assert_eq!(
            result.action,
            "C:\\Program Files (x86)\\Microsoft Visual Studio\\Installer\\resources\\app\\ServiceHub\\Services\\Microsoft.VisualStudio.Setup.Service\\VSIXAutoUpdate.exe"
        );
    }

    #[test]
    fn test_xml_info() {
        let xml = TaskXml {
            registration_info: None,
            triggers: None,
            settings: None,
            data: None,
            principals: None,
            actions: Actions {
                exec: Vec::new(),
                com_handler: Vec::new(),
                send_email: Vec::new(),
                show_message: Vec::new(),
            },
            evidence: String::from("none"),
        };
        let info = xml_info(&xml);
        assert_eq!(info.evidence, "none");
    }

    #[test]
    fn test_job_info() {
        let job = TaskJob {
            evidence: String::from("none"),
            job_id: String::new(),
            error_retry_count: 0,
            error_retry_interval: 0,
            idle_deadline: 0,
            idle_wait: 0,
            priority: Priority::Unknown,
            max_run_time: 0,
            exit_code: 0,
            status: Status::Unknown,
            flags: Vec::new(),
            system_time: String::new(),
            running_instance_count: 0,
            application_name: String::new(),
            parameters: String::new(),
            working_directory: String::new(),
            author: String::new(),
            comments: String::new(),
            user_data: String::new(),
            start_error: 0,
            triggers: Vec::new(),
        };
        let info = job_info(&job);
        assert_eq!(info.evidence, "none");
    }
}
