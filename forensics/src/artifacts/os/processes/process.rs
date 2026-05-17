use super::executable::elf_metadata;
use super::pe::pe_metadata;
/**
 * Get a process listing using `sysinfo` crate
 * Depending on `ProcessOptions` will also parse and get basic executable metadata
 */
use super::{error::ProcessError, macho::macho_metadata};
use crate::artifacts::os::systeminfo::info::get_platform_enum;
use crate::{
    artifacts::{os::systeminfo::info::PlatformType, output::output_artifact},
    filesystem::{directory::get_parent_directory, files::hash_file},
    structs::toml::Output,
    utils::time::{time_now, unixepoch_to_iso},
};
use common::files::Hashes;
use common::system::Processes;
use log::{error, info, warn};
use serde_json::Value;
use std::ffi::OsStr;
use sysinfo::{Process, ProcessRefreshKind, ProcessesToUpdate, System};

/// Get process listing.
pub(crate) fn proc_list(
    hashes: &Hashes,
    binary_data: bool,
    filter: bool,
    output: &mut Output,
) -> Result<(), ProcessError> {
    let mut proc = System::new();
    let mut processes_list: Vec<Processes> = Vec::new();
    let start_time = time_now();

    proc.refresh_processes_specifics(
        ProcessesToUpdate::All,
        false,
        ProcessRefreshKind::everything(),
    );
    if proc.processes().is_empty() {
        return Err(ProcessError::Empty);
    }

    // We may encounter really large binaries. So to keep memory usage low
    // Every 5 processes we parse, we output the results
    // If we do not parse binary info. We gather all processes at once
    let binary_proc_limit = 5;
    let plat = get_platform_enum();

    for process in proc.processes().values() {
        let system_proc = proc_info(process, hashes, binary_data, &plat);
        processes_list.push(system_proc);
        if binary_data && processes_list.len() == binary_proc_limit {
            let _ = output_process(&processes_list, output, filter, start_time);
            processes_list = Vec::new();
        }
    }

    if !processes_list.is_empty() {
        let _ = output_process(&processes_list, output, filter, start_time);
    }
    Ok(())
}

/// Pull a process listing and return the results. If we parse binary data for all processes. Expect a lot of data
pub(crate) fn proc_list_entries(
    hashes: &Hashes,
    binary_data: bool,
) -> Result<Vec<Processes>, ProcessError> {
    let mut proc = System::new();
    let mut processes_list: Vec<Processes> = Vec::new();
    proc.refresh_processes_specifics(
        ProcessesToUpdate::All,
        false,
        ProcessRefreshKind::everything(),
    );
    if proc.processes().is_empty() {
        return Err(ProcessError::Empty);
    }
    let plat = get_platform_enum();
    for process in proc.processes().values() {
        let system_proc = proc_info(process, hashes, binary_data, &plat);
        processes_list.push(system_proc);
    }

    Ok(processes_list)
}

// Get the process info data
fn proc_info(
    process: &Process,
    hashes: &Hashes,
    binary_data: bool,
    plat: &PlatformType,
) -> Processes {
    let uid_result = process.user_id();
    let uid = match uid_result {
        Some(result) => result.to_string(),
        _ => String::new(),
    };

    let gid_result = process.group_id();
    let gid = match gid_result {
        Some(result) => result.to_string(),
        _ => String::new(),
    };

    let path_result = process.exe();
    let path = match path_result {
        Some(result) => result.display().to_string(),
        None => String::new(),
    };
    let mut system_proc = Processes {
        path: get_parent_directory(&path),
        full_path: path,
        name: process.name().to_str().unwrap_or_default().to_string(),
        pid: process.pid().as_u32(),
        ppid: 0,
        environment: process
            .environ()
            .join(OsStr::new(" "))
            .to_str()
            .unwrap_or_default()
            .to_string(),
        status: process.status().to_string(),
        arguments: process
            .cmd()
            .join(OsStr::new(" "))
            .to_str()
            .unwrap_or_default()
            .to_string(),
        memory_usage: process.memory(),
        virtual_memory_usage: process.virtual_memory(),
        start_time: unixepoch_to_iso(process.start_time() as i64),
        uid,
        gid,
        md5: String::new(),
        sha1: String::new(),
        sha256: String::new(),
        binary_info: Value::Null,
    };

    if binary_data && !system_proc.full_path.is_empty() {
        let binary_results = executable_metadata(&system_proc.full_path, plat);
        match binary_results {
            Ok(results) => {
                system_proc.binary_info = results;
            }
            Err(err) => info!("[processes] Failed to get executable data: {err:?}"),
        }
    }

    // Check if arguments contain process full path. If it does remove it and get all other arguments
    if system_proc
        .arguments
        .to_lowercase()
        .starts_with(&system_proc.full_path.to_lowercase())
    {
        system_proc.arguments = system_proc.arguments[system_proc.full_path.len()..].to_string();
    }

    let mut first_proc = 1;
    if plat == &PlatformType::Windows {
        first_proc = 0;
    }

    if process.pid().as_u32() != first_proc {
        let parent_pid = process.parent();
        if let Some(ppid) = parent_pid {
            system_proc.ppid = ppid.as_u32();
        } else {
            warn!("[processes] No Parent PID for: {}", process.pid());
        }
    }

    if hashes.md5 || hashes.sha1 || hashes.sha256 {
        (system_proc.md5, system_proc.sha1, system_proc.sha256) =
            hash_file(hashes, &system_proc.full_path);
    }

    system_proc
}

