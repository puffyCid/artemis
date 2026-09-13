/**
 * Get a standard filelisting from the system.
 * Supports both Windows and macOS, in addition can parse executable metadata for each OS based on `FileOptions`
 * `PE` for Windows
 * `MACHO` for macOS
 * `ELF` for Linux
 *
 * On macOS the filelisting will read the firmlinks file at `/usr/share/firmlinks` and skip firmlink paths
 */
use super::error::FileError;
use crate::accessor::access::Accessor;
use crate::accessor::entry::handle::Timestamp;
use crate::accessor::io::reader::AccessorReader;
use crate::accessor::source::handle::SourceHandle;
use crate::accessor::walk::{WalkAccessor, WalkEntry};
use crate::artifacts::os::linux::executable::parser::parse_elf_reader;
use crate::artifacts::os::macos::macho::error::MachoError;
use crate::artifacts::os::macos::macho::parser::parse_macho_reader;
use crate::artifacts::os::systeminfo::info::{PlatformType, get_platform_enum};
use crate::artifacts::os::windows::pe::parser::parse_pe_reader;
use crate::filesystem::files::hash_reader;
use crate::output::manager::OutputManager;
use crate::output::record::serialize_records_to_stream;
use crate::structs::artifacts::os::files::FileOptions;
use crate::utils::regex_options::{create_regex, regex_check};
use common::files::EntryKind;
use common::files::FileInfo;
use common::files::Hashes;
use regex::Regex;
use serde_json::Value;
use tracing::{error, info, warn};

#[cfg(feature = "yarax")]
use crate::utils::yara::{extract_rule, scan_bytes};

/// Grab filelisting based on `FileOptions` provided
pub(crate) fn get_filelist(
    options: &FileOptions,
    manager: &mut OutputManager,
) -> Result<(), FileError> {
    let mut accessor = Accessor::with_defaults();

    let source = match accessor.open_source(&options.source) {
        Ok(result) => result,
        Err(err) => {
            error!("Could not open source: {err:?}");
            return Err(FileError::Filelisting);
        }
    };

    let mut walk = match WalkAccessor::new(&source, &options.start_path) {
        Ok(result) => result,
        Err(err) => {
            error!(
                "Could not start filelisting at {}: {err:?}",
                options.start_path
            );
            return Err(FileError::Filelisting);
        }
    };
    let depth = options.depth.unwrap_or(1);
    walk = walk.max_depth(depth);
    for exclude in options.exclude_directories.as_ref().unwrap_or(&Vec::new()) {
        walk = walk.exclude(exclude);
    }

    let mut rule = String::new();
    #[cfg(feature = "yarax")]
    if options.yara.as_ref().is_some_and(|s| !s.is_empty()) {
        // Unwrap is safe since we validate above
        rule = match extract_rule(options.yara.as_ref().unwrap()) {
            Ok(result) => result,
            Err(err) => {
                error!("Bad yara rule {err:?}");
                return Err(FileError::Filelisting);
            }
        };
    }

    let path_filter = user_regex(options.path_regex.as_ref().unwrap_or(&String::new()))?;
    let file_filter = user_regex(options.filename_regex.as_ref().unwrap_or(&String::new()))?;

    let walk_options = WalkOptions {
        path_filter,
        file_filter,
        yara_rule: rule,
        plat: get_platform_enum(),
    };

    walking(
        &mut walk,
        &mut accessor,
        &source,
        options,
        manager,
        &walk_options,
    )
}

struct WalkOptions {
    path_filter: Regex,
    file_filter: Regex,
    yara_rule: String,
    plat: PlatformType,
}

