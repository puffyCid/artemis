use std::collections::HashMap;

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
use super::{error::TaskError, job::read_job, xml::parse_xml};
use crate::{
    accessor::{
        access::Accessor,
        entry::handle::{EntryKind, GlobMatch},
        location::scheme::Scheme,
    },
    artifacts::os::windows::tasks::registry::cache_info,
    filesystem::{files::get_filename, metadata::get_timestamps},
    structs::artifacts::os::windows::TasksOptions,
    utils::environment::get_systemdrive,
};
use common::windows::{Flags, TaskFormat, TaskInfo, TaskJob, TaskXml};
use tracing::{error, warn};

/// Grab Schedule Tasks based on `TaskOptions`
pub(crate) fn grab_tasks(options: &TasksOptions) -> Result<Vec<TaskInfo>, TaskError> {
    let patterns = if let Some(file) = &options.alt_file {
        vec![file.clone()]
    } else {
        let drive_result = get_systemdrive();
        let drive = match drive_result {
            Ok(result) => result,
            Err(err) => {
                error!("Could not determine systemdrive: {err:?}");
                return Err(TaskError::DriveLetter);
            }
        };

        vec![
            format!("{drive}:\\Windows\\System32\\Tasks\\**"),
            format!("{drive}:\\Windows\\Tasks\\*"),
        ]
    };

    let mut accessor = Accessor::with_defaults();
    let mut tasks = Vec::new();
    for pattern in patterns {
        let paths = match accessor.globfs(&pattern) {
            Ok(result) => result,
            Err(err) => {
                error!("Could not glob tasks {pattern}: {err:?}");
                continue;
            }
        };

        tasks.append(&mut extract_tasks(paths, options)?);
    }

    Ok(tasks)
}

/// Extract and parse Windows `Schedule Tasks`
fn extract_tasks(
    paths: Vec<GlobMatch>,
    options: &TasksOptions,
) -> Result<Vec<TaskInfo>, TaskError> {
    let mut cache = HashMap::new();
    let mut tasks = Vec::new();

    for entry in paths {
        if entry.meta.kind != EntryKind::File {
            continue;
        }

        let Some(handle) = entry.handle.as_file() else {
            continue;
        };

        // Parse XML Task files
        if !handle.display_path().ends_with(".job") && !handle.display_path().ends_with(".DAT") {
            // If running on a live Windows system. We can parse SOFTWARE Registry file for additional data
            if handle.scheme() == Scheme::Host && options.alt_file.is_none() && cache.is_empty() {
                cache = cache_info(handle.display_path().chars().next().unwrap_or_default())?;
            }

            let task_data = match parse_xml(handle) {
                Ok(result) => result,
                Err(err) => {
                    warn!(
                        "Could not parse Task File at {}: {err:?}",
                        handle.display_path()
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

            tasks.push(info);
        } else {
            let job_result = match read_job(handle) {
                Ok(result) => result,
                Err(err) => {
                    warn!(
                        "Could not parse Task Job {}: {err:?}",
                        handle.display_path()
                    );
                    continue;
                }
            };

            let info = job_info(&job_result);
            tasks.push(info);
        }
    }

    Ok(tasks)
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
    use crate::accessor::access::Accessor;
    use crate::artifacts::os::windows::tasks::parser::{
        extract_tasks, grab_tasks, job_info, xml_info,
    };
    use crate::structs::artifacts::os::windows::TasksOptions;
    use common::windows::{Actions, Priority, Status, TaskJob, TaskXml};
    use std::path::PathBuf;

    #[test]
    fn test_grab_tasks() {
        let options = TasksOptions { alt_file: None };
        let results = grab_tasks(&options).unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_extract_tasks() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/tasks/*/*");

        let mut accessor = Accessor::with_defaults();
        let paths = accessor.globfs(test_location.to_str().unwrap()).unwrap();
        let options = TasksOptions { alt_file: None };

        let results = extract_tasks(paths, &options).unwrap();
        assert!(!results.is_empty());
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
