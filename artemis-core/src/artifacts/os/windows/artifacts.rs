use super::search::parser::grab_search;
use super::tasks::parser::grab_tasks;
use super::{
    accounts::parser::grab_users, amcache::parser::grab_amcache, bits::parser::grab_bits,
    error::WinArtifactError, eventlogs::parser::grab_eventlogs, ntfs::parser::RawFilelist,
    prefetch::parser::grab_prefetch, registry::parser::RegistryData,
    shellbags::parser::grab_shellbags, shimcache::parser::grab_shimcache,
    shimdb::parser::grab_shimdb, shortcuts::parser::grab_lnk_directory, srum::parser::grab_srum,
    userassist::parser::grab_userassist, usnjrnl::parser::grab_usnjrnl,
};
use crate::artifacts::os::{
    files::filelisting::FileInfo, processes::process::Processes, systeminfo::info::SystemInfo,
};
use crate::filesystem::files::Hashes;
use crate::output::formats::{json::json_format, jsonl::jsonl_format};
use crate::runtime::deno::filter_script;
use crate::structs::artifacts::os::windows::{
    AmcacheOptions, BitsOptions, EventLogsOptions, PrefetchOptions, RawFilesOptions,
    RegistryOptions, SearchOptions, ShellbagsOptions, ShimcacheOptions, ShimdbOptions,
    ShortcutOptions, SrumOptions, TasksOptions, UserAssistOptions, UserOptions, UsnJrnlOptions,
};
use crate::{
    structs::artifacts::os::{files::FileOptions, processes::ProcessOptions},
    utils::{artemis_toml::Output, time},
};
use log::{error, warn};
use serde_json::Value;

/// Parse the Windows `Prefetch` artifact
pub(crate) fn prefetch(
    options: &PrefetchOptions,
    output: &mut Output,
    filter: &bool,
) -> Result<(), WinArtifactError> {
    let start_time = time::time_now();

    let pf_results = grab_prefetch(options);
    let pf_data = match pf_results {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Artemis failed to parse Prefetch: {err:?}");
            return Err(WinArtifactError::Prefetch);
        }
    };

    let serde_data_result = serde_json::to_value(pf_data);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize prefetch: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };

    let output_name = "prefetch";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Parse the Windows `EventLogs` artifact
pub(crate) fn eventlogs(
    options: &EventLogsOptions,
    output: &mut Output,
    filter: &bool,
) -> Result<(), WinArtifactError> {
    // Since we may be parsing multiple files, let the parser handle outputting the data
    let result = grab_eventlogs(options, output, filter);
    match result {
        Ok(_) => {}
        Err(err) => {
            error!("[artemis-core] Artemis failed to parse EventLogs: {err:?}");
            return Err(WinArtifactError::EventLogs);
        }
    };
    Ok(())
}

/// Parse the Windows `Registry` artifact
pub(crate) fn registry(
    options: &RegistryOptions,
    output: &mut Output,
    filter: &bool,
) -> Result<(), WinArtifactError> {
    // Since we may be parsing multiple files, let the parser handle outputting the data
    let result = RegistryData::parse_registry(options, output, filter);
    match result {
        Ok(_) => {}
        Err(err) => {
            error!("[artemis-core] Failed to parse Registry: {err:?}");
            return Err(WinArtifactError::Registry);
        }
    }
    Ok(())
}

/// Parse the Windows `NTFS` artifact
pub(crate) fn raw_filelist(
    options: &RawFilesOptions,
    output: &mut Output,
    filter: &bool,
) -> Result<(), WinArtifactError> {
    // Since we may be walking the file system, let the parser handle outputting the data
    let result = RawFilelist::raw_filelist(options, output, filter);
    match result {
        Ok(_) => {}
        Err(err) => {
            error!("[artemis-core] Failed to parse NTFS: {err:?}");
            return Err(WinArtifactError::Ntfs);
        }
    }
    Ok(())
}

