use super::{
    error::ServicesError,
    options::name::{error_control, failure_actions, service_state, service_type, start_mode},
    registry::get_services_data,
};
use common::windows::{
    KeyValue, RegistryData, ServiceError, ServiceState, ServicesData, StartMode,
};

/// Parse Services data from provided Registry file
pub(crate) fn parse_services(path: &str) -> Result<Vec<ServicesData>, ServicesError> {
    let entries = get_services_data(path)?;

    let mut services = Vec::new();

    let mut service_group = Vec::new();
    let mut current_service = String::new();
    for entry in entries {
        // Get the service name when starting loop
        if current_service.is_empty() {
            current_service.clone_from(&entry.name);
        }

        // Services may have multiple subkeys. Group services together until we arrive at next service
        if entry.path.ends_with(&current_service)
            || entry.path.contains(&format!("\\{current_service}\\"))
        {
            service_group.push(entry);
            continue;
        }

        // Collected all keys associated with Service. Now parse what we have
        let service = collect_service(&service_group, &current_service);
        services.push(service);
        service_group.clear();

        // After done with parsing, start new Service collection
        current_service.clone_from(&entry.name);
        service_group.push(entry);
    }

    // Get last service
    let last_service = collect_service(&service_group, &current_service);
    services.push(last_service);

    Ok(services)
}

/// Collect data associated with Service
fn collect_service(service_data: &Vec<RegistryData>, service_name: &str) -> ServicesData {
    let name = service_name.to_string();

    let mut service = ServicesData {
        state: ServiceState::Unknown,
        name,
        display_name: String::new(),
        description: String::new(),
        start_mode: StartMode::Unknown,
        path: String::new(),
        service_type: Vec::new(),
        account: String::new(),
        modified: String::new(),
        service_dll: String::new(),
        failure_command: String::new(),
        reset_period: 0,
        failure_actions: Vec::new(),
        required_privileges: Vec::new(),
        error_control: ServiceError::Unknown,
        reg_path: String::new(),
    };

    for info in service_data {
        if info.values.is_empty() {
            if info.name == service_name {
                service.modified.clone_from(&info.last_modified);
                service.reg_path.clone_from(&info.path);
            }
            continue;
        }

        for value in &info.values {
            if info.name == service_name {
                service.modified.clone_from(&info.last_modified);
                service.reg_path.clone_from(&info.path);
                // Get Service metadata associated with Service Name key
                metadata(value, &mut service);
            }
            if info.name == "Parameters" && value.value.to_lowercase() == "servicedll" {
                service.service_dll.clone_from(&value.data);
            }
        }
    }

    service
}

/// Get metadata associated with Service
fn metadata(value: &KeyValue, service: &mut ServicesData) {
    match value.value.as_str() {
        "Description" => service.description.clone_from(&value.data),
        "DisplayName" => service.display_name.clone_from(&value.data),
        "ErrorControl" => service.error_control = error_control(&value.data),
        "FailureActions" => {
            // Attempt to Service actions if Service fails
            (service.failure_actions, service.reset_period) =
                failure_actions(&value.data).unwrap_or_default();
        }
        "ImagePath" => service.path.clone_from(&value.data),
        "ObjectName" => service.account.clone_from(&value.data),
        "ServiceSidType" => service.state = service_state(&value.data),
        "Start" => service.start_mode = start_mode(&value.data),
        "Type" => service.service_type = service_type(&value.data),
        "FailureCommand" => service.failure_command.clone_from(&value.data),
        "RequiredPrivileges" => {
            service.required_privileges = value.data.split('\n').map(str::to_string).collect();
        }
        _ => {}
    }
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use crate::{
        artifacts::os::windows::services::{
            registry::get_services_data,
            service::{collect_service, metadata, parse_services, ServicesData},
        },
        utils::environment::get_systemdrive,
    };
    use common::windows::{KeyValue, ServiceError, ServiceState, StartMode};

    #[test]
    fn test_parse_services() {
        let drive = get_systemdrive().unwrap();
        let path = format!("{drive}:\\Windows\\System32\\config\\SYSTEM");
        let results = parse_services(&path).unwrap();

        assert!(results.len() > 10);
    }

    #[test]
    fn test_collect_service() {
        let drive = get_systemdrive().unwrap();
        let path = format!("{drive}:\\Windows\\System32\\config\\SYSTEM");
        let entries = get_services_data(&path).unwrap();

        let mut services = Vec::new();

        let mut service_group = Vec::new();
        let mut current_service = String::new();
        for entry in entries {
            if current_service.is_empty() {
                current_service = entry.name.clone();
            }

            if entry.path.ends_with(&current_service)
                || entry.path.contains(&format!("\\{current_service}\\"))
            {
                service_group.push(entry);
                continue;
            }

            let service = collect_service(&service_group, &current_service);
            services.push(service);
            service_group.clear();

            current_service = entry.name.clone();
            service_group.push(entry);
        }

        assert!(services.len() > 10);
    }

    #[test]
    fn test_metadata() {
        let test = KeyValue {
            value: "ImagePath".to_owned(),
            data: "\\SystemRoot\\System32\\drivers\\1394ohci.sys".to_owned(),
            data_type: "REG_EXPAND_SZ".to_owned(),
        };

        let mut service = ServicesData {
            state: ServiceState::Unknown,
            name: String::new(),
            display_name: String::new(),
            description: String::new(),
            start_mode: StartMode::Unknown,
            path: String::new(),
            service_type: Vec::new(),
            account: String::new(),
            modified: String::new(),
            service_dll: String::new(),
            failure_command: String::new(),
            reset_period: 0,
            failure_actions: Vec::new(),
            required_privileges: Vec::new(),
            error_control: ServiceError::Unknown,
            reg_path: String::new(),
        };

        metadata(&test, &mut service);
        assert_eq!(
            service.path,
            "\\SystemRoot\\System32\\drivers\\1394ohci.sys"
        );
    }
}
