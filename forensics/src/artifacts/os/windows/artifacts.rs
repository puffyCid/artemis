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
use crate::artifacts::output::output_artifact;
use crate::structs::artifacts::os::windows::{
    AmcacheOptions, BitsOptions, EventLogsOptions, JumplistsOptions, MftOptions, OutlookOptions,
    PrefetchOptions, RawFilesOptions, RecycleBinOptions, RegistryOptions, SearchOptions,
    ServicesOptions, ShellbagsOptions, ShimcacheOptions, ShimdbOptions, ShortcutOptions,
    SrumOptions, TasksOptions, UserAssistOptions, UsnJrnlOptions, WindowsUserOptions,
    WmiPersistOptions,
};
use crate::structs::toml::Output;
use crate::utils::time;
use log::error;
use serde_json::Value;

/// Parse the Windows `Prefetch` artifact
pub(crate) fn prefetch(
    options: &PrefetchOptions,
    output: &mut Output,
    filter: bool,
) -> Result<(), WinArtifactError> {
    let start_time = time::time_now();

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

    let serde_data_result = serde_json::to_value(entries);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[forensics] Failed to serialize prefetch: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };

    let output_name = "prefetch";
    output_data(&mut serde_data, output_name, output, start_time, filter)
}

/// Parse the Windows `EventLogs` artifact
pub(crate) fn eventlogs(
    options: &EventLogsOptions,
    output: &mut Output,
    filter: bool,
) -> Result<(), WinArtifactError> {
    // Since we may be parsing multiple files, let the parser handle outputting the data
    if let Err(err) = grab_eventlogs(options, output, filter) {
        error!("[forensics] Artemis failed to parse EventLogs: {err:?}");
        return Err(WinArtifactError::EventLogs);
    }

    Ok(())
}

/// Parse the Windows `Registry` artifact
pub(crate) fn registry(
    options: &RegistryOptions,
    output: &mut Output,
    filter: bool,
) -> Result<(), WinArtifactError> {
    // Since we may be parsing multiple files, let the parser handle outputting the data
    if let Err(err) = parse_registry(options, output, filter) {
        error!("[forensics] Failed to parse Registry: {err:?}");
        return Err(WinArtifactError::Registry);
    }

    Ok(())
}

/// Parse the Windows `NTFS` artifact
pub(crate) fn raw_filelist(
    options: &RawFilesOptions,
    output: &mut Output,
    filter: bool,
) -> Result<(), WinArtifactError> {
    // Since we may be walking the file system, let the parser handle outputting the data
    if let Err(err) = ntfs_filelist(options, output, filter) {
        error!("[forensics] Failed to parse NTFS: {err:?}");
        return Err(WinArtifactError::Ntfs);
    }

    Ok(())
}

/// Get Windows `Shimdatabase(s)`
pub(crate) fn shimdb(
    options: &ShimdbOptions,
    output: &mut Output,
    filter: bool,
) -> Result<(), WinArtifactError> {
    let start_time = time::time_now();
    let shimdb_results = grab_shimdb(options);
    let entries = match shimdb_results {
        Ok(results) => results,
        Err(err) => {
            error!("[forensics] Artemis failed to parse Shimdb: {err:?}");
            return Err(WinArtifactError::Shimdb);
        }
    };

    if entries.is_empty() {
        return Ok(());
    }

    let serde_data_result = serde_json::to_value(entries);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[forensics] Failed to serialize Shimdb: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };

    let output_name = "shimdb";
    output_data(&mut serde_data, output_name, output, start_time, filter)
}

/// Get Windows `UserAssist` entries
pub(crate) fn userassist(
    options: &UserAssistOptions,
    output: &mut Output,
    filter: bool,
) -> Result<(), WinArtifactError> {
    let start_time = time::time_now();

    let assist_results = grab_userassist(options);
    let entries = match assist_results {
        Ok(results) => results,
        Err(err) => {
            error!("[forensics] Artemis failed to parse UserAssist: {err:?}");
            return Err(WinArtifactError::UserAssist);
        }
    };

    if entries.is_empty() {
        return Ok(());
    }

    let serde_data_result = serde_json::to_value(entries);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[forensics] Failed to serialize UserAssist: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };
    let output_name = "userassist";
    output_data(&mut serde_data, output_name, output, start_time, filter)
}

