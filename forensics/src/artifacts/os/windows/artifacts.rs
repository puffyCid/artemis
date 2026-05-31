use super::{
    accounts::parser::grab_users, amcache::parser::grab_amcache, bits::parser::grab_bits,
    error::WinArtifactError, eventlogs::parser::grab_eventlogs, jumplists::parser::grab_jumplists,
    mft::parser::grab_mft, ntfs::parser::ntfs_filelist, outlook::parser::grab_outlook,
    prefetch::parser::grab_prefetch, recyclebin::parser::grab_recycle_bin,
    registry::parser::parse_registry, search::parser::grab_search, services::parser::grab_services,
    shellbags::parser::grab_shellbags, shimcache::parser::grab_shimcache,
    shimdb::parser::grab_shimdb, shortcuts::parser::grab_lnk_directory, srum::parser::grab_srum,
    tasks::parser::grab_tasks, userassist::parser::grab_userassist, usnjrnl::parser::grab_usnjrnl,
    wmi::parser::grab_wmi_persist,
};
use crate::output2::manager::OutputManager;
use crate::output2::record::serialize_records_to_stream;
use crate::structs::artifacts::os::windows::{
    AmcacheOptions, BitsOptions, EventLogsOptions, JumplistsOptions, MftOptions, OutlookOptions,
    PrefetchOptions, RawFilesOptions, RecycleBinOptions, RegistryOptions, SearchOptions,
    ServicesOptions, ShellbagsOptions, ShimcacheOptions, ShimdbOptions, ShortcutOptions,
    SrumOptions, TasksOptions, UserAssistOptions, UsnJrnlOptions, WindowsUserOptions,
    WmiPersistOptions,
};
use log::error;

/// Parse the Windows `Prefetch` artifact
pub(crate) fn prefetch(
    options: &PrefetchOptions,
    manager: &mut OutputManager,
) -> Result<(), WinArtifactError> {
    let pf_results = grab_prefetch(options);
    let entries = match pf_results {
        Ok(results) => results,
        Err(err) => {
            error!("[forensics] Artemis failed to parse Prefetch: {err:?}");
            return Err(WinArtifactError::Prefetch);
        }
    };

    if entries.is_empty() {
        return Ok(());
    }

    let mut records = match serialize_records_to_stream(entries) {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Failed to serialize prefetch: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };

    let artifact_name = "prefetch";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("[forensics] Failed to output prefetch: {err:?}");
        return Err(WinArtifactError::Output);
    }

    Ok(())
}

/// Parse the Windows `EventLogs` artifact
pub(crate) fn eventlogs(
    options: &EventLogsOptions,
    manager: &mut OutputManager,
) -> Result<(), WinArtifactError> {
    // Since we may be parsing multiple files, let the parser handle outputting the data
    if let Err(err) = grab_eventlogs(options, manager) {
        error!("[forensics] Artemis failed to parse EventLogs: {err:?}");
        return Err(WinArtifactError::EventLogs);
    }

    Ok(())
}

/// Parse the Windows `Registry` artifact
pub(crate) fn registry(
    options: &RegistryOptions,
    manager: &mut OutputManager,
) -> Result<(), WinArtifactError> {
    // Since we may be parsing multiple files, let the parser handle outputting the data
    if let Err(err) = parse_registry(options, manager) {
        error!("[forensics] Failed to parse Registry: {err:?}");
        return Err(WinArtifactError::Registry);
    }

    Ok(())
}

/// Parse the Windows `NTFS` artifact
pub(crate) fn raw_filelist(
    options: &RawFilesOptions,
    manager: &mut OutputManager,
) -> Result<(), WinArtifactError> {
    // Since we may be walking the file system, let the parser handle outputting the data
    if let Err(err) = ntfs_filelist(options, manager) {
        error!("[forensics] Failed to parse NTFS: {err:?}");
        return Err(WinArtifactError::Ntfs);
    }

    Ok(())
}

/// Get Windows `Shimdatabase(s)`
pub(crate) fn shimdb(
    options: &ShimdbOptions,
    manager: &mut OutputManager,
) -> Result<(), WinArtifactError> {
    let shimdb_results = grab_shimdb(options);
    let entries = match shimdb_results {
        Ok(results) => results,
        Err(err) => {
            error!("[forensics] Artemis failed to parse shimdb: {err:?}");
            return Err(WinArtifactError::Shimdb);
        }
    };

    if entries.is_empty() {
        return Ok(());
    }

    let mut records = match serialize_records_to_stream(entries) {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Failed to serialize shimdb: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };

    let artifact_name = "shimdb";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("[forensics] Failed to output shimdb: {err:?}");
        return Err(WinArtifactError::Output);
    }

    Ok(())
}