/// Get Windows `Processes`
pub(crate) fn processes(
    options: &ProcessOptions,
    output: &mut Output,
    filter: &bool,
) -> Result<(), WinArtifactError> {
    let start_time = time::time_now();

    let hashes = Hashes {
        md5: options.md5,
        sha1: options.sha1,
        sha256: options.sha256,
    };

    let results = Processes::proc_list(&hashes, options.metadata);
    let proc_data = match results {
        Ok(data) => data,
        Err(err) => {
            warn!("[artemis-core] Artemis Windows failed to get process list: {err:?}");
            return Err(WinArtifactError::Process);
        }
    };

    let serde_data_result = serde_json::to_value(proc_data);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize processes: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };

    let output_name = "processes";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Get Windows `filelist`
pub(crate) fn files(
    options: &FileOptions,
    output: &mut Output,
    filter: &bool,
) -> Result<(), WinArtifactError> {
    let hashes = Hashes {
        md5: options.md5.unwrap_or(false),
        sha1: options.sha1.unwrap_or(false),
        sha256: options.sha256.unwrap_or(false),
    };
    let artifact_result = FileInfo::get_filelist(
        &options.start_path,
        options.depth.unwrap_or(1).into(),
        options.metadata.unwrap_or(false),
        &hashes,
        options.regex_filter.as_ref().unwrap_or(&String::new()),
        output,
        filter,
    );
    match artifact_result {
        Ok(_) => {}
        Err(err) => {
            error!("[artemis-core] Failed to get filelist: {err:?}");
            return Err(WinArtifactError::File);
        }
    }
    Ok(())
}

/// Get Windows `Shimdatabase(s)`
pub(crate) fn shimdb(
    options: &ShimdbOptions,
    output: &mut Output,
    filter: &bool,
) -> Result<(), WinArtifactError> {
    let start_time = time::time_now();
    let shimdb_results = grab_shimdb(options);
    let sdb_data = match shimdb_results {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Artemis failed to parse Shimdb: {err:?}");
            return Err(WinArtifactError::Shimdb);
        }
    };

    let serde_data_result = serde_json::to_value(sdb_data);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize Shimdb: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };

    let output_name = "shimdb";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Get Windows `Systeminfo`
pub(crate) fn systeminfo(output: &mut Output, filter: &bool) -> Result<(), WinArtifactError> {
    let start_time = time::time_now();

    let system_data = SystemInfo::get_info();
    let serde_data_result = serde_json::to_value(system_data);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize system data: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };

    let output_name = "systeminfo";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Get Windows `UserAssist` entries
pub(crate) fn userassist(
    options: &UserAssistOptions,
    output: &mut Output,
    filter: &bool,
) -> Result<(), WinArtifactError> {
    let start_time = time::time_now();

    let assist_results = grab_userassist(options);
    let assist_data = match assist_results {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Artemis failed to parse UserAssist: {err:?}");
            return Err(WinArtifactError::UserAssist);
        }
    };

    let serde_data_result = serde_json::to_value(assist_data);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize UserAssist: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };
    let output_name = "userassist";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Get Windows `Shimcache` entries
pub(crate) fn shimcache(
    options: &ShimcacheOptions,
    output: &mut Output,
    filter: &bool,
) -> Result<(), WinArtifactError> {
    let start_time = time::time_now();

    let shim_results = grab_shimcache(options);
    let shim_data = match shim_results {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Artemis failed to parse Shimcache: {err:?}");
            return Err(WinArtifactError::Shimcache);
        }
    };

    let serde_data_result = serde_json::to_value(shim_data);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize Shimcache: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };
    let output_name = "shimcache";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Get Windows `Shellbag` entries
