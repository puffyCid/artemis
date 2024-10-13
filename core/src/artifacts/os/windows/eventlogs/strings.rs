use super::{
    error::EventLogsError,
    resources::{
        manifest::wevt::{parse_manifest, ManifestTemplate},
        message::{parse_table, MessageTable},
        mui::parse_mui,
    },
};
use crate::{
    artifacts::os::windows::{
        pe::resources::{read_eventlog_resource, EventLogResource},
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
use std::collections::HashMap;

/// Parse and extract eventlog string resources
pub(crate) fn get_resources() -> Result<HashMap<String, StringResource>, EventLogsError> {
    let mut resources = gather_resource_paths()?;
    for resource in resources.values_mut() {
        if resource.data.mui_data.is_empty() && resource.data.message_data.is_empty() {
            continue;
        }

        let _ = parse_resource(resource);

        // We are done parsing the resource data. Empty the bytes so we are not carrying them around
        resource.data.message_data = Vec::new();
        resource.data.mui_data = Vec::new();
        resource.data.wevt_data = Vec::new();
    }

    Ok(resources)
}

/// Parse the PE resource data and get eventlog related strings
pub(crate) fn parse_resource(resource: &mut StringResource) -> Result<(), EventLogsError> {
    if resource.data.message_data.is_empty() {
        let mui_result = parse_mui(&resource.data.mui_data, &resource.data.path);
        let mui_resource = match mui_result {
            Ok((_, result)) => result,
            Err(_err) => return Err(EventLogsError::NoMessageTable),
        };
        if mui_resource.message_data.is_empty() {
            return Err(EventLogsError::NoMessageTable);
        }
        let (_, message) = parse_table(&mui_resource.message_data).unwrap();
        resource.message_table = Some(message);
    } else {
        let (_, message) = parse_table(&resource.data.message_data).unwrap();
        resource.message_table = Some(message);
    }

    if !resource.data.wevt_data.is_empty() {
        let (_, template) = parse_manifest(&resource.data.wevt_data).unwrap();
        resource.wevt_template = Some(template);
    } else {
        let mui_result = parse_mui(&resource.data.mui_data, &resource.data.path);
        let mui_resource = match mui_result {
            Ok((_, result)) => result,
            Err(_err) => return Err(EventLogsError::NoMessageTable),
        };
        if mui_resource.wevt_data.is_empty() {
            return Err(EventLogsError::NoWevtTemplate);
        }
        let (_, template) = parse_manifest(&mui_resource.wevt_data).unwrap();
        resource.wevt_template = Some(template);
    }

    Ok(())
}

#[derive(Debug)]
pub(crate) struct StringResource {
    pub(crate) path: String,
    pub(crate) registry_file: String,
    pub(crate) registry_info: Vec<StringResourceRegistry>,
    pub(crate) data: EventLogResource,
    pub(crate) message_table: Option<HashMap<u32, MessageTable>>,
    pub(crate) wevt_template: Option<HashMap<String, ManifestTemplate>>,
}

#[derive(Debug)]
pub(crate) struct StringResourceRegistry {
    pub(crate) registry_key: String,
    pub(crate) resource_type: ResourceType,
}

#[derive(PartialEq, Debug)]
enum ResourceType {
    EventMessageFile,
    ParameterMessageFile,
    Unknown,
}

/// Parse PE files containing Event Log message resources
fn gather_resource_paths() -> Result<HashMap<String, StringResource>, EventLogsError> {
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

    let envs = get_env();
    let mut update_env = HashMap::new();
    // ENV keys are insensitive so we lower case all env keys. Ex: %systemroot% == %SystemRoot%
    for (key, value) in envs {
        update_env.insert(key.to_lowercase(), value);
    }

    let mut resources = HashMap::new();

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
                    let mut resource = StringResource {
                        path: String::new(),
                        registry_file: path.registry_path.clone(),
                        data: EventLogResource {
                            mui_data: Vec::new(),
                            wevt_data: Vec::new(),
                            message_data: Vec::new(),
                            path: String::new(),
                        },
                        registry_info: Vec::new(),
                        message_table: None,
                        wevt_template: None,
                    };

                    let mut registry_resource = StringResourceRegistry {
                        registry_key: path.path.clone(),
                        resource_type: ResourceType::Unknown,
                    };

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
                            resource.path = real_path;
                            registry_resource.resource_type = ResourceType::EventMessageFile;
                            resource.registry_info.push(registry_resource);

                            update_resource(&mut resources, resource);
                        } else {
                            resource.path = real_path;
                            registry_resource.resource_type = ResourceType::ParameterMessageFile;
                            resource.registry_info.push(registry_resource);

                            update_resource(&mut resources, resource);
                        }
                        continue;
                    }
                    if value.value == "EventMessageFile" {
                        resource.path = entry.to_string();
                        registry_resource.resource_type = ResourceType::EventMessageFile;
                        resource.registry_info.push(registry_resource);

                        update_resource(&mut resources, resource);
                    } else {
                        resource.path = entry.to_string();
                        registry_resource.resource_type = ResourceType::ParameterMessageFile;
                        resource.registry_info.push(registry_resource);

                        update_resource(&mut resources, resource);
                    }
                }
                continue;
            } else if value.data.contains('%') {
                let env_values: Vec<&str> = value.data.split('%').collect();
                let mut real_path = String::new();

                let mut resource = StringResource {
                    path: String::new(),
                    registry_file: path.registry_path.clone(),
                    data: EventLogResource {
                        mui_data: Vec::new(),
                        wevt_data: Vec::new(),
                        message_data: Vec::new(),
                        path: String::new(),
                    },
                    registry_info: Vec::new(),
                    message_table: None,
                    wevt_template: None,
                };

                let mut registry_resource = StringResourceRegistry {
                    registry_key: path.path.clone(),
                    resource_type: ResourceType::Unknown,
                };

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
                    resource.path = real_path;
                    registry_resource.resource_type = ResourceType::EventMessageFile;
                    resource.registry_info.push(registry_resource);

                    update_resource(&mut resources, resource);
                } else {
                    resource.path = real_path;
                    registry_resource.resource_type = ResourceType::ParameterMessageFile;
                    resource.registry_info.push(registry_resource);

                    update_resource(&mut resources, resource);
                }
                continue;
            }
            let mut resource = StringResource {
                path: String::new(),
                registry_file: path.registry_path.clone(),
                data: EventLogResource {
                    mui_data: Vec::new(),
                    wevt_data: Vec::new(),
                    message_data: Vec::new(),
                    path: String::new(),
                },
                registry_info: Vec::new(),
                message_table: None,
                wevt_template: None,
            };

            let mut registry_resource = StringResourceRegistry {
                registry_key: path.path.clone(),
                resource_type: ResourceType::Unknown,
            };

            if value.value == "EventMessageFile" {
                resource.path = value.data;
                registry_resource.resource_type = ResourceType::EventMessageFile;
                resource.registry_info.push(registry_resource);

                update_resource(&mut resources, resource);
            } else {
                resource.path = value.data;
                registry_resource.resource_type = ResourceType::ParameterMessageFile;
                resource.registry_info.push(registry_resource);

                update_resource(&mut resources, resource);
            }
        }
    }

    for resource in resources.values_mut() {
        if !is_file(&resource.path) {
            // File does not exist. May be in a locale subdirectory
            let parent = get_parent_directory(&resource.path);
            let pe = format!("{parent}\\en-US\\{}.mui", get_filename(&resource.path));

            if !is_file(&pe) {
                // no idea where it is
                continue;
            }
            resource.path = pe;
        }
        let resources_result = read_eventlog_resource(&resource.path);
        let pe_resources = match resources_result {
            Ok(result) => result,
            Err(err) => {
                error!("[eventlog] Could not parse PE resource: {err:?}");
                continue;
            }
        };
        if pe_resources.message_data.is_empty()
            && pe_resources.mui_data.is_empty()
            && pe_resources.wevt_data.is_empty()
        {
            continue;
        }

        resource.data = pe_resources;
    }

    Ok(resources)
}

/// Determine if we are already tracking the path to the DLL file
fn update_resource(resources: &mut HashMap<String, StringResource>, mut resource: StringResource) {
    if let Some(update_resource) = resources.get_mut(&resource.path) {
        update_resource
            .registry_info
            .append(&mut resource.registry_info);
        return;
    }

    resources.insert(resource.path.clone(), resource);
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::{gather_resource_paths, get_resources};

    #[test]
    fn test_gather_resource_paths() {
        let resources = gather_resource_paths().unwrap();
        assert!(!resources.is_empty());
    }

    #[test]
    fn test_get_resources() {
        let result = get_resources().unwrap();
    }
}
