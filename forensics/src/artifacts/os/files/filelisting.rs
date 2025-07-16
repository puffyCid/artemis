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
use crate::artifacts::os::systeminfo::info::{PlatformType, get_platform, get_platform_enum};
use crate::artifacts::os::windows::pe::parser::parse_pe_file;
use crate::artifacts::output::output_artifact;
use crate::filesystem::files::{file_extension, hash_file};
use crate::filesystem::metadata::get_metadata;
use crate::filesystem::metadata::get_timestamps;
use crate::structs::toml::Output;
use crate::utils::regex_options::{create_regex, regex_check};
use crate::utils::time::time_now;
use crate::utils::yara::scan_file;
use common::files::FileInfo;
use common::files::Hashes;
use log::{error, info, warn};
use regex::Regex;
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader, Error as ioError};
use walkdir::{DirEntry, WalkDir};

pub(crate) struct FileArgs {
    pub(crate) start_directory: String,
    pub(crate) depth: usize,
    pub(crate) metadata: bool,
    pub(crate) yara: String,
    pub(crate) path_filter: String,
}

/// Get file listing
pub(crate) fn get_filelist(
    args: &FileArgs,
    hashes: &Hashes,
    output: &mut Output,
    filter: bool,
) -> Result<(), FileError> {
    let start_time = time_now();

    let start_walk = WalkDir::new(&args.start_directory).same_file_system(false);
    let begin_walk = start_walk.max_depth(args.depth);
    let mut filelist_vec: Vec<FileInfo> = Vec::new();

    let path_filter = user_regex(&args.path_filter)?;
    let mut firmlink_paths: Vec<String> = Vec::new();

    let platform = get_platform_enum();
    if platform == PlatformType::Macos {
        let firmlink_paths_data = read_firmlinks();
        match firmlink_paths_data {
            Ok(mut firmlinks) => firmlink_paths.append(&mut firmlinks),
            Err(err) => warn!("[files] Failed to read firmlinks file on macOS: {err:?}"),
        }
    }

    for entries in begin_walk
        .into_iter()
        .filter_entry(|f| !skip_firmlinks(f, &firmlink_paths))
    {
        let entry = match entries {
            Ok(result) => result,
            Err(err) => {
                warn!("[files] Failed to get file info: {err:?}");
                continue;
            }
        };

        let mut scan = Vec::new();
        if !args.yara.is_empty() {
            if !entry.file_type().is_file() {
                continue;
            }
            let scan_result = scan_file(&entry.path().display().to_string(), &args.yara);
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

        // If Regex does not match then skip file info
        if !regex_check(&path_filter, &entry.path().display().to_string()) {
            continue;
        }

        let file_entry_result = file_metadata(&entry, args.metadata, hashes, &platform);
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
        let max_list = if !args.metadata && !output.timeline {
            10000
        } else {
            1000
        };
        if filelist_vec.len() >= max_list {
            file_output(&filelist_vec, output, start_time, filter);
            filelist_vec = Vec::new();
        }
    }
    file_output(&filelist_vec, output, start_time, filter);
    Ok(())
}

/// Get info on file (or directory)
fn file_metadata(
    entry: &DirEntry,
    get_executable_info: bool,
    hashes: &Hashes,
    plat: &PlatformType,
) -> Result<FileInfo, ioError> {
    let mut file_entry = FileInfo {
        full_path: entry.path().display().to_string(),
        directory: String::new(),
        filename: String::new(),
        extension: String::new(),
        created: String::new(),
        modified: String::new(),
        changed: String::new(),
        accessed: String::new(),
        size: 0,
        inode: 0,
        mode: 0,
        uid: 0,
        gid: 0,
        md5: String::new(),
        sha1: String::new(),
        sha256: String::new(),
        is_file: false,
        is_directory: false,
        is_symlink: false,
        depth: entry.depth(),
        binary_info: Value::Null,
        yara_hits: Vec::new(),
    };
    file_entry.extension = file_extension(&file_entry.full_path);
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
    if get_executable_info && file_entry.is_file {
        file_entry.binary_info =
            executable_metadata(&entry.path().display().to_string(), plat).unwrap_or_default();
    }

    if hashes.md5 || hashes.sha1 || hashes.sha256 {
        let (md5, sha1, sha256) = hash_file(hashes, &file_entry.full_path);
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

/// Skip default firmlinks on macOS
fn skip_firmlinks(entry: &DirEntry, firmlink_paths: &[String]) -> bool {
    if firmlink_paths.is_empty() {
        return false;
    }
    let platform = get_platform();
    if platform == "Darwin" {
        let mut is_firmlink = true;
        for firmlink in firmlink_paths {
            is_firmlink = entry
                .path()
                .to_str()
                .is_some_and(|s| s.starts_with(firmlink));
            if is_firmlink {
                return is_firmlink;
            }
        }
        return is_firmlink;
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
fn file_output(filelist: &[FileInfo], output: &mut Output, start_time: u64, filter: bool) {
    let serde_data_result = serde_json::to_value(filelist);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[files] Failed to serialize filelisting: {err:?}");
            return;
        }
    };

    let status = output_artifact(&mut serde_data, "files", output, start_time, filter);
    if status.is_err() {
        error!(
            "[forensics] Could not output data: {:?}",
            status.unwrap_err()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::file_output;
    use crate::artifacts::os::files::filelisting::FileArgs;
    use crate::artifacts::os::files::filelisting::executable_metadata;
    use crate::artifacts::os::files::filelisting::file_metadata;
    use crate::artifacts::os::files::filelisting::get_filelist;
    use crate::artifacts::os::systeminfo::info::PlatformType;
    use crate::{
        artifacts::os::files::filelisting::{Hashes, user_regex},
        structs::toml::Output,
    };
    use common::files::FileInfo;
    use serde_json::Value;
    use walkdir::WalkDir;

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
            compress,
            url: Some(String::new()),
            timeline: false,
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_get_filelist() {
        let start_location = "/System/Volumes/Data/Users";
        let depth = 4;
        let metadata = true;
        let hashes = Hashes {
            md5: true,
            sha1: false,
            sha256: false,
        };
        let path_filter = r".*/Downloads";
        let mut output = output_options("files_temp", "local", "./tmp", false);

        let args = FileArgs {
            start_directory: start_location.to_string(),
            depth,
            metadata,
            yara: String::new(),
            path_filter: path_filter.to_string(),
        };

        let results = get_filelist(&args, &hashes, &mut output, false).unwrap();
        assert_eq!(results, ());
    }

    #[test]
    fn test_file_output() {
        let mut output = output_options("files_temp", "local", "./tmp", false);
        let info = FileInfo {
            full_path: String::from("/root"),
            directory: String::from("/root"),
            filename: String::new(),
            extension: String::new(),
            created: String::new(),
            modified: String::new(),
            changed: String::new(),
            accessed: String::new(),
            size: 0,
            inode: 0,
            mode: 0,
            uid: 0,
            gid: 0,
            md5: String::new(),
            sha1: String::new(),
            sha256: String::new(),
            is_file: false,
            is_directory: true,
            is_symlink: false,
            depth: 1,
            binary_info: Value::Null,
            yara_hits: Vec::new(),
        };
        file_output(&vec![info], &mut output, 0, false);
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
        let start_location = "C:\\Windows";
        let depth = 1;
        let metadata = true;
        let hashes = Hashes {
            md5: true,
            sha1: false,
            sha256: false,
        };
        let path_filter = "";
        let mut output = output_options("files_temp", "local", "./tmp", false);

        let args = FileArgs {
            start_directory: start_location.to_string(),
            depth,
            metadata,
            yara: String::new(),
            path_filter: path_filter.to_string(),
        };

        let results = get_filelist(&args, &hashes, &mut output, false).unwrap();
        assert_eq!(results, ());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_get_filelist() {
        let start_location = "/bin";
        let depth = 1;
        let metadata = false;
        let hashes = Hashes {
            md5: true,
            sha1: false,
            sha256: false,
        };
        let path_filter = "";
        let mut output = output_options("files_temp", "local", "./tmp", false);

        let args = FileArgs {
            start_directory: start_location.to_string(),
            depth,
            metadata,
            yara: String::new(),
            path_filter: path_filter.to_string(),
        };

        let results = get_filelist(&args, &hashes, &mut output, false).unwrap();
        assert_eq!(results, ());
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_file_metadata() {
        let start_path = WalkDir::new("C:\\Windows\\System32").max_depth(1);
        let metadata = true;
        let hashes = Hashes {
            md5: false,
            sha1: false,
            sha256: false,
        };
        let mut results: Vec<FileInfo> = Vec::new();
        for entries in start_path {
            let entry_data = entries.unwrap();
            let data =
                file_metadata(&entry_data, metadata, &hashes, &PlatformType::Windows).unwrap();
            results.push(data);
        }
        assert!(results.len() > 3);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_file_metadata() {
        use crate::artifacts::os::systeminfo::info::PlatformType;

        let start_path = WalkDir::new("/bin").max_depth(1);
        let metadata = true;
        let hashes = Hashes {
            md5: false,
            sha1: false,
            sha256: false,
        };
        let mut results: Vec<FileInfo> = Vec::new();
        for entries in start_path {
            let entry_data = entries.unwrap();
            let data = file_metadata(&entry_data, metadata, &hashes, &PlatformType::Linux).unwrap();
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
    fn test_skip_firmlinks() {
        use crate::artifacts::os::files::filelisting::{read_firmlinks, skip_firmlinks};
        let skip_path = WalkDir::new("/Users").max_depth(1);
        let results = read_firmlinks().unwrap();
        assert!(results.len() > 3);

        for entries in skip_path {
            let entry_data = entries.unwrap();
            let is_firmlink = skip_firmlinks(&entry_data, &results);
            assert_eq!(is_firmlink, true);
        }

        let start_path = WalkDir::new("/sbin").max_depth(1);
        for entries in start_path {
            let entry_data = entries.unwrap();
            let is_firmlink = skip_firmlinks(&entry_data, &results);
            assert_eq!(is_firmlink, false);
        }
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_file_metadata() {
        let start_path = WalkDir::new("/sbin").max_depth(1);
        let metadata = true;
        let hashes = Hashes {
            md5: true,
            sha1: false,
            sha256: false,
        };
        let mut results: Vec<FileInfo> = Vec::new();
        for entries in start_path {
            let entry_data = entries.unwrap();
            let data = file_metadata(&entry_data, metadata, &hashes, &PlatformType::Macos).unwrap();
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