/// Get Windows `Shimcache` entries
pub(crate) fn shimcache(
    options: &ShimcacheOptions,
    output: &mut Output,
    filter: bool,
) -> Result<(), WinArtifactError> {
    let start_time = time::time_now();

    let shim_results = grab_shimcache(options);
    let entries = match shim_results {
        Ok(results) => results,
        Err(err) => {
            error!("[forensics] Artemis failed to parse Shimcache: {err:?}");
            return Err(WinArtifactError::Shimcache);
        }
    };

    if entries.is_empty() {
        return Ok(());
    }

    let serde_data_result = serde_json::to_value(entries);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[forensics] Failed to serialize Shimcache: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };
    let output_name = "shimcache";
    output_data(&mut serde_data, output_name, output, start_time, filter)
}

/// Get Windows `Shellbag` entries
pub(crate) fn shellbags(
    options: &ShellbagsOptions,
    output: &mut Output,
    filter: bool,
) -> Result<(), WinArtifactError> {
    let start_time = time::time_now();

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

    let serde_data_result = serde_json::to_value(entries);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[forensics] Failed to serialize Shellbags: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };
    let output_name = "shellbags";
    output_data(&mut serde_data, output_name, output, start_time, filter)
}

/// Get Windows `Amcache` entries
pub(crate) fn amcache(
    options: &AmcacheOptions,
    output: &mut Output,
    filter: bool,
) -> Result<(), WinArtifactError> {
    let start_time = time::time_now();

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

    let serde_data_result = serde_json::to_value(entries);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[forensics] Failed to serialize Amcache: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };
    let output_name = "amcache";
    output_data(&mut serde_data, output_name, output, start_time, filter)
}

/// Get Windows `Shortcut` data
pub(crate) fn shortcuts(
    options: &ShortcutOptions,
    output: &mut Output,
    filter: bool,
) -> Result<(), WinArtifactError> {
    let start_time = time::time_now();

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

    let serde_data_result = serde_json::to_value(entries);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[forensics] Failed to serialize shortcuts: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };
    let output_name = "shortcuts";
    output_data(&mut serde_data, output_name, output, start_time, filter)
}

/// Get Windows `UsnJrnl` data
pub(crate) fn usnjrnl(
    options: &UsnJrnlOptions,
    output: &mut Output,
    filter: bool,
) -> Result<(), WinArtifactError> {
    if let Err(err) = grab_usnjrnl(options, output, filter) {
        error!("[forensics] Artemis failed to parse UsnJrnl data: {err:?}");
        return Err(WinArtifactError::UsnJrnl);
    }

    Ok(())
}

/// Get Windows `Bits` data
pub(crate) fn bits(
    options: &BitsOptions,
    output: &mut Output,
    filter: bool,
) -> Result<(), WinArtifactError> {
    let start_time = time::time_now();

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

    let serde_data_result = serde_json::to_value(entries);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[forensics] Failed to serialize bits: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };
    let output_name = "bits";
    output_data(&mut serde_data, output_name, output, start_time, filter)
}

/// Get Windows `SRUM` data
pub(crate) fn srum(
    options: &SrumOptions,
    output: &mut Output,
    filter: bool,
) -> Result<(), WinArtifactError> {
    if let Err(err) = grab_srum(options, output, filter) {
        error!("[forensics] Artemis failed to parse SRUM data: {err:?}");
        return Err(WinArtifactError::Srum);
    }

    Ok(())
}

/// Get Windows `Search` data
pub(crate) fn search(
    options: &SearchOptions,
    output: &mut Output,
    filter: bool,
) -> Result<(), WinArtifactError> {
    if let Err(err) = grab_search(options, output, filter) {
        error!("[forensics] Artemis failed to parse Search data: {err:?}");
        return Err(WinArtifactError::Search);
    }

    Ok(())
}

/// Get Windows `Users` info
pub(crate) fn users_windows(
    options: &WindowsUserOptions,
    output: &mut Output,
    filter: bool,
) -> Result<(), WinArtifactError> {
    let start_time = time::time_now();

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

    let serde_data_result = serde_json::to_value(entries);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[forensics] Failed to serialize users: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };
    let output_name = "users-windows";
    output_data(&mut serde_data, output_name, output, start_time, filter)
}

/// Parse the Windows `Schedule Tasks` artifact
pub(crate) fn tasks(
    options: &TasksOptions,
    output: &mut Output,
    filter: bool,
) -> Result<(), WinArtifactError> {
    if let Err(err) = grab_tasks(options, output, filter) {
        error!("[forensics] Artemis failed to parse Tasks: {err:?}");
        return Err(WinArtifactError::Tasks);
    }

    Ok(())
}

