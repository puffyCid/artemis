use common::windows::{ServiceState, ServicesData};
use log::warn;
use std::{
    ffi::{OsString, c_void},
    iter::once,
    os::windows::ffi::OsStrExt,
    ptr::null_mut,
};
use windows_sys::Win32::System::Services::{
    CloseServiceHandle, OpenSCManagerW, OpenServiceW, QueryServiceStatusEx,
    SC_MANAGER_ENUMERATE_SERVICE, SC_STATUS_PROCESS_INFO, SERVICE_QUERY_STATUS,
    SERVICE_STATUS_PROCESS,
};

pub(crate) fn service_state(services: &mut [ServicesData]) {
    #[allow(unsafe_code)]
    let service_manager =
        unsafe { OpenSCManagerW(null_mut(), null_mut(), SC_MANAGER_ENUMERATE_SERVICE) };
    if service_manager.is_null() {
        return;
    }
    for entry in services {
        let wide_string = OsString::from(&entry.name);
        let ut16_bytes: Vec<u16> = wide_string.encode_wide().chain(once(0)).collect();
        #[allow(unsafe_code)]
        let service =
            unsafe { OpenServiceW(service_manager, ut16_bytes.as_ptr(), SERVICE_QUERY_STATUS) };
        if service.is_null() {
            warn!("[services] Cannot get service state for: {}", entry.name);
            continue;
        }

        let mut state = SERVICE_STATUS_PROCESS {
            dwServiceType: 0,
            dwCurrentState: 0,
            dwControlsAccepted: 0,
            dwWin32ExitCode: 0,
            dwServiceSpecificExitCode: 0,
            dwCheckPoint: 0,
            dwWaitHint: 0,
            dwProcessId: 0,
            dwServiceFlags: 0,
        };

        let mut result_len = 0;
        let size = size_of::<SERVICE_STATUS_PROCESS>() as u32;
        #[allow(unsafe_code)]
        let status = unsafe {
            QueryServiceStatusEx(
                service,
                SC_STATUS_PROCESS_INFO,
                (&mut state as *mut SERVICE_STATUS_PROCESS).cast::<u8>(),
                size,
                &mut result_len,
            )
        };

        close_handle(service);
        if status == 0 {
            continue;
        }
        entry.state = state_value(state.dwCurrentState);
    }

    close_handle(service_manager);
}

fn close_handle(handle: *mut c_void) -> i32 {
    #[allow(unsafe_code)]
    unsafe {
        CloseServiceHandle(handle)
    }
}

/// Get Service State type for Service
fn state_value(value: u32) -> ServiceState {
    match value {
        1 => ServiceState::Stopped,
        2 => ServiceState::StartPending,
        3 => ServiceState::StopPending,
        4 => ServiceState::Running,
        5 => ServiceState::ContinuePending,
        6 => ServiceState::PausePending,
        7 => ServiceState::Paused,
        _ => ServiceState::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::services::state::{
        close_handle, service_state, state_value,
    };
    use common::windows::{ServiceState, ServicesData};
    use std::ptr::null_mut;
    use windows_sys::Win32::System::Services::{OpenSCManagerW, SC_MANAGER_ENUMERATE_SERVICE};

    #[test]
    fn test_service_state() {
        let mut test = vec![
            ServicesData {
                name: String::from("ACPI"),
                ..Default::default()
            },
            ServicesData {
                name: String::from("AppID"),
                ..Default::default()
            },
        ];
        service_state(&mut test);
        for entry in test {
            assert_ne!(entry.state, ServiceState::Unknown);
        }
    }

    #[test]
    fn test_state_value() {
        let test = [1, 2, 3, 4, 5, 6, 7];
        for entry in test {
            assert_ne!(state_value(entry), ServiceState::Unknown);
        }
    }

    #[test]
    fn test_close_handle() {
        #[allow(unsafe_code)]
        let test = unsafe { OpenSCManagerW(null_mut(), null_mut(), SC_MANAGER_ENUMERATE_SERVICE) };
        close_handle(test);
    }
}