/// Iterate through the filesystem
fn walking(
    walk: &mut WalkAccessor,
    accessor: &mut Accessor,
    source: &SourceHandle,
    options: &FileOptions,
    manager: &mut OutputManager,
    walk_options: &WalkOptions,
) -> Result<(), FileError> {
    let mut filelist_vec = Vec::new();

    while let Some(value) = walk.next(accessor) {
        let entry = match value {
            Ok(result) => result,
            Err(err) => {
                warn!("Could not walk: {err:?}");
                continue;
            }
        };

        // If Regex does not match then skip file info
        if options.path_regex.is_some()
            && !regex_check(&walk_options.path_filter, &entry.entry.meta.full_path)
        {
            continue;
        }
        if options.filename_regex.is_some()
            && !regex_check(&walk_options.file_filter, &entry.entry.meta.filename)
        {
            continue;
        }

        let mut scan: Vec<String> = Vec::new();
        #[cfg(feature = "yarax")]
        if !walk_options.yara_rule.is_empty() && entry.entry.meta.kind == EntryKind::File {
            let max_size = 100 * 1024 * 1024;
            if entry.entry.meta.size > max_size {
                info!(
                    "Skipping file {}. File size is {} vs 100MB max scans size",
                    entry.entry.meta.display_path, entry.entry.meta.size
                );
                continue;
            }

            let Some(handle) = entry.entry.handle.as_file() else {
                continue;
            };

            let bytes = match accessor.source_read_file_handle(source, handle) {
                Ok(result) => result,
                Err(err) => {
                    error!("Could not read file {}: {err:?}", handle.display_path());
                    continue;
                }
            };

            let scan_result = scan_bytes(&bytes, &walk_options.yara_rule);
            scan = match scan_result {
                Ok(result) => result,
                Err(err) => {
                    warn!("Failed to scan with yara: {err:?}");
                    continue;
                }
            };

            if scan.is_empty() {
                continue;
            }
        }

        let mut file = file_metadata(entry, options, &walk_options.plat, source, accessor);
        file.yara_hits = scan;

        filelist_vec.push(file);
        let max_list = 1000;

        if filelist_vec.len() >= max_list {
            file_output(filelist_vec, manager, options);
            filelist_vec = Vec::new();
        }
    }

    if !filelist_vec.is_empty() {
        file_output(filelist_vec, manager, options);
    }

    Ok(())
}

fn file_metadata(
    entry: WalkEntry,
    options: &FileOptions,
    plat: &PlatformType,
    source: &SourceHandle,
    accessor: &mut Accessor,
) -> FileInfo {
    let mut file = FileInfo {
        full_path: entry.entry.meta.full_path,
        depth: entry.depth as usize,
        filename: entry.entry.meta.filename,
        extension: entry.entry.meta.extension,
        size: entry.entry.meta.size,
        directory: entry.entry.meta.directory,
        display_path: entry.entry.meta.display_path,
        kind: entry.entry.meta.kind,
        ..Default::default()
    };

    if file.kind == EntryKind::File
        && let Some(handle) = entry.entry.handle.as_file()
        && let Ok(stat) = accessor.source_stat_handle(source, handle)
    {
        for entry in stat.times {
            match entry {
                Timestamp::Created(value) => file.created = Some(value),
                Timestamp::Modified(value) => file.modified = value,
                Timestamp::Accessed(value) => file.accessed = Some(value),
                Timestamp::Changed(value) => file.changed = Some(value),
                Timestamp::FilenameCreated(value) => file.filename_created = Some(value),
                Timestamp::FilenameModified(value) => file.filename_modified = Some(value),
                Timestamp::FilenameAccessed(value) => file.filename_accessed = Some(value),
                Timestamp::FilenameChanged(value) => file.filename_changed = Some(value),
            }
        }
    } else if file.kind == EntryKind::Directory
        && let Some(handle) = entry.entry.handle.as_directory()
        && let Ok(stat) = accessor.source_stat_dir_handle(source, handle)
    {
        for entry in stat.times {
            match entry {
                Timestamp::Created(value) => file.created = Some(value),
                Timestamp::Modified(value) => file.modified = value,
                Timestamp::Accessed(value) => file.accessed = Some(value),
                Timestamp::Changed(value) => file.changed = Some(value),
                Timestamp::FilenameCreated(value) => file.filename_created = Some(value),
                Timestamp::FilenameModified(value) => file.filename_modified = Some(value),
                Timestamp::FilenameAccessed(value) => file.filename_accessed = Some(value),
                Timestamp::FilenameChanged(value) => file.filename_changed = Some(value),
            }
        }
    }

    let max_size = 100 * 1024 * 1024;
    // Get executable metadata if enabled
    if file.kind == EntryKind::File
        && options.metadata.is_some_and(|b| b)
        && let Some(handle) = entry.entry.handle.as_file()
        && entry.entry.meta.size < max_size
        && let Ok(mut reader) = accessor.source_open_reader_handle(source, handle)
    {
        file.binary_info = executable_metadata(&mut reader, plat).unwrap_or_default();
    }

    let hashes = Hashes {
        md5: options.md5.unwrap_or_default(),
        sha1: options.sha1.unwrap_or_default(),
        sha256: options.sha256.unwrap_or_default(),
    };

    if (hashes.md5 || hashes.sha1 || hashes.sha256)
        && let Some(handle) = entry.entry.handle.as_file()
        && let Ok(mut reader) = accessor.source_open_reader_handle(source, handle)
    {
        let (md5, sha1, sha256) = hash_reader(&hashes, &mut reader);
        file.md5 = md5;
        file.sha1 = sha1;
        file.sha256 = sha256;
    }

    file
}