/// Get Windows `UserAssist` entries
pub(crate) fn userassist(
    options: &UserAssistOptions,
    manager: &mut OutputManager,
) -> Result<(), WinArtifactError> {
    let assist_results = grab_userassist(options);
    let entries = match assist_results {
        Ok(results) => results,
        Err(err) => {
            error!("[forensics] Artemis failed to parse userassist: {err:?}");
            return Err(WinArtifactError::UserAssist);
        }
    };

    if entries.is_empty() {
        return Ok(());
    }

    let mut records = match serialize_records_to_stream(entries) {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Failed to serialize userassist: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };

    let artifact_name = "userassist";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("[forensics] Failed to output userassist: {err:?}");
        return Err(WinArtifactError::Output);
    }

    Ok(())
}

/// Get Windows `Shimcache` entries
pub(crate) fn shimcache(
    options: &ShimcacheOptions,
    manager: &mut OutputManager,
) -> Result<(), WinArtifactError> {
    let shim_results = grab_shimcache(options);
    let entries = match shim_results {
        Ok(results) => results,
        Err(err) => {
            error!("[forensics] Artemis failed to parse shimcache: {err:?}");
            return Err(WinArtifactError::Shimcache);
        }
    };

    if entries.is_empty() {
        return Ok(());
    }

    let mut records = match serialize_records_to_stream(entries) {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Failed to serialize shimcache: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };

    let artifact_name = "shimcache";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("[forensics] Failed to output shimcache: {err:?}");
        return Err(WinArtifactError::Output);
    }

    Ok(())
}

/// Get Windows `Shellbag` entries
pub(crate) fn shellbags(
    options: &ShellbagsOptions,
    manager: &mut OutputManager,
) -> Result<(), WinArtifactError> {
    let mut entries = Vec::new();
    let artifact_result = grab_shellbags(options);
    match artifact_result {
        Ok(mut result) => entries.append(&mut result),
        Err(err) => {
            error!("[forensics] Artemis failed to parse Shellbags: {err:?}");
            return Err(WinArtifactError::Shellbag);
        }
    }

    if entries.is_empty() {
        return Ok(());
    }

    let mut records = match serialize_records_to_stream(entries) {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Failed to serialize shellbags: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };

    let artifact_name = "shellbags";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("[forensics] Failed to output shellbags: {err:?}");
        return Err(WinArtifactError::Output);
    }

    Ok(())
}

/// Get Windows `Amcache` entries
pub(crate) fn amcache(
    options: &AmcacheOptions,
    manager: &mut OutputManager,
) -> Result<(), WinArtifactError> {
    let mut entries = Vec::new();
    let artifact_result = grab_amcache(options);
    match artifact_result {
        Ok(mut result) => entries.append(&mut result),
        Err(err) => {
            error!("[forensics] Artemis failed to parse Amcache: {err:?}");
            return Err(WinArtifactError::Amcache);
        }
    }

    if entries.is_empty() {
        return Ok(());
    }

    let mut records = match serialize_records_to_stream(entries) {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Failed to serialize amcache: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };

    let artifact_name = "amcache";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("[forensics] Failed to output amcache: {err:?}");
        return Err(WinArtifactError::Output);
    }

    Ok(())
}

/// Get Windows `Shortcut` data
pub(crate) fn shortcuts(
    options: &ShortcutOptions,
    manager: &mut OutputManager,
) -> Result<(), WinArtifactError> {
    let artifact_result = grab_lnk_directory(&options.dir);
    let entries = match artifact_result {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Artemis failed to parse Shortcut data: {err:?}");
            return Err(WinArtifactError::Shortcuts);
        }
    };

    if entries.is_empty() {
        return Ok(());
    }

    let mut records = match serialize_records_to_stream(entries) {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Failed to serialize shortcuts: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };

    let artifact_name = "shortcuts";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("[forensics] Failed to output shortcuts: {err:?}");
        return Err(WinArtifactError::Output);
    }

    Ok(())
}