/// Parse the Windows `Services` artifact
pub(crate) fn services(
    options: &ServicesOptions,
    output: &mut Output,
    filter: bool,
) -> Result<(), WinArtifactError> {
    let start_time = time::time_now();

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

    let serde_data_result = serde_json::to_value(entries);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[forensics] Failed to serialize services: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };

    let output_name = "services";
    output_data(&mut serde_data, output_name, output, start_time, filter)
}

/// Parse the Windows `Jumplists` artifact
pub(crate) fn jumplists(
    options: &JumplistsOptions,
    output: &mut Output,
    filter: bool,
) -> Result<(), WinArtifactError> {
    let start_time = time::time_now();

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

    let serde_data_result = serde_json::to_value(entries);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[forensics] Failed to serialize jumplists: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };

    let output_name = "jumplists";
    output_data(&mut serde_data, output_name, output, start_time, filter)
}

/// Parse the Windows `Recycle Bin` artifact
pub(crate) fn recycle_bin(
    options: &RecycleBinOptions,
    output: &mut Output,
    filter: bool,
) -> Result<(), WinArtifactError> {
    let start_time = time::time_now();

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

    let serde_data_result = serde_json::to_value(entries);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[forensics] Failed to serialize recycle bin: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };

    let output_name = "recyclebin";
    output_data(&mut serde_data, output_name, output, start_time, filter)
}

/// Parse the Windows `WMI Persist` artifact
pub(crate) fn wmi_persist(
    options: &WmiPersistOptions,
    output: &mut Output,
    filter: bool,
) -> Result<(), WinArtifactError> {
    let start_time = time::time_now();

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

    let serde_data_result = serde_json::to_value(entries);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[forensics] Failed to serialize recycle bin: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };

    let output_name = "wmipersist";
    output_data(&mut serde_data, output_name, output, start_time, filter)
}

/// Parse the Windows `Outlook` artifact
pub(crate) fn outlook(
    options: &OutlookOptions,
    output: &mut Output,
    filter: bool,
) -> Result<(), WinArtifactError> {
    if let Err(err) = grab_outlook(options, output, filter) {
        error!("[forensics] Artemis failed to parse Outlook: {err:?}");
        return Err(WinArtifactError::Outlook);
    }

    Ok(())
}

/// Parse the Windows `MFT` artifact
pub(crate) fn mft(
    options: &MftOptions,
    output: &mut Output,
    filter: bool,
) -> Result<(), WinArtifactError> {
    if let Err(err) = grab_mft(options, output, filter) {
        error!("[forensics] Artemis failed to parse MFT: {err:?}");
        return Err(WinArtifactError::Mft);
    }

    Ok(())
}