/// Get executable metadata
fn executable_metadata(
    reader: &mut AccessorReader,
    plat: &PlatformType,
) -> Result<Value, FileError> {
    let binary_info = match plat {
        PlatformType::Linux => {
            let binary_result = match parse_elf_reader(reader) {
                Ok(result) => result,
                Err(err) => {
                    if !err.to_string().contains("Magic Bytes") {
                        error!(
                            "Could not parse ELF file {} error: {err:?}",
                            reader.location.display_path()
                        );
                    }
                    return Err(FileError::ParseFile);
                }
            };
            serde_json::to_value(&binary_result).unwrap_or_default()
        }
        PlatformType::Macos => {
            let binary_result = match parse_macho_reader(reader) {
                Ok(results) => results,
                Err(err) => {
                    if err != MachoError::Buffer && err != MachoError::Magic {
                        error!(
                            "Failed to parse executable binary {}, error: {err:?}",
                            reader.location.display_path()
                        );
                    }
                    return Err(FileError::ParseFile);
                }
            };
            serde_json::to_value(&binary_result).unwrap_or_default()
        }
        PlatformType::Windows => {
            let binary_result = match parse_pe_reader(reader) {
                Ok(result) => result,
                Err(err) => {
                    if err != pelite::Error::Invalid && err != pelite::Error::BadMagic {
                        warn!(
                            "Could not parse PE file {}: {err:?}",
                            reader.location.display_path()
                        );
                    }
                    return Err(FileError::ParseFile);
                }
            };
            serde_json::to_value(&binary_result).unwrap_or_default()
        }
        PlatformType::Unknown => Value::Null,
    };

    Ok(binary_info)
}

/// Create Regex based on provided input
fn user_regex(input: &str) -> Result<Regex, FileError> {
    let reg_result = create_regex(input);
    match reg_result {
        Ok(result) => Ok(result),
        Err(err) => {
            error!("Bad regex: {input}, error: {err:?}");
            Err(FileError::Regex)
        }
    }
}

/// Send filelisting to output based on `Output` parameter
fn file_output(entries: Vec<FileInfo>, manager: &mut OutputManager, options: &FileOptions) {
    let mut records = match serialize_records_to_stream(entries) {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to serialize filelisting: {err:?}");
            return;
        }
    };

    let artifact_name = "files";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("Failed to output filelisting: {err:?}");
    }
}

#[cfg(test)]
mod tests {
    use crate::accessor::access::Accessor;
    use crate::accessor::walk::WalkAccessor;
    use crate::artifacts::os::files::filelisting::{
        executable_metadata, file_metadata, file_output, get_filelist, user_regex,
    };
    use crate::artifacts::os::systeminfo::info::PlatformType;
    use crate::output::manager::OutputManager;
    use crate::structs::artifacts::os::files::FileOptions;
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use common::files::FileInfo;
    use std::path::PathBuf;