/// Get Windows `UsnJrnl` data
pub(crate) fn usnjrnl(
    options: &UsnJrnlOptions,
    manager: &mut OutputManager,
) -> Result<(), WinArtifactError> {
    if let Err(err) = grab_usnjrnl(options, manager) {
        error!("[forensics] Artemis failed to parse UsnJrnl data: {err:?}");
        return Err(WinArtifactError::UsnJrnl);
    }

    Ok(())
}

/// Get Windows `Bits` data
pub(crate) fn bits(
    options: &BitsOptions,
    manager: &mut OutputManager,
) -> Result<(), WinArtifactError> {
    let artifact_result = grab_bits(options);
    let entries = match artifact_result {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Artemis failed to parse Bits data: {err:?}");
            return Err(WinArtifactError::Bits);
        }
    };

    if entries.is_empty() {
        return Ok(());
    }

    let mut records = match serialize_records_to_stream(entries) {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Failed to serialize bits: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };

    let artifact_name = "bits";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("[forensics] Failed to output bits: {err:?}");
        return Err(WinArtifactError::Output);
    }

    Ok(())
}

/// Get Windows `SRUM` data
pub(crate) fn srum(
    options: &SrumOptions,
    manager: &mut OutputManager,
) -> Result<(), WinArtifactError> {
    if let Err(err) = grab_srum(options, manager) {
        error!("[forensics] Artemis failed to parse SRUM data: {err:?}");
        return Err(WinArtifactError::Srum);
    }

    Ok(())
}

/// Get Windows `Search` data
pub(crate) fn search(
    options: &SearchOptions,
    manager: &mut OutputManager,
) -> Result<(), WinArtifactError> {
    if let Err(err) = grab_search(options, manager) {
        error!("[forensics] Artemis failed to parse Search data: {err:?}");
        return Err(WinArtifactError::Search);
    }

    Ok(())
}

/// Get Windows `Users` info
pub(crate) fn users_windows(
    options: &WindowsUserOptions,
    manager: &mut OutputManager,
) -> Result<(), WinArtifactError> {
    let artifact_result = grab_users(options);
    let entries = match artifact_result {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Artemis failed to parse User info: {err:?}");
            return Err(WinArtifactError::Users);
        }
    };

    if entries.is_empty() {
        return Ok(());
    }

    let mut records = match serialize_records_to_stream(entries) {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Failed to serialize users-windows: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };

    let artifact_name = "users-windows";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("[forensics] Failed to output users-windows: {err:?}");
        return Err(WinArtifactError::Output);
    }

    Ok(())
}

/// Parse the Windows `Schedule Tasks` artifact
pub(crate) fn tasks(
    options: &TasksOptions,
    manager: &mut OutputManager,
) -> Result<(), WinArtifactError> {
    if let Err(err) = grab_tasks(options, manager) {
        error!("[forensics] Artemis failed to parse Tasks: {err:?}");
        return Err(WinArtifactError::Tasks);
    }

    Ok(())
}

/// Parse the Windows `Services` artifact
pub(crate) fn services(
    options: &ServicesOptions,
    manager: &mut OutputManager,
) -> Result<(), WinArtifactError> {
    let service_results = grab_services(options);
    let entries = match service_results {
        Ok(results) => results,
        Err(err) => {
            error!("[forensics] Artemis failed to parse Services: {err:?}");
            return Err(WinArtifactError::Services);
        }
    };

    if entries.is_empty() {
        return Ok(());
    }

    let mut records = match serialize_records_to_stream(entries) {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Failed to serialize services: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };

    let artifact_name = "services";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("[forensics] Failed to output services: {err:?}");
        return Err(WinArtifactError::Output);
    }

    Ok(())
}

/// Parse the Windows `Jumplists` artifact
pub(crate) fn jumplists(
    options: &JumplistsOptions,
    manager: &mut OutputManager,
) -> Result<(), WinArtifactError> {
    let jumplist_result = grab_jumplists(options);
    let entries = match jumplist_result {
        Ok(results) => results,
        Err(err) => {
            error!("[forensics] Artemis failed to parse Jumplists: {err:?}");
            return Err(WinArtifactError::Jumplists);
        }
    };

    if entries.is_empty() {
        return Ok(());
    }

    let mut records = match serialize_records_to_stream(entries) {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Failed to serialize jumplists: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };

    let artifact_name = "jumplists";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("[forensics] Failed to output jumplists: {err:?}");
        return Err(WinArtifactError::Output);
    }

    Ok(())
}

