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
use crate::artifacts::os::linux::executable::parser::parse_elf_file;
use crate::artifacts::os::macos::macho::error::MachoError;
use crate::artifacts::os::macos::macho::parser::parse_macho;
use crate::artifacts::os::systeminfo::info::{PlatformType, get_platform_enum};
use crate::artifacts::os::windows::pe::parser::parse_pe_file;
use crate::filesystem::files::hash_file;
use crate::filesystem::metadata::get_metadata;
use crate::filesystem::metadata::get_timestamps;
use crate::output::manager::OutputManager;
use crate::output::record::serialize_records_to_stream;
use crate::structs::artifacts::os::files::FileOptions;
use crate::structs::toml::OutputFormat;
use crate::utils::regex_options::{create_regex, regex_check};
use common::files::FileInfo;
use common::files::Hashes;
use log::{error, info, warn};
use regex::Regex;
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader, Error as ioError};
use walkdir::{DirEntry, WalkDir};

#[cfg(feature = "yarax")]
use crate::utils::yara::{extract_rule, scan_file};

/// Get file listing
pub(crate) fn get_filelist(
    options: &FileOptions,
    manager: &mut OutputManager,
) -> Result<(), FileError> {
    let start_walk = WalkDir::new(&options.start_path).same_file_system(false);
    let depth = options.depth.unwrap_or(1);
    let begin_walk = start_walk.max_depth(depth as usize);
    let mut filelist_vec: Vec<FileInfo> = Vec::new();

    let path_filter = user_regex(options.path_regex.as_ref().unwrap_or(&String::new()))?;
    let file_filter = user_regex(options.filename_regex.as_ref().unwrap_or(&String::new()))?;
    let mut firmlink_paths: Vec<String> = Vec::new();

    let platform = get_platform_enum();
    if platform == PlatformType::Macos {
        let firmlink_paths_data = read_firmlinks();
        match firmlink_paths_data {
            Ok(mut firmlinks) => firmlink_paths.append(&mut firmlinks),
            Err(err) => warn!("[files] Failed to read firmlinks file on macOS: {err:?}"),
        }
    }

    let mut exclude_directories = options
        .exclude_directories
        .as_ref()
        .unwrap_or(&Vec::new())
        .clone();
    // On macOS we always skip firmlinks
    exclude_directories.append(&mut firmlink_paths);

    let mut rule = String::new();
    #[cfg(feature = "yarax")]
    if options.yara.as_ref().is_some_and(|s| !s.is_empty()) {
        // Unwrap is safe since we validate above
        rule = match extract_rule(options.yara.as_ref().unwrap()) {
            Ok(result) => result,
            Err(err) => {
                error!("[files] Bad yara rule {err:?}");
                return Err(FileError::Filelisting);
            }
        };
    }

    for entries in begin_walk
        .into_iter()
        .filter_entry(|f| !skip_directory(f, &exclude_directories))
    {
        let entry = match entries {
            Ok(result) => result,
            Err(err) => {
                warn!("[files] Failed to get file info: {err:?}");
                continue;
            }
        };

        // If Regex does not match then skip file info
        if options.path_regex.is_some()
            && !regex_check(&path_filter, &entry.path().display().to_string())
        {
            continue;
        }
        if options.filename_regex.is_some()
            && !regex_check(&file_filter, &entry.file_name().display().to_string())
        {
            continue;
        }

        let mut scan = Vec::new();
        #[cfg(feature = "yarax")]
        if !rule.is_empty() {
            if !entry.file_type().is_file() {
                continue;
            }
            let scan_result = scan_file(&entry.path().display().to_string(), &rule);
            scan = match scan_result {
                Ok(result) => result,
                Err(err) => {
                    warn!("[files] Failed to scan with yara: {err:?}");
                    continue;
                }
            };

            if scan.is_empty() {
                continue;
            }
        }

        let file_entry_result = file_metadata(&entry, options, &platform);
        let mut file_entry = match file_entry_result {
            Ok(result) => result,
            Err(err) => {
                warn!(
                    "[files] Failed to get file {:?} entry data: {err:?}",
                    entry.path()
                );
                continue;
            }
        };
        file_entry.yara_hits = scan;

        filelist_vec.push(file_entry);
        // If we are not parsing binary data and not timelining our limit is 10k, otherwise set limit to 1k
        let max_list = if !options.metadata.is_some_and(|b| b)
            && manager.config.format != OutputFormat::Timeline
        {
            10000
        } else {
            1000
        };
        if filelist_vec.len() >= max_list {
            file_output(filelist_vec, manager, options);
            filelist_vec = Vec::new();
        }
    }
    file_output(filelist_vec, manager, options);
    Ok(())
}

