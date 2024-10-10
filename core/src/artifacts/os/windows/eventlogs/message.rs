use super::error::EventLogsError;
use crate::{
    artifacts::os::windows::{
        pe::resources::{read_eventlog_resource, EventLogResource, ResourceType},
        registry::helper::get_registry_keys,
    },
    filesystem::{
        directory::get_parent_directory,
        files::{get_filename, is_file},
    },
    utils::{
        environment::{get_env, get_systemdrive},
        regex_options::create_regex,
    },
};
use log::error;
use std::collections::{HashMap, HashSet};

/*
 * TODO:
 * 3. Parse each one:
 *    - MUI: Done
 *    - Wevt_template - Done
 *    - MessageTable: Done
 * 4. Map event log data to message string
 * 5. Provide optional directory containing DLL and other files
 */

pub(crate) fn get_resources() -> Result<(), EventLogsError> {
    let (message_resources, parameter_resources) = gather_resource_paths()?;
    for message in message_resources {
        if message.resource != ResourceType::WevtTemplate {
            continue;
        }
        parse_resource(&message)?;
        break;
    }
    Ok(())
}

pub(crate) fn parse_resource(resource: &EventLogResource) -> Result<(), EventLogsError> {
    println!("{resource:?}");
    Ok(())
}

/// Parse PE files containing Event Log message resources. Returns tuple of EventMessageFile and ParameterMessageFile
fn gather_resource_paths() -> Result<(Vec<EventLogResource>, Vec<EventLogResource>), EventLogsError>
{
    let drive_result = get_systemdrive();
    let drive = match drive_result {
        Ok(result) => result,
        Err(_err) => {
            error!("[eventlog] Could not get systemdrive");
            return Err(EventLogsError::DefaultDrive);
        }
    };

    // Grab eventlog resource registry paths
    let reg_result = get_registry_keys(
        "ROOT",
        &create_regex(r".*\\controlset.*\\services\\eventlog\\.*").unwrap(),
        &format!("{drive}:\\Windows\\System32\\config\\SYSTEM"),
    );

    let reg_paths = match reg_result {
        Ok(result) => result,
        Err(err) => {
            error!("[eventlog] Could not parse registry for eventlog services: {err:?}");
            return Err(EventLogsError::EventLogServices);
        }
    };

    let mut event_paths = HashSet::new();
    let mut parameter_paths = HashSet::new();

    let envs = get_env();
    let mut update_env = HashMap::new();
    // ENV keys are insensitive so we lower case all env keys. Ex: %systemroot% == %SystemRoot%
    for (key, value) in envs {
        update_env.insert(key.to_lowercase(), value);
    }

    for path in reg_paths {
        for mut value in path.values {
            // We only want EventMessageFile and ParameterMessageFile values, which contain path to PE file
            if value.value != "EventMessageFile" && value.value != "ParameterMessageFile" {
                continue;
            }

            if value.data.starts_with("\\SystemRoot\\") {
                value.data = value.data.replace("\\SystemRoot\\", "%SystemRoot%");
            }

            if value.data.contains(';') {
                let multi_paths: Vec<&str> = value.data.split(';').collect();
                for entry in multi_paths {
                    if entry.contains('%') {
                        let env_values: Vec<&str> = entry.split('%').collect();
                        let mut real_path = String::new();
                        for env_value in env_values {
                            if env_value.is_empty() {
                                continue;
                            } else if env_value.contains('\\') {
                                if !env_value.starts_with('\\') {
                                    real_path += "\\";
                                }
                                real_path += env_value;
                            } else {
                                if let Some(path) = update_env.get(&env_value.to_lowercase()) {
                                    real_path += path;
                                }
                            }
                        }
                        if value.value == "EventMessageFile" {
                            event_paths.insert(real_path);
                        } else {
                            parameter_paths.insert(real_path);
                        }
                        continue;
                    }
                    if value.value == "EventMessageFile" {
                        event_paths.insert(entry.to_owned());
                    } else {
                        parameter_paths.insert(entry.to_owned());
                    }
                }
                continue;
            } else if value.data.contains('%') {
                let env_values: Vec<&str> = value.data.split('%').collect();
                let mut real_path = String::new();
                for env_value in env_values {
                    if env_value.is_empty() {
                        continue;
                    } else if env_value.contains('\\') {
                        if !env_value.starts_with('\\') {
                            real_path += "\\";
                        }
                        real_path += env_value;
                    } else {
                        if let Some(path) = update_env.get(&env_value.to_lowercase()) {
                            real_path += path;
                        }
                    }
                }
                if value.value == "EventMessageFile" {
                    event_paths.insert(real_path);
                } else {
                    parameter_paths.insert(real_path);
                }
                continue;
            }

            if value.value == "EventMessageFile" {
                event_paths.insert(value.data);
            } else {
                parameter_paths.insert(value.data);
            }
        }
    }

    let mut message_resources = Vec::new();
    for mut pe in event_paths {
        if !is_file(&pe) {
            // File does not exist. May be in a locale subdirectory
            let parent = get_parent_directory(&pe);
            pe = format!("{parent}\\en-US\\{}.mui", get_filename(&pe));

            if !is_file(&pe) {
                // no idea where it is
                continue;
            }
        }
        let resources_result = read_eventlog_resource(&pe);
        let resources = match resources_result {
            Ok(result) => result,
            Err(err) => {
                error!("[eventlog] Could not parse PE resource: {err:?}");
                continue;
            }
        };
        if resources.resource == ResourceType::Unknown {
            continue;
        }

        message_resources.push(resources)
    }

    let mut parameter_resources = Vec::new();
    for mut pe in parameter_paths {
        if !is_file(&pe) {
            // File does not exist. May be in a locale subdirectory
            let parent = get_parent_directory(&pe);
            pe = format!("{parent}\\en-US\\{}.mui", get_filename(&pe));

            if !is_file(&pe) {
                // no idea where it is
                continue;
            }
        }
        let resources_result = read_eventlog_resource(&pe);
        let resources = match resources_result {
            Ok(result) => result,
            Err(err) => {
                error!("[eventlog] Could not parse PE resource parameters: {err:?}");
                continue;
            }
        };
        if resources.resource == ResourceType::Unknown {
            continue;
        }

        parameter_resources.push(resources)
    }

    Ok((message_resources, parameter_resources))
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::{gather_resource_paths, get_resources};

    #[test]
    fn test_gather_resource_paths() {
        let (resources, para_resources) = gather_resource_paths().unwrap();
        assert!(!resources.is_empty());
        assert!(!para_resources.is_empty());
    }

    #[test]
    fn test_get_resources() {
        let result = get_resources().unwrap();
    }
}