/// Output Windows artifacts
pub(crate) fn output_data(
    serde_data: &mut Value,
    output_name: &str,
    output: &mut Output,
    start_time: u64,
    filter: bool,
) -> Result<(), WinArtifactError> {
    let status = output_artifact(serde_data, output_name, output, start_time, filter);
    if let Err(result) = status {
        error!("[forensics] Could not output data: {result:?}");
        return Err(WinArtifactError::Output);
    }
    Ok(())
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use crate::{
        artifacts::os::windows::artifacts::{
            amcache, bits, eventlogs, jumplists, mft, output_data, prefetch, raw_filelist,
            recycle_bin, registry, search, services, shellbags, shimcache, shimdb, shortcuts, srum,
            tasks, userassist, users_windows, usnjrnl, wmi_persist,
        },
        structs::{
            artifacts::os::windows::{
                AmcacheOptions, BitsOptions, EventLogsOptions, JumplistsOptions, MftOptions,
                PrefetchOptions, RawFilesOptions, RecycleBinOptions, RegistryOptions,
                SearchOptions, ServicesOptions, ShellbagsOptions, ShimcacheOptions, ShimdbOptions,
                ShortcutOptions, SrumOptions, TasksOptions, UserAssistOptions, UsnJrnlOptions,
                WindowsUserOptions, WmiPersistOptions,
            },
            toml::Output,
        },
        utils::time,
    };
    use serde_json::json;
    use std::path::PathBuf;

    fn output_options(name: &str, format: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: format.to_string(),
            compress,
            endpoint_id: String::from("abcd"),
            output: String::from("local"),
            ..Default::default()
        }
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
        let mut output = output_options("eventlogs_temp", "json", "./tmp", true);

        let status = eventlogs(&evt, &mut output, false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_shimdb() {
        let sdb = ShimdbOptions { alt_file: None };
        let mut output = output_options("shimdb_temp", "json", "./tmp", false);

        let status = shimdb(&sdb, &mut output, false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_prefetch() {
        let pf = PrefetchOptions { alt_dir: None };
        let mut output = output_options("prefetch_temp", "json", "./tmp", false);

        let status = prefetch(&pf, &mut output, false).unwrap();
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
        let mut output = output_options("reg_temp", "json", "./tmp", true);

        let status = registry(&options, &mut output, false).unwrap();
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
        let mut output = output_options("rawfiles_temp", "json", "./tmp", false);

        let status = raw_filelist(&options, &mut output, false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_userassist() {
        let options = UserAssistOptions {
            alt_file: None,
            resolve_descriptions: Some(true),
        };
        let mut output = output_options("assist_temp", "json", "./tmp", false);

        let status = userassist(&options, &mut output, false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_shimcache() {
        let options = ShimcacheOptions { alt_file: None };
        let mut output = output_options("shimcache_temp", "json", "./tmp", false);

        let status = shimcache(&options, &mut output, false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_shellbags() {
        let options = ShellbagsOptions {
            alt_file: None,
            resolve_guids: false,
        };
        let mut output = output_options("bags_temp", "json", "./tmp", false);

        let status = shellbags(&options, &mut output, false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_amcache() {
        let options = AmcacheOptions { alt_file: None };
        let mut output = output_options("amcache_temp", "json", "./tmp", false);

        let status = amcache(&options, &mut output, false).unwrap();
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
        let mut output = output_options("usn_temp", "json", "./tmp", false);

        let status = usnjrnl(&options, &mut output, false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_shortcuts() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/lnk/win11/*");

        let options = ShortcutOptions {
            dir: test_location.display().to_string(),
        };
        let mut output = output_options("shortcuts_temp", "json", "./tmp", false);

        let status = shortcuts(&options, &mut output, false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_bits() {
        let options = BitsOptions {
            alt_file: None,
            carve: false,
        };
        let mut output = output_options("bits_temp", "json", "./tmp", false);

        let status = bits(&options, &mut output, false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_srum() {
        let options = SrumOptions { alt_file: None };
        let mut output = output_options("srum_temp", "json", "./tmp", false);

        let status = srum(&options, &mut output, false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    #[ignore = "Takes a long time"]
    fn test_search() {
        let options = SearchOptions { alt_file: None };
        let mut output = output_options("search_temp", "json", "./tmp", false);

        let status = search(&options, &mut output, false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_wmipersist() {
        let options = WmiPersistOptions { alt_dir: None };
        let mut output = output_options("wmipersist_temp", "json", "./tmp", false);

        let status = wmi_persist(&options, &mut output, false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_users_windows() {
        let options = WindowsUserOptions { alt_file: None };
        let mut output = output_options("users_temp", "json", "./tmp", false);

        let status = users_windows(&options, &mut output, false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_tasks() {
        let options = TasksOptions { alt_file: None };
        let mut output = output_options("tasks_temp", "json", "./tmp", false);

        let status = tasks(&options, &mut output, false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_services() {
        let options = ServicesOptions { alt_file: None };
        let mut output = output_options("services_temp", "json", "./tmp", false);

        let status = services(&options, &mut output, false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_jumplists() {
        let options = JumplistsOptions { alt_dir: None };
        let mut output = output_options("jumplists_temp", "json", "./tmp", false);

        let status = jumplists(&options, &mut output, false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_recycle_bin() {
        let options = RecycleBinOptions { alt_file: None };
        let mut output = output_options("recyclebin_temp", "json", "./tmp", false);

        let status = recycle_bin(&options, &mut output, false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    #[ignore = "Takes time to run"]
    fn test_mft() {
        let options = MftOptions {
            alt_drive: None,
            alt_file: None,
        };
        let mut output = output_options("mft_temp", "json", "./tmp", false);

        let status = mft(&options, &mut output, false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_output_data() {
        let mut output = output_options("output_test", "json", "./tmp", false);
        let start_time = time::time_now();

        let name = "test";
        let mut data = json!({"test":"test"});
        let status = output_data(&mut data, name, &mut output, start_time, false).unwrap();
        assert_eq!(status, ());
    }
}