/// Parse the Windows `Recycle Bin` artifact
pub(crate) fn recycle_bin(
    options: &RecycleBinOptions,
    manager: &mut OutputManager,
) -> Result<(), WinArtifactError> {
    let bin_result = grab_recycle_bin(options);
    let entries = match bin_result {
        Ok(results) => results,
        Err(err) => {
            error!("[forensics] Artemis failed to parse Recycle Bin: {err:?}");
            return Err(WinArtifactError::RecycleBin);
        }
    };

    if entries.is_empty() {
        return Ok(());
    }

    let mut records = match serialize_records_to_stream(entries) {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Failed to serialize recyclebin: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };

    let artifact_name = "recyclebin";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("[forensics] Failed to output recyclebin: {err:?}");
        return Err(WinArtifactError::Output);
    }

    Ok(())
}

/// Parse the Windows `WMI Persist` artifact
pub(crate) fn wmi_persist(
    options: &WmiPersistOptions,
    manager: &mut OutputManager,
) -> Result<(), WinArtifactError> {
    let wmi_result = grab_wmi_persist(options);
    let entries = match wmi_result {
        Ok(results) => results,
        Err(err) => {
            error!("[forensics] Artemis failed to parse WMI Persistence: {err:?}");
            return Err(WinArtifactError::WmiPersist);
        }
    };

    if entries.is_empty() {
        return Ok(());
    }

    let mut records = match serialize_records_to_stream(entries) {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Failed to serialize wmipersist: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };

    let artifact_name = "wmipersist";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("[forensics] Failed to output wmipersist: {err:?}");
        return Err(WinArtifactError::Output);
    }

    Ok(())
}

/// Parse the Windows `Outlook` artifact
pub(crate) fn outlook(
    options: &OutlookOptions,
    manager: &mut OutputManager,
) -> Result<(), WinArtifactError> {
    if let Err(err) = grab_outlook(options, manager) {
        error!("[forensics] Artemis failed to parse Outlook: {err:?}");
        return Err(WinArtifactError::Outlook);
    }

    Ok(())
}