pub(crate) fn shellbags(
    options: &ShellbagsOptions,
    output: &mut Output,
    filter: &bool,
) -> Result<(), WinArtifactError> {
    let start_time = time::time_now();

    let mut entries = Vec::new();
    let artifact_result = grab_shellbags(options);
    match artifact_result {
        Ok(mut result) => entries.append(&mut result),
        Err(err) => {
            error!("[artemis-core] Artemis failed to parse Shellbags: {err:?}");
            return Err(WinArtifactError::Shellbag);
        }
    }

    let serde_data_result = serde_json::to_value(entries);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize Shellbags: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };
    let output_name = "shellbags";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Get Windows `Amcache` entries
pub(crate) fn amcache(
    options: &AmcacheOptions,
    output: &mut Output,
    filter: &bool,
) -> Result<(), WinArtifactError> {
    let start_time = time::time_now();

    let mut entries = Vec::new();
    let artifact_result = grab_amcache(options);
    match artifact_result {
        Ok(mut result) => entries.append(&mut result),
        Err(err) => {
            error!("[artemis-core] Artemis failed to parse Amcache: {err:?}");
            return Err(WinArtifactError::Amcache);
        }
    }

    let serde_data_result = serde_json::to_value(entries);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize Amcache: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };
    let output_name = "amcache";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Get Windows `Shortcut` data
pub(crate) fn shortcuts(
    options: &ShortcutOptions,
    output: &mut Output,
    filter: &bool,
) -> Result<(), WinArtifactError> {
    let start_time = time::time_now();

    let artifact_result = grab_lnk_directory(&options.path);
    let entries = match artifact_result {
        Ok(result) => result,
        Err(err) => {
            error!("[artemis-core] Artemis failed to parse Shortcut data: {err:?}");
            return Err(WinArtifactError::Shortcuts);
        }
    };

    let serde_data_result = serde_json::to_value(entries);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize shortcuts: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };
    let output_name = "shortcuts";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Get Windows `UsnJrnl` data
pub(crate) fn usnjrnl(
    options: &UsnJrnlOptions,
    output: &mut Output,
    filter: &bool,
) -> Result<(), WinArtifactError> {
    let start_time = time::time_now();

    let artifact_result = grab_usnjrnl(options);
    let entries = match artifact_result {
        Ok(result) => result,
        Err(err) => {
            error!("[artemis-core] Artemis failed to parse UsnJrnl data: {err:?}");
            return Err(WinArtifactError::UsnJrnl);
        }
    };

    let serde_data_result = serde_json::to_value(entries);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize usnjrnl: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };
    let output_name = "usnjrnl";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Get Windows `Bits` data
pub(crate) fn bits(
    options: &BitsOptions,
    output: &mut Output,
    filter: &bool,
) -> Result<(), WinArtifactError> {
    let start_time = time::time_now();

    let artifact_result = grab_bits(options);
    let entries = match artifact_result {
        Ok(result) => result,
        Err(err) => {
            error!("[artemis-core] Artemis failed to parse Bits data: {err:?}");
            return Err(WinArtifactError::Bits);
        }
    };

    let serde_data_result = serde_json::to_value(entries);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize bits: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };
    let output_name = "bits";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Get Windows `SRUM` data
pub(crate) fn srum(
    options: &SrumOptions,
    output: &mut Output,
    filter: &bool,
) -> Result<(), WinArtifactError> {
    let artifact_result = grab_srum(options, output, filter);
    match artifact_result {
        Ok(_) => (),
        Err(err) => {
            error!("[artemis-core] Artemis failed to parse SRUM data: {err:?}");
            return Err(WinArtifactError::Srum);
        }
    };
    Ok(())
}

/// Get Windows `Search` data
pub(crate) fn search(
    options: &SearchOptions,
    output: &mut Output,
    filter: &bool,
) -> Result<(), WinArtifactError> {
    let artifact_result = grab_search(options, output, filter);
    match artifact_result {
        Ok(_) => (),
        Err(err) => {
            error!("[artemis-core] Artemis failed to parse Search data: {err:?}");
            return Err(WinArtifactError::Search);
        }
    };
    Ok(())
}

