use super::{
    error::ServicesError,
    options::name::{
        error_control, failure_actions, service_state, service_type, start_mode, FailureActions,
        ServiceError, ServiceState, ServiceType, StartMode,
    },
    registry::get_services_data,
};
use crate::artifacts::os::windows::registry::parser::{KeyValue, RegistryEntry};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct ServicesData {
    state: ServiceState,
    name: String,
    display_name: String,
    description: String,
    start_mode: StartMode,
    path: String,
    service_type: Vec<ServiceType>,
    account: String,
    modified: i64,
    service_dll: String,
    failure_command: String,
    reset_period: u32,
    failure_actions: Vec<FailureActions>,
    required_privileges: Vec<String>,
    error_control: ServiceError,
    reg_path: String,
}

impl ServicesData {
    /// Parse Services data from provided Registry file
    pub(crate) fn parse_services(path: &str) -> Result<Vec<ServicesData>, ServicesError> {
        let entries = get_services_data(path)?;

        let mut services = Vec::new();

        let mut service_group = Vec::new();
        let mut current_service = String::new();
        for entry in entries {
            // Get the service name when starting loop
            if current_service.is_empty() {
                current_service = entry.name.clone();
            }

            // Services may have multiple subkeys. Group services together until we arrive at next service
            if entry.path.ends_with(&current_service)
                || entry.path.contains(&format!("\\{current_service}\\"))
            {
                service_group.push(entry);
                continue;
            }

            // Collected all keys associated with Service. Now parse what we have
            let service = ServicesData::collect_service(&service_group, &current_service);
            services.push(service);
            service_group.clear();

            // After done with parsing, start new Service collection
            current_service = entry.name.clone();
            service_group.push(entry);
        }

        // Get last service
        let last_service = ServicesData::collect_service(&service_group, &current_service);
        services.push(last_service);

        Ok(services)
    }

    /// Collect data associated with Service
    fn collect_service(service_data: &Vec<RegistryEntry>, service_name: &str) -> ServicesData {
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
            modified: 0,
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
                    service.modified = info.last_modified;
                    service.reg_path = info.path.clone();
                }
                continue;
            }

            for value in &info.values {
                if info.name == service_name {
                    service.modified = info.last_modified;
                    service.reg_path = info.path.clone();
                    // Get Service metadata associated with Service Name key
                    ServicesData::metadata(value, &mut service);
                }
                if info.name == "Parameters" && value.value.to_lowercase() == "servicedll" {
                    service.service_dll = value.data.clone();
                }
            }
        }

        service
    }

    /// Get metadata associated with Service
    fn metadata(value: &KeyValue, service: &mut ServicesData) {
        match value.value.as_str() {
            "Description" => service.description = value.data.clone(),
            "DisplayName" => service.display_name = value.data.clone(),
            "ErrorControl" => service.error_control = error_control(&value.data.clone()),
            "FailureActions" => {
                // Attempt to Service actions if Service fails
                (service.failure_actions, service.reset_period) =
                    failure_actions(&value.data).unwrap_or_default();
            }
            "ImagePath" => service.path = value.data.clone(),
            "ObjectName" => service.account = value.data.clone(),
            "ServiceSidType" => service.state = service_state(&value.data.clone()),
            "Start" => service.start_mode = start_mode(&value.data.clone()),
            "Type" => service.service_type = service_type(&value.data.clone()),
            "FailureCommand" => service.failure_command = value.data.clone(),
            "RequiredPrivileges" => {
                service.required_privileges = value.data.split('\n').map(str::to_string).collect();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::{
            registry::parser::KeyValue,
            services::{
                options::name::{ServiceError, ServiceState, StartMode},
                registry::get_services_data,
                service::ServicesData,
            },
        },
        utils::environment::get_systemdrive,
    };

    #[test]
    fn test_parse_services() {
        let drive = get_systemdrive().unwrap();
        let path = format!("{drive}:\\Windows\\System32\\config\\SYSTEM");
        let results = ServicesData::parse_services(&path).unwrap();

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

            let service = ServicesData::collect_service(&service_group, &current_service);
            services.push(service);
            service_group.clear();

            current_service = entry.name.clone();
            service_group.push(entry);
        }

        assert!(service_group.len() > 10);
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
            modified: 0,
            service_dll: String::new(),
            failure_command: String::new(),
            reset_period: 0,
            failure_actions: Vec::new(),
            required_privileges: Vec::new(),
            error_control: ServiceError::Unknown,
            reg_path: String::new(),
        };

        ServicesData::metadata(&test, &mut service);
        assert_eq!(
            service.path,
            "\\SystemRoot\\System32\\drivers\\1394ohci.sys"
        );
    }
}