    fn output_options(name: &str, directory: &str, compress: bool) -> OutputManager {
        let config = OutputConfig {
            name: name.to_string(),
            endpoint_id: String::from("abcd"),
            directory: PathBuf::from(directory),
            destination: OutputDestination::Local,
            format: OutputFormat::Jsonl,
            compress,
            ..Default::default()
        };

        OutputManager::new(config).unwrap()
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_get_filelist() {
        let mut manager = output_options("files_temp", "./tmp", false);

        let options = FileOptions {
            start_path: String::from("/System/Volumes/Data/Users"),
            depth: Some(4),
            metadata: Some(true),
            md5: Some(true),
            path_regex: Some(String::from(r".*/Downloads")),
            source: String::from("host:"),
            ..Default::default()
        };

        let results = get_filelist(&options, &mut manager).unwrap();
        assert_eq!(results, ());
    }

    #[test]
    fn test_file_output() {
        let mut manager = output_options("files_temp", "./tmp", false);

        let info = FileInfo {
            full_path: String::from("/root"),
            directory: String::from("/root"),
            depth: 1,
            ..Default::default()
        };
        let options = FileOptions {
            start_path: String::new(),
            depth: Some(1),
            source: String::from("host:"),
            ..Default::default()
        };
        file_output(vec![info], &mut manager, &options);
    }

    #[test]
    fn test_user_regex() {
        let test = r".*/Downloads";
        let reg = user_regex(test).unwrap();
        assert_eq!(reg.as_str(), ".*/Downloads");
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_get_filelist() {
        let mut manager = output_options("files_temp", "./tmp", false);

        let options = FileOptions {
            start_path: String::from("C:\\Windows"),
            depth: Some(1),
            metadata: Some(true),
            md5: Some(true),
            source: String::from("host:"),
            ..Default::default()
        };

        let results = get_filelist(&options, &mut manager).unwrap();
        assert_eq!(results, ());
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn test_get_filelist() {
        let mut manager = output_options("files_temp", "./tmp", false);

        let options = FileOptions {
            start_path: String::from("/bin"),
            depth: Some(1),
            source: String::from("host:"),
            ..Default::default()
        };
        let results = get_filelist(&options, &mut manager).unwrap();
        assert_eq!(results, ());
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_file_metadata() {
        let mut accessor = Accessor::with_defaults();
        let source = accessor.open_source("host:").unwrap();
        let mut walk = WalkAccessor::new(&source, "C:\\Windows\\").unwrap();
        walk = walk.max_depth(1);

        let metadata = true;
        let options = FileOptions {
            depth: Some(1),
            metadata: Some(metadata),
            source: String::from("host:"),
            ..Default::default()
        };

        let mut results = Vec::new();
        while let Some(entries) = walk.next(&accessor) {
            let entry_data = entries.unwrap();
            let data = file_metadata(
                entry_data,
                &options,
                &PlatformType::Linux,
                &source,
                &mut accessor,
            );
            results.push(data);
        }

        assert!(results.len() > 3);
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn test_file_metadata() {
        let mut accessor = Accessor::with_defaults();
        let source = accessor.open_source("host:").unwrap();
        let mut walk = WalkAccessor::new(&source, "/bin").unwrap();
        walk = walk.max_depth(1);

        let metadata = true;
        let mut results = Vec::new();
        let options = FileOptions {
            depth: Some(1),
            metadata: Some(metadata),
            source: String::from("host:"),
            ..Default::default()
        };

        while let Some(entries) = walk.next(&accessor) {
            let entry_data = entries.unwrap();
            let data = file_metadata(
                entry_data,
                &options,
                &PlatformType::Linux,
                &source,
                &mut accessor,
            );
            results.push(data);
        }
        assert!(results.len() > 3);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_binary_metadata() {
        let mut reader = Accessor::with_defaults().open_reader("/bin/ls").unwrap();
        let results = executable_metadata(&mut reader, &PlatformType::Macos).unwrap();

        assert_eq!(results.as_array().unwrap().len(), 2);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_binary_metadata() {
        let mut reader = Accessor::with_defaults().open_reader("/bin/ls").unwrap();
        let results = executable_metadata(&mut reader, &PlatformType::Linux).unwrap();

        assert!(!results.is_null());
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_binary_metadata() {
        let mut reader = Accessor::with_defaults()
            .open_reader("C:\\Windows\\explorer.exe")
            .unwrap();
        let results = executable_metadata(&mut reader, &PlatformType::Windows).unwrap();

        assert!(!results.is_null());
    }
}
