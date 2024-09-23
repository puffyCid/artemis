use std::ffi::OsStr;

/**
 * Get a process listing using `sysinfo` crate
 * Depending on `ProcessOptions` will also parse and get basic executable metadata
 */
use super::error::ProcessError;
use crate::{
    filesystem::{directory::get_parent_directory, files::hash_file},
    utils::time::unixepoch_to_iso,
};
use common::files::Hashes;
use common::system::Processes;
use log::{info, warn};
use sysinfo::{Process, ProcessRefreshKind, ProcessesToUpdate, System};

#[cfg(target_os = "windows")]
use super::pe::pe_metadata;
#[cfg(target_os = "windows")]
use common::windows::PeInfo;

#[cfg(target_os = "macos")]
use common::macos::MachoInfo;

#[cfg(target_os = "linux")]
use super::executable::elf_metadata;
#[cfg(target_os = "linux")]
use common::linux::ElfInfo;

/// Get process listing.
pub fn proc_list(hashes: &Hashes, binary_data: bool) -> Result<Vec<Processes>, ProcessError> {
    let mut proc = System::new();
    let mut processes_list: Vec<Processes> = Vec::new();

    proc.refresh_processes_specifics(ProcessesToUpdate::All, ProcessRefreshKind::everything());
    if proc.processes().is_empty() {
        return Err(ProcessError::Empty);
    }

    for process in proc.processes().values() {
        let system_proc = proc_info(process, hashes, binary_data);
        processes_list.push(system_proc);
    }
    Ok(processes_list)
}

// Get the process info data
fn proc_info(process: &Process, hashes: &Hashes, binary_data: bool) -> Processes {
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
        start_time: unixepoch_to_iso(&(process.start_time() as i64)),
        uid,
        gid,
        md5: String::new(),
        sha1: String::new(),
        sha256: String::new(),
        binary_info: Vec::new(),
    };

    if binary_data && !system_proc.full_path.is_empty() {
        let binary_results = executable_metadata(&system_proc.full_path);
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

    #[cfg(target_os = "windows")]
    let first_proc = 0;
    #[cfg(target_family = "unix")]
    let first_proc = 1;

    if process.pid().as_u32() != first_proc {
        let parent_pid = process.parent();
        if let Some(ppid) = parent_pid {
            system_proc.ppid = ppid.as_u32();
        } else {
            warn!("No Parent PID for: {}", process.pid());
        }
    }

    if hashes.md5 || hashes.sha1 || hashes.sha256 {
        (system_proc.md5, system_proc.sha1, system_proc.sha256) =
            hash_file(hashes, &system_proc.full_path);
    }

    system_proc
}

#[cfg(target_os = "macos")]
/// Get executable metadata
fn executable_metadata(path: &str) -> Result<Vec<MachoInfo>, ProcessError> {
    use super::macho::macho_metadata;

    macho_metadata(path)
}

#[cfg(target_os = "linux")]
/// Get executable metadata
fn executable_metadata(path: &str) -> Result<Vec<ElfInfo>, ProcessError> {
    elf_metadata(path)
}

#[cfg(target_os = "windows")]
/// Get executable metadata
fn executable_metadata(path: &str) -> Result<Vec<PeInfo>, ProcessError> {
    pe_metadata(path)
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::processes::process::executable_metadata;
    use crate::artifacts::os::processes::process::{proc_info, proc_list};
    use common::files::Hashes;
    use common::system::Processes;
    use sysinfo::{ProcessesToUpdate, System};

    #[test]
    fn test_proc_list() {
        let hashes = Hashes {
            md5: false,
            sha1: false,
            sha256: false,
        };

        let data = proc_list(&hashes, false).unwrap();
        assert!(data.len() > 10);
    }

    #[test]
    fn test_proc_info() {
        let mut proc = System::new();
        let mut processes_list: Vec<Processes> = Vec::new();

        proc.refresh_processes(ProcessesToUpdate::All);

        let hashes = Hashes {
            md5: true,
            sha1: true,
            sha256: true,
        };

        for process in proc.processes().values() {
            let system_proc = proc_info(process, &hashes, false);
            processes_list.push(system_proc);
        }
        assert!(processes_list.len() > 10);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_executable_metadata() {
        let test_path = "/bin/ls";
        let results = executable_metadata(test_path).unwrap();

        assert_eq!(results.len(), 2);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_executable_metadata() {
        let test_path = "/bin/ls";
        let results = executable_metadata(test_path).unwrap();

        assert_eq!(results.len(), 1);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_executable_metadata() {
        let test_path = "C:\\Windows\\explorer.exe";
        let results = executable_metadata(test_path).unwrap();

        assert_eq!(results.len(), 1);
    }
}