/// Get Windows `Users` info
pub(crate) fn users(
    options: &UserOptions,
    output: &mut Output,
    filter: &bool,
) -> Result<(), WinArtifactError> {
    let start_time = time::time_now();

    let artifact_result = grab_users(options);
    let entries = match artifact_result {
        Ok(result) => result,
        Err(err) => {
            error!("[artemis-core] Artemis failed to parse User info: {err:?}");
            return Err(WinArtifactError::Users);
        }
    };
    let serde_data_result = serde_json::to_value(entries);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize users: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };
    let output_name = "users";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Parse the Windows `Schedule Tasks` artifact
pub(crate) fn tasks(
    options: &TasksOptions,
    output: &mut Output,
    filter: &bool,
) -> Result<(), WinArtifactError> {
    let start_time = time::time_now();

    let task_results = grab_tasks(options);
    let task_data = match task_results {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Artemis failed to parse Tasks: {err:?}");
            return Err(WinArtifactError::Tasks);
        }
    };

    let serde_data_result = serde_json::to_value(task_data);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize tasks: {err:?}");
            return Err(WinArtifactError::Serialize);
        }
    };

    let output_name = "tasks";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Output Windows artifacts
pub(crate) fn output_data(
    serde_data: &Value,
    output_name: &str,
    output: &mut Output,
    start_time: &u64,
    filter: &bool,
) -> Result<(), WinArtifactError> {
    if *filter {
        if let Some(script) = &output.filter_script.clone() {
            let args = vec![serde_data.to_string(), output_name.to_string()];
            if let Some(name) = &output.filter_name.clone() {
                let filter_result = filter_script(output, &args, name, script);
                return match filter_result {
                    Ok(_) => Ok(()),
                    Err(err) => {
                        error!(
                            "[artemis-core] Could not apply filter script to windows data: {err:?}"
                        );
                        Err(WinArtifactError::FilterOutput)
                    }
                };
            }
            let filter_result = filter_script(output, &args, "UnknownFilterName", script);
            return match filter_result {
                Ok(_) => Ok(()),
                Err(err) => {
                    error!(
                        "[artemis-core] Could not apply unknown filter script to windows data: {err:?}"
                    );
                    Err(WinArtifactError::FilterOutput)
                }
            };
        }
    }

    let output_status = if output.format == "json" {
        json_format(serde_data, output_name, output, start_time)
    } else if output.format == "jsonl" {
        jsonl_format(serde_data, output_name, output, start_time)
    } else {
        error!(
            "[artemis-core] Unknown formatter provided: {}",
            output.format
        );
        return Err(WinArtifactError::Format);
    };
    match output_status {
        Ok(_) => {}
        Err(err) => {
            error!("[artemis-core] Could not output data: {err:?}");
            return Err(WinArtifactError::Output);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::artifacts::{
            amcache, bits, eventlogs, files, output_data, prefetch, processes, raw_filelist,
            registry, search, shellbags, shimcache, shimdb, shortcuts, srum, systeminfo,
            userassist, users, usnjrnl,
        },
        structs::artifacts::os::{
            files::FileOptions,
            processes::ProcessOptions,
            windows::{
                AmcacheOptions, BitsOptions, EventLogsOptions, PrefetchOptions, RawFilesOptions,
                RegistryOptions, SearchOptions, ShellbagsOptions, ShimcacheOptions, ShimdbOptions,
                ShortcutOptions, SrumOptions, UserAssistOptions, UserOptions, UsnJrnlOptions,
            },
        },
        utils::{artemis_toml::Output, time},
    };
    use std::path::PathBuf;

    fn output_options(name: &str, format: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: format.to_string(),
            compress,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: String::from("local"),
            filter_name: None,
            filter_script: None,
            logging: None,
        }
    }

    #[test]
    fn test_eventlogs() {
        let evt = EventLogsOptions {
            alt_drive: Some('C'),
        };
        let mut output = output_options("eventlogs_temp", "json", "./tmp", true);

        let status = eventlogs(&evt, &mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_shimdb() {
        let sdb = ShimdbOptions { alt_drive: None };
        let mut output = output_options("shimdb_temp", "json", "./tmp", false);

        let status = shimdb(&sdb, &mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_prefetch() {
        let pf = PrefetchOptions { alt_drive: None };
        let mut output = output_options("prefetch_temp", "json", "./tmp", false);

        let status = prefetch(&pf, &mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_registry() {
        let options = RegistryOptions {
            user_hives: false,
            system_hives: true,
            path_regex: None,
            alt_drive: None,
        };
        let mut output = output_options("reg_temp", "json", "./tmp", true);

        let status = registry(&options, &mut output, &false).unwrap();
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

        let status = raw_filelist(&options, &mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_processes() {
        let options = ProcessOptions {
            md5: false,
            sha1: false,
            sha256: false,
            metadata: false,
        };
        let mut output = output_options("proc_temp", "json", "./tmp", true);

        let status = processes(&options, &mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_files() {
        let options = FileOptions {
            md5: None,
            sha1: None,
            sha256: None,
            metadata: None,
            start_path: String::from("C:\\"),
            depth: None,
            regex_filter: None,
        };
        let mut output = output_options("files_temp", "json", "./tmp", false);

        let status = files(&options, &mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_systeminfo() {
        let mut output = output_options("info_temp", "json", "./tmp", false);

        let status = systeminfo(&mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_userassist() {
        let options = UserAssistOptions { alt_drive: None };
        let mut output = output_options("assist_temp", "json", "./tmp", false);

        let status = userassist(&options, &mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_shimcache() {
        let options = ShimcacheOptions { alt_drive: None };
        let mut output = output_options("shimcache_temp", "json", "./tmp", false);

        let status = shimcache(&options, &mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_shellbags() {
        let options = ShellbagsOptions {
            alt_drive: None,
            resolve_guids: false,
        };
        let mut output = output_options("bags_temp", "json", "./tmp", false);

        let status = shellbags(&options, &mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_amcache() {
        let options = AmcacheOptions { alt_drive: None };
        let mut output = output_options("amcache_temp", "json", "./tmp", false);

        let status = amcache(&options, &mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    #[ignore = "Takes a long time"]
    fn test_usnjrnl() {
        let options = UsnJrnlOptions { alt_drive: None };
        let mut output = output_options("usn_temp", "json", "./tmp", false);

        let status = usnjrnl(&options, &mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_shortcuts() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/lnk/win11");

        let options = ShortcutOptions {
            path: test_location.display().to_string(),
        };
        let mut output = output_options("shortcuts_temp", "json", "./tmp", false);

        let status = shortcuts(&options, &mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_bits() {
        let options = BitsOptions {
            alt_path: None,
            carve: false,
        };
        let mut output = output_options("bits_temp", "json", "./tmp", false);

        let status = bits(&options, &mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_srum() {
        let options = SrumOptions { alt_path: None };
        let mut output = output_options("srum_temp", "json", "./tmp", false);

        let status = srum(&options, &mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    #[ignore = "Takes a long time"]
    fn test_search() {
        let options = SearchOptions { alt_path: None };
        let mut output = output_options("search_temp", "json", "./tmp", false);

        let status = search(&options, &mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_users() {
        let options = UserOptions { alt_drive: None };
        let mut output = output_options("users_temp", "json", "./tmp", false);

        let status = users(&options, &mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_output_data() {
        let mut output = output_options("output_test", "json", "./tmp", false);
        let start_time = time::time_now();

        let name = "test";
        let data = serde_json::Value::String(String::from("test"));
        let status = output_data(&data, name, &mut output, &start_time, &false).unwrap();
        assert_eq!(status, ());
    }
}