/// Get info on file (or directory)
fn file_metadata(
    entry: &DirEntry,
    options: &FileOptions,
    plat: &PlatformType,
) -> Result<FileInfo, ioError> {
    let mut file_entry = FileInfo {
        full_path: entry.path().display().to_string(),
        depth: entry.depth(),
        ..Default::default()
    };

    file_entry.extension = entry
        .path()
        .extension()
        .unwrap_or_default()
        .display()
        .to_string();
    let metadata = get_metadata(&file_entry.full_path)?;

    let timestamps = get_timestamps(&file_entry.full_path)?;
    file_entry.is_file = metadata.is_file();
    file_entry.is_directory = metadata.is_dir();
    file_entry.is_symlink = metadata.is_symlink();
    file_entry.created = timestamps.created;
    file_entry.modified = timestamps.modified;
    file_entry.accessed = timestamps.accessed;
    file_entry.changed = timestamps.changed;

    file_entry.size = if metadata.is_file() {
        metadata.len()
    } else {
        0
    };

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        use std::os::unix::prelude::MetadataExt;

        file_entry.inode = metadata.ino();
        file_entry.mode = metadata.mode();
        file_entry.uid = metadata.uid();
        file_entry.gid = metadata.gid();
    }

    // Get executable metadata if enabled
    if options.metadata.is_some_and(|b| b) && file_entry.is_file {
        file_entry.binary_info =
            executable_metadata(&entry.path().display().to_string(), plat).unwrap_or_default();
    }

    let hashes = Hashes {
        md5: options.md5.unwrap_or_default(),
        sha1: options.sha1.unwrap_or_default(),
        sha256: options.sha256.unwrap_or_default(),
    };

    if (hashes.md5 || hashes.sha1 || hashes.sha256) && file_entry.is_file && !file_entry.is_symlink
    {
        let (md5, sha1, sha256) = hash_file(&hashes, &file_entry.full_path);
        file_entry.md5 = md5;
        file_entry.sha1 = sha1;
        file_entry.sha256 = sha256;
    }

    let base_path = entry.path();
    if let Some(parent) = base_path.parent() {
        file_entry.directory = parent.display().to_string();
    } else {
        info!(
            "[files] Did not get parent directory for filename at: {:?}",
            entry.path()
        );
    }

    if let Some(filename) = entry.file_name().to_str() {
        file_entry.filename = filename.to_string();
    } else {
        warn!("[files] Failed to get filename for: {:?}", entry.path());
    }
    Ok(file_entry)
}

/// Skip directory if in our exclusion array
fn skip_directory(entry: &DirEntry, directories: &[String]) -> bool {
    if directories.is_empty() {
        return false;
    }
    for exclude_dir in directories {
        let skip = entry.path().to_str().is_some_and(|s| s == exclude_dir);
        if skip {
            return skip;
        }
    }

    false
}

/// Read the firmlinks file on disk (holds all default firmlink paths)
fn read_firmlinks() -> Result<Vec<String>, std::io::Error> {
    let default_firmlinks = "/usr/share/firmlinks";
    let file = File::open(default_firmlinks)?;
    let reader = BufReader::new(file);
    let mut firmlink_paths: Vec<String> = Vec::new();

    for entry in reader.lines() {
        let line_entry = entry?;
        let firmlink: Vec<&str> = line_entry.split_whitespace().collect();
        firmlink_paths.push(firmlink[0].to_string());
    }
    Ok(firmlink_paths)
}

