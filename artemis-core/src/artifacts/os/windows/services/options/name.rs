use crate::{
    artifacts::os::windows::services::error::ServicesError,
    utils::{
        encoding::base64_decode_standard,
        nom_helper::{nom_unsigned_four_bytes, Endian},
    },
};
use log::error;
use serde::Serialize;

#[derive(Debug, PartialEq, Serialize)]
pub(crate) enum StartMode {
    Automatic,
    Boot,
    Disabled,
    Manual,
    System,
    Unknown,
}

#[derive(Debug, PartialEq, Serialize)]
pub(crate) enum ServiceState {
    Stopped,
    StartPending,
    StopPending,
    Running,
    ContinuePending,
    PausePending,
    Paused,
    Unknown,
}

#[derive(Debug, PartialEq, Serialize)]
pub(crate) enum ServiceError {
    Ignore,
    Normal,
    Severe,
    Critical,
    Unknown,
}

#[derive(Debug, PartialEq, Serialize)]
pub(crate) enum ServiceType {
    Adapter,
    FileSystemDriver,
    InteractiveProcess,
    KernelDriver,
    RecognizeDriver,
    Win32OwnProcess,
    Win32SharedProcess,
}

#[derive(Debug, PartialEq, Serialize)]
pub(crate) struct FailureActions {
    action: Action,
    delay: u32,
}

#[derive(Debug, PartialEq, Serialize)]
pub(crate) enum Action {
    None,
    Reboot,
    Restart,
    RunCommand,
    Unknown,
}

/// Get Error Control type for Service
pub(crate) fn error_control(value: &str) -> ServiceError {
    match value {
        "0" => ServiceError::Ignore,
        "1" => ServiceError::Normal,
        "2" => ServiceError::Severe,
        "3" => ServiceError::Critical,
        _ => ServiceError::Unknown,
    }
}

/// Get Service State type for Service
pub(crate) fn service_state(value: &str) -> ServiceState {
    match value {
        "1" => ServiceState::Stopped,
        "2" => ServiceState::StartPending,
        "3" => ServiceState::StopPending,
        "4" => ServiceState::Running,
        "5" => ServiceState::ContinuePending,
        "6" => ServiceState::PausePending,
        "7" => ServiceState::Paused,
        _ => ServiceState::Unknown,
    }
}

/// Get Start Mode type for Service
pub(crate) fn start_mode(value: &str) -> StartMode {
    match value {
        "0" => StartMode::Boot,
        "1" => StartMode::System,
        "2" => StartMode::Automatic,
        "3" => StartMode::Manual,
        "4" => StartMode::Disabled,
        _ => StartMode::Unknown,
    }
}

/// Get Error Control type for Service
pub(crate) fn service_type(value: &str) -> Vec<ServiceType> {
    let adapt = 4;
    let file = 2;
    let kernel = 1;
    let driver = 8;
    let own = 16;
    let share = 32;
    let interactive = 256;

    let serv_type: u16 = str::parse(value).unwrap_or_default();

    let mut types = Vec::new();
    if (serv_type & adapt) == adapt {
        types.push(ServiceType::Adapter);
    }
    if (serv_type & file) == file {
        types.push(ServiceType::FileSystemDriver);
    }
    if (serv_type & interactive) == interactive {
        types.push(ServiceType::InteractiveProcess);
    }
    if (serv_type & kernel) == kernel {
        types.push(ServiceType::KernelDriver);
    }
    if (serv_type & driver) == driver {
        types.push(ServiceType::RecognizeDriver);
    }
    if (serv_type & own) == own {
        types.push(ServiceType::Win32OwnProcess);
    }
    if (serv_type & share) == share {
        types.push(ServiceType::Win32SharedProcess);
    }

    types
}

/// Get Failure Actions for Service
pub(crate) fn failure_actions(value: &str) -> Result<(Vec<FailureActions>, u32), ServicesError> {
    let data_result = base64_decode_standard(value);
    let data = match data_result {
        Ok(result) => result,
        Err(err) => {
            error!("[services] Failed to base64 decode failure actions data: {err:?}");
            return Err(ServicesError::Base64Decode);
        }
    };
    let failures_result = parse_failure_actions(&data);
    let (failures, reset) = match failures_result {
        Ok((_, result)) => result,
        Err(_err) => {
            error!("[services] Could not parse Failure Actions");
            return Err(ServicesError::ServicesData);
        }
    };

    Ok((failures, reset))
}

/// Parse Failure Actions
fn parse_failure_actions(data: &[u8]) -> nom::IResult<&[u8], (Vec<FailureActions>, u32)> {
    let (input, reset_period) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (input, _reboot_msg) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, _command) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let (input, actions_count) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (mut action_start, _actions_pointer) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let mut count = 0;
    let mut failures = Vec::new();
    while count < actions_count {
        let (input, action_type) = nom_unsigned_four_bytes(action_start, Endian::Le)?;
        let (input, delay_microseconds) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let action = match action_type {
            0 => Action::None,
            2 => Action::Reboot,
            1 => Action::Restart,
            3 => Action::RunCommand,
            _ => Action::Unknown,
        };

        let adjust_seconds = 1000;
        let failure = FailureActions {
            action,
            delay: delay_microseconds / adjust_seconds,
        };
        failures.push(failure);

        count += 1;
        action_start = input;
    }

    Ok((action_start, (failures, reset_period)))
}

#[cfg(test)]
mod tests {
    use super::{error_control, failure_actions};
    use crate::artifacts::os::windows::services::options::name::{
        parse_failure_actions, service_state, service_type, start_mode, ServiceError, ServiceState,
        ServiceType, StartMode,
    };

    #[test]
    fn test_error_control() {
        let test = "1";
        let result = error_control(test);
        assert_eq!(result, ServiceError::Normal);
    }

    #[test]
    fn test_failure_actions() {
        let test = "gFEBAAAAAAAAAAAABAAAABQAAAABAAAAECcAAAEAAAAQJwAAAQAAABAnAAAAAAAAAAAAAA==";
        let (failures, reset) = failure_actions(test).unwrap();

        assert_eq!(failures.len(), 4);
        assert_eq!(reset, 86400);
    }

    #[test]
    fn test_parse_failure_actions() {
        let test = [
            128, 81, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 20, 0, 0, 0, 1, 0, 0, 0, 16, 39, 0,
            0, 1, 0, 0, 0, 16, 39, 0, 0, 1, 0, 0, 0, 16, 39, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let (_, (failures, reset)) = parse_failure_actions(&test).unwrap();

        assert_eq!(failures.len(), 4);
        assert_eq!(reset, 86400);
    }

    #[test]
    fn test_service_type() {
        let test = "1";
        let result = service_type(test);
        assert_eq!(result, vec![ServiceType::KernelDriver]);
    }

    #[test]
    fn test_start_mode() {
        let test = "1";
        let result = start_mode(test);
        assert_eq!(result, StartMode::System);
    }

    #[test]
    fn test_service_state() {
        let test = "1";
        let result = service_state(test);
        assert_eq!(result, ServiceState::Stopped);
    }
}