/// Get executable metadata
fn executable_metadata(path: &str, plat: &PlatformType) -> Result<Value, ProcessError> {
    let binary_info = match plat {
        PlatformType::Linux => {
            let result = elf_metadata(path)?;
            serde_json::to_value(&result).unwrap_or_default()
        }
        PlatformType::Macos => {
            let result = macho_metadata(path)?;
            serde_json::to_value(&result).unwrap_or_default()
        }
        PlatformType::Windows => {
            let result = pe_metadata(path)?;
            serde_json::to_value(&result).unwrap_or_default()
        }
        PlatformType::Unknown => Value::Null,
    };

    Ok(binary_info)
}

/// Output processes results
fn output_process(
    entries: &[Processes],
    output: &mut Output,
    filter: bool,
    start_time: u64,
) -> Result<(), ProcessError> {
    if entries.is_empty() {
        return Ok(());
    }

    let serde_data_result = serde_json::to_value(entries);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[processes] Failed to serialize process entries: {err:?}");
            return Err(ProcessError::Serialize);
        }
    };
    let result = output_artifact(&mut serde_data, "processes", output, start_time, filter);
    match result {
        Ok(_result) => {}
        Err(err) => {
            error!("[processes] Could not output process data: {err:?}");
            return Err(ProcessError::OutputData);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::processes::process::executable_metadata;
    use crate::artifacts::os::processes::process::{proc_info, proc_list};
    use crate::artifacts::os::systeminfo::info::PlatformType;
    use crate::artifacts::os::systeminfo::info::get_platform_enum;
    use crate::structs::toml::Output;
    use common::files::Hashes;
    use common::system::Processes;
    use sysinfo::{ProcessesToUpdate, System};

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
    fn test_proc_list() {
        let hashes = Hashes {
            md5: false,
            sha1: false,
            sha256: false,
        };
        let mut output = output_options("proc_test", "local", "./tmp", false);

        proc_list(&hashes, false, false, &mut output).unwrap();
    }

    #[test]
    fn test_proc_info() {
        let mut proc = System::new();
        let mut processes_list: Vec<Processes> = Vec::new();

        proc.refresh_processes(ProcessesToUpdate::All, false);

        let hashes = Hashes {
            md5: true,
            sha1: true,
            sha256: true,
        };

        let plat = get_platform_enum();
        for process in proc.processes().values() {
            let system_proc = proc_info(process, &hashes, false, &plat);
            processes_list.push(system_proc);
        }
        assert!(processes_list.len() > 10);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_executable_metadata() {
        let test_path = "/bin/ls";
        let results = executable_metadata(test_path, &PlatformType::Macos).unwrap();

        assert_eq!(results.as_array().unwrap().len(), 2);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_executable_metadata() {
        let test_path = "/bin/ls";
        let results = executable_metadata(test_path, &PlatformType::Linux).unwrap();

        assert!(!results.is_null());
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_executable_metadata() {
        let test_path = "C:\\Windows\\explorer.exe";
        let results = executable_metadata(test_path, &PlatformType::Windows).unwrap();

        assert!(!results.is_null());
    }
}