/// Get executable metadata
fn executable_metadata(path: &str, plat: &PlatformType) -> Result<Value, FileError> {
    let binary_info = match plat {
        PlatformType::Linux => {
            let binary_result = match parse_elf_file(path) {
                Ok(result) => result,
                Err(err) => {
                    if !err.to_string().contains("Magic Bytes") {
                        error!("[files] Could not parse ELF file {path} error: {err:?}");
                    }
                    return Err(FileError::ParseFile);
                }
            };
            serde_json::to_value(&binary_result).unwrap_or_default()
        }
        PlatformType::Macos => {
            let binary_result = match parse_macho(path) {
                Ok(results) => results,
                Err(err) => {
                    if err != MachoError::Buffer && err != MachoError::Magic {
                        error!("[files] Failed to parse executable binary {path}, error: {err:?}");
                    }
                    return Err(FileError::ParseFile);
                }
            };
            serde_json::to_value(&binary_result).unwrap_or_default()
        }
        PlatformType::Windows => {
            let binary_result = match parse_pe_file(path) {
                Ok(result) => result,
                Err(err) => {
                    if err != pelite::Error::Invalid && err != pelite::Error::BadMagic {
                        warn!("[files] Could not parse PE file {path}: {err:?}");
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
            error!("[files] Bad regex: {input}, error: {err:?}");
            Err(FileError::Regex)
        }
    }
}

/// Send filelisting to output based on `Output` parameter
fn file_output(entries: Vec<FileInfo>, manager: &mut OutputManager, options: &FileOptions) {
    let mut records = match serialize_records_to_stream(entries) {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Failed to serialize filelisting: {err:?}");
            return;
        }
    };

    let artifact_name = "files";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("[forensics] Failed to output filelisting: {err:?}");
    }
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::files::filelisting::{
        executable_metadata, file_metadata, file_output, get_filelist, user_regex,
    };
    use crate::artifacts::os::systeminfo::info::PlatformType;
    use crate::output::manager::OutputManager;
    use crate::structs::artifacts::os::files::FileOptions;
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use common::files::FileInfo;
    use std::path::PathBuf;
    use walkdir::WalkDir;

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
            sha1: Some(false),
            sha256: Some(false),
            path_regex: Some(String::from(r".*/Downloads")),
            filename_regex: None,
            yara: None,
            exclude_directories: None,
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
            metadata: Some(false),
            md5: Some(false),
            sha1: Some(false),
            sha256: Some(false),
            path_regex: None,
            filename_regex: None,
            yara: None,
            exclude_directories: None,
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
            sha1: Some(false),
            sha256: Some(false),
            path_regex: None,
            filename_regex: None,
            yara: None,
            exclude_directories: None,
        };

        let results = get_filelist(&options, &mut manager).unwrap();
        assert_eq!(results, ());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_get_filelist() {
        let mut manager = output_options("files_temp", "./tmp", false);

        let options = FileOptions {
            start_path: String::from("/bin"),
            depth: Some(1),
            metadata: Some(false),
            md5: Some(false),
            sha1: Some(false),
            sha256: Some(false),
            path_regex: None,
            filename_regex: None,
            yara: None,
            exclude_directories: None,
        };
        let results = get_filelist(&options, &mut manager).unwrap();
        assert_eq!(results, ());
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_file_metadata() {
        let start_path = WalkDir::new("C:\\Windows\\System32").max_depth(1);
        let metadata = true;
        let options = FileOptions {
            start_path: String::new(),
            depth: Some(1),
            metadata: Some(metadata),
            md5: Some(false),
            sha1: Some(false),
            sha256: Some(false),
            path_regex: None,
            filename_regex: None,
            yara: None,
            exclude_directories: None,
        };
        let mut results: Vec<FileInfo> = Vec::new();
        for entries in start_path {
            let entry_data = entries.unwrap();
            let data = file_metadata(&entry_data, &options, &PlatformType::Windows).unwrap();
            results.push(data);
        }
        assert!(results.len() > 3);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_file_metadata() {
        let start_path = WalkDir::new("/bin").max_depth(1);
        let metadata = true;
        let mut results: Vec<FileInfo> = Vec::new();
        let options = FileOptions {
            start_path: String::new(),
            depth: Some(1),
            metadata: Some(metadata),
            md5: Some(false),
            sha1: Some(false),
            sha256: Some(false),
            path_regex: None,
            filename_regex: None,
            yara: None,
            exclude_directories: None,
        };
        for entries in start_path {
            let entry_data = entries.unwrap();
            let data = file_metadata(&entry_data, &options, &PlatformType::Linux).unwrap();
            results.push(data);
        }
        assert!(results.len() > 3);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_read_firmlinks() {
        use crate::artifacts::os::files::filelisting::read_firmlinks;
        let results = read_firmlinks().unwrap();
        assert!(results.len() > 3);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_skip_directory() {
        use crate::artifacts::os::files::filelisting::{read_firmlinks, skip_directory};
        let skip_path = WalkDir::new("/").max_depth(1);
        let results = read_firmlinks().unwrap();
        assert!(results.len() > 3);

        for entries in skip_path {
            let entry_data = entries.unwrap();
            let is_firmlink = skip_directory(&entry_data, &results);
            if entry_data.file_name() == "Users" {
                assert!(is_firmlink);
            }

            if entry_data.file_name() == "Applications" || entry_data.file_name() == "Library" {
                assert!(is_firmlink);
            }

            if entry_data.file_name() == "bin" {
                assert!(!is_firmlink);
            }
        }

        let start_path = WalkDir::new("/sbin").max_depth(1);
        for entries in start_path {
            let entry_data = entries.unwrap();
            let is_firmlink = skip_directory(&entry_data, &results);
            assert_eq!(is_firmlink, false);
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_file_metadata() {
        let start_path = WalkDir::new("/sbin").max_depth(1);
        let metadata = true;
        let options = FileOptions {
            start_path: String::new(),
            depth: Some(1),
            metadata: Some(metadata),
            md5: Some(false),
            sha1: Some(false),
            sha256: Some(false),
            path_regex: None,
            filename_regex: None,
            yara: None,
            exclude_directories: None,
        };
        let mut results: Vec<FileInfo> = Vec::new();
        for entries in start_path {
            let entry_data = entries.unwrap();
            let data = file_metadata(&entry_data, &options, &PlatformType::Macos).unwrap();
            results.push(data);
        }
        assert!(results.len() > 3);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_binary_metadata() {
        let test_path = "/bin/ls";
        let results = executable_metadata(test_path, &PlatformType::Macos).unwrap();

        assert_eq!(results.as_array().unwrap().len(), 2);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_binary_metadata() {
        let test_path = "/bin/ls";
        let results = executable_metadata(test_path, &PlatformType::Linux).unwrap();

        assert!(!results.is_null());
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_binary_metadata() {
        let test_path = "C:\\Windows\\explorer.exe";
        let results = executable_metadata(test_path, &PlatformType::Windows).unwrap();

        assert!(!results.is_null());
    }
}