/// Parse the Windows `MFT` artifact
pub(crate) fn mft(
    options: &MftOptions,
    manager: &mut OutputManager,
) -> Result<(), WinArtifactError> {
    if let Err(err) = grab_mft(options, manager) {
        error!("[forensics] Artemis failed to parse MFT: {err:?}");
        return Err(WinArtifactError::Mft);
    }

    Ok(())
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use crate::{
        artifacts::os::windows::artifacts::{
            amcache, bits, eventlogs, jumplists, mft, prefetch, raw_filelist, recycle_bin,
            registry, search, services, shellbags, shimcache, shimdb, shortcuts, srum, tasks,
            userassist, users_windows, usnjrnl, wmi_persist,
        },
        output2::{
            config::{OutputConfig, OutputDestination, OutputFormat},
            manager::OutputManager,
        },
        structs::artifacts::os::windows::{
            AmcacheOptions, BitsOptions, EventLogsOptions, JumplistsOptions, MftOptions,
            PrefetchOptions, RawFilesOptions, RecycleBinOptions, RegistryOptions, SearchOptions,
            ServicesOptions, ShellbagsOptions, ShimcacheOptions, ShimdbOptions, ShortcutOptions,
            SrumOptions, TasksOptions, UserAssistOptions, UsnJrnlOptions, WindowsUserOptions,
            WmiPersistOptions,
        },
    };
    use std::path::PathBuf;

    fn output_options(name: &str, directory: &str, compress: bool) -> OutputManager {
        let config = OutputConfig {
            name: name.to_string(),
            directory: PathBuf::from(directory),
            format: OutputFormat::Jsonl,
            compress,
            endpoint_id: String::from("abcd"),
            destination: OutputDestination::Local,
            ..Default::default()
        };
        OutputManager::new(config).unwrap()
    }

    #[test]
    fn test_eventlogs() {
        let evt = EventLogsOptions {
            alt_file: None,
            alt_dir: None,
            dump_templates: false,
            include_templates: false,
            alt_template_file: None,
            only_templates: false,
        };
        let mut output = output_options("eventlogs_temp", "./tmp", true);

        let status = eventlogs(&evt, &mut output).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_shimdb() {
        let sdb = ShimdbOptions { alt_file: None };
        let mut output = output_options("shimdb_temp", "./tmp", false);

        let status = shimdb(&sdb, &mut output).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_prefetch() {
        let pf = PrefetchOptions { alt_dir: None };
        let mut output = output_options("prefetch_temp", "./tmp", false);

        let status = prefetch(&pf, &mut output).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_registry() {
        let options = RegistryOptions {
            user_hives: false,
            system_hives: true,
            path_regex: None,
            alt_file: None,
        };
        let mut output = output_options("reg_temp", "./tmp", true);

        let status = registry(&options, &mut output).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_raw_filelist() {
        let options = RawFilesOptions {
            drive_letter: 'C',
            start_path: String::from("C:\\"),
            depth: 1,
            recover_indx: false,
            md5: None,
            sha1: None,
            sha256: None,
            metadata: None,
            filename_regex: None,
            path_regex: None,
        };
        let mut output = output_options("rawfiles_temp", "./tmp", false);

        let status = raw_filelist(&options, &mut output).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_userassist() {
        let options = UserAssistOptions {
            alt_file: None,
            resolve_descriptions: Some(true),
        };
        let mut output = output_options("assist_temp", "./tmp", false);

        let status = userassist(&options, &mut output).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_shimcache() {
        let options = ShimcacheOptions { alt_file: None };
        let mut output = output_options("shimcache_temp", "./tmp", false);

        let status = shimcache(&options, &mut output).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_shellbags() {
        let options = ShellbagsOptions {
            alt_file: None,
            resolve_guids: false,
        };
        let mut output = output_options("bags_temp", "./tmp", false);

        let status = shellbags(&options, &mut output).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_amcache() {
        let options = AmcacheOptions { alt_file: None };
        let mut output = output_options("amcache_temp", "./tmp", false);

        let status = amcache(&options, &mut output).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    #[ignore = "Takes a long time"]
    fn test_usnjrnl() {
        let options = UsnJrnlOptions {
            alt_drive: None,
            alt_file: None,
            alt_mft: None,
        };
        let mut output = output_options("usn_temp", "./tmp", false);

        let status = usnjrnl(&options, &mut output).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_shortcuts() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/lnk/win11/*");

        let options = ShortcutOptions {
            dir: test_location.display().to_string(),
        };
        let mut output = output_options("shortcuts_temp", "./tmp", false);

        let status = shortcuts(&options, &mut output).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_bits() {
        let options = BitsOptions {
            alt_file: None,
            carve: false,
        };
        let mut output = output_options("bits_temp", "./tmp", false);

        let status = bits(&options, &mut output).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_srum() {
        let options = SrumOptions { alt_file: None };
        let mut output = output_options("srum_temp", "./tmp", false);

        let status = srum(&options, &mut output).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    #[ignore = "Takes a long time"]
    fn test_search() {
        let options = SearchOptions { alt_file: None };
        let mut output = output_options("search_temp", "./tmp", false);

        let status = search(&options, &mut output).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_wmipersist() {
        let options = WmiPersistOptions { alt_dir: None };
        let mut output = output_options("wmipersist_temp", "./tmp", false);

        let status = wmi_persist(&options, &mut output).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_users_windows() {
        let options = WindowsUserOptions { alt_file: None };
        let mut output = output_options("users_temp", "./tmp", false);

        let status = users_windows(&options, &mut output).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_tasks() {
        let options = TasksOptions { alt_file: None };
        let mut output = output_options("tasks_temp", "./tmp", false);

        let status = tasks(&options, &mut output).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_services() {
        let options = ServicesOptions { alt_file: None };
        let mut output = output_options("services_temp", "./tmp", false);

        let status = services(&options, &mut output).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_jumplists() {
        let options = JumplistsOptions { alt_dir: None };
        let mut output = output_options("jumplists_temp", "./tmp", false);

        let status = jumplists(&options, &mut output).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_recycle_bin() {
        let options = RecycleBinOptions { alt_file: None };
        let mut output = output_options("recyclebin_temp", "./tmp", false);

        let status = recycle_bin(&options, &mut output).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    #[ignore = "Takes time to run"]
    fn test_mft() {
        let options = MftOptions {
            alt_drive: None,
            alt_file: None,
        };
        let mut output = output_options("mft_temp", "./tmp", false);

        let status = mft(&options, &mut output).unwrap();
        assert_eq!(status, ());
    }
}
