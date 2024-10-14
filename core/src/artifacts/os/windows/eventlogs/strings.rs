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
pub(crate) fn get_resources() -> Result<StringResource, EventLogsError> {
    let mut resources = gather_resource_paths()?;
    for resource in resources.templates.values_mut() {
        if resource.resource_data.mui_data.is_empty()
            && resource.resource_data.message_data.is_empty()
        {
            continue;
        }

        let _ = parse_resource(resource);

        // We are done parsing the resource data. Empty the bytes so we are not carrying them around
        resource.resource_data.message_data = Vec::new();
        resource.resource_data.mui_data = Vec::new();
        resource.resource_data.wevt_data = Vec::new();
    }

    Ok(resources)
}

/// Parse the PE resource data and get eventlog related strings
pub(crate) fn parse_resource(resource: &mut TemplateResource) -> Result<(), EventLogsError> {
    if resource.resource_data.message_data.is_empty() {
        let mui_result = parse_mui(
            &resource.resource_data.mui_data,
            &resource.resource_data.path,
        );
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
        let (_, message) = parse_table(&resource.resource_data.message_data).unwrap();
        resource.message_table = Some(message);
    }

    if !resource.resource_data.wevt_data.is_empty() {
        let (_, template) = parse_manifest(&resource.resource_data.wevt_data).unwrap();
        resource.wevt_template = Some(template);
    } else {
        let mui_result = parse_mui(
            &resource.resource_data.mui_data,
            &resource.resource_data.path,
        );
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

pub(crate) struct StringResource {
    /**Registry info about providers. Key is provider name */
    pub(crate) providers: HashMap<String, ProviderInfo>,
    /**Extracted DLL resources associated with providers. Key is file path */
    pub(crate) templates: HashMap<String, TemplateResource>,
}

pub(crate) struct ProviderInfo {
    pub(crate) registry_file_path: String,
    pub(crate) registry_path: String,
    pub(crate) name: String,
    pub(crate) message_file: Vec<String>,
    pub(crate) parameter_file: Vec<String>,
}

pub(crate) struct TemplateResource {
    pub(crate) path: String,
    pub(crate) resource_data: EventLogResource,
    pub(crate) message_table: Option<HashMap<u32, MessageTable>>,
    pub(crate) wevt_template: Option<HashMap<String, ManifestTemplate>>,
}

/// Parse PE files containing Event Log message resources
fn gather_resource_paths() -> Result<StringResource, EventLogsError> {
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

    let mut resources = StringResource {
        providers: HashMap::new(),
        templates: HashMap::new(),
    };

    for path in reg_paths {
        for mut value in path.values {
            // We only want EventMessageFile and ParameterMessageFile values, which contain path to PE file
            if value.value != "EventMessageFile" && value.value != "ParameterMessageFile" {
                continue;
            }

            if value.data.starts_with("\\SystemRoot\\") {
                value.data = value.data.replace("\\SystemRoot\\", "%SystemRoot%");
            }

            let mut provider = ProviderInfo {
                registry_file_path: path.registry_path.clone(),
                registry_path: path.path.clone(),
                name: path.name.clone(),
                message_file: Vec::new(),
                parameter_file: Vec::new(),
            };

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
                            provider.message_file.push(real_path);
                        } else {
                            provider.parameter_file.push(real_path);
                        }
                        continue;
                    }

                    if value.value == "EventMessageFile" {
                        provider.message_file.push(entry.to_string());
                    } else {
                        provider.parameter_file.push(entry.to_string());
                    }
                }
                resources.providers.insert(provider.name.clone(), provider);
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
                    provider.message_file.push(real_path);
                } else {
                    provider.parameter_file.push(real_path);
                }

                resources.providers.insert(provider.name.clone(), provider);
                continue;
            }

            if value.value == "EventMessageFile" {
                provider.message_file.push(value.data);
            } else {
                provider.parameter_file.push(value.data);
            }

            resources.providers.insert(provider.name.clone(), provider);
        }
    }

    // Now go through and parse all PE files associated with the EventLog providers
    for provider in resources.providers.values() {
        for file in &provider.message_file {
            update_resource(&mut resources.templates, file);
        }

        for file in &provider.parameter_file {
            update_resource(&mut resources.templates, file);
        }
    }

    Ok(resources)
}

/// Determine if we are already tracking the path to the DLL file
fn update_resource(templates: &mut HashMap<String, TemplateResource>, file: &str) {
    let mut real_path = file.to_string();
    if !is_file(&real_path) {
        // File does not exist. May be in a locale subdirectory
        let parent = get_parent_directory(&real_path);
        let pe = format!("{parent}\\en-US\\{}.mui", get_filename(&real_path));

        if !is_file(&pe) {
            // no idea where it is
            return;
        }
        real_path = pe;
    }

    // Check if we already parsed this PE file
    if templates.get(&real_path).is_some() {
        return;
    }

    let resources_result = read_eventlog_resource(&real_path);
    let resource_data = match resources_result {
        Ok(result) => result,
        Err(err) => {
            error!("[eventlog] Could not parse PE resource: {err:?}");
            return;
        }
    };
    if resource_data.message_data.is_empty()
        && resource_data.mui_data.is_empty()
        && resource_data.wevt_data.is_empty()
    {
        return;
    }

    let temp_info = TemplateResource {
        path: real_path.clone(),
        resource_data,
        message_table: None,
        wevt_template: None,
    };

    templates.insert(real_path, temp_info);
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::{gather_resource_paths, get_resources};

    #[test]
    fn test_gather_resource_paths() {
        let resources = gather_resource_paths().unwrap();
        assert!(!resources.providers.is_empty());
        assert!(!resources.templates.is_empty());
    }

    #[test]
    fn test_get_resources() {
        let result = get_resources().unwrap();
    }
}
