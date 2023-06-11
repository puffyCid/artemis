/**
 * Get a standard filelisting from the system.  
 * Supports both Windows and macOS, in addition can parse executable metadata for each OS based on `FileOptions`  
 * `PE` for Windows  
 * `Macho` for macOS
 *
 * On macOS the filelisting will read the firmlinks file at `/usr/share/firmlinks` and skip firmlink paths
 */
use super::error::FileError;
use crate::artifacts::os::systeminfo::info::SystemInfo;
use crate::filesystem::files::{file_extension, hash_file};
use crate::filesystem::metadata::get_metadata;
use crate::filesystem::{files::Hashes, metadata::get_timestamps};
use crate::utils::artemis_toml::Output;
use crate::utils::regex_options::{create_regex, regex_check};
use crate::utils::time::time_now;
use log::{error, info, warn};
use regex::Regex;
use serde::Serialize;
use std::io::Error as ioError;
use walkdir::{DirEntry, WalkDir};

#[cfg(target_os = "macos")]
use crate::artifacts::os::macos::{artifacts::output_data, macho::parser::MachoInfo};

#[cfg(target_os = "windows")]
use crate::artifacts::os::windows::{
    artifacts::output_data,
    pe::parser::{parse_pe_file, PeInfo},
};

#[cfg(target_os = "linux")]
use crate::artifacts::os::linux::artifacts::output_data;

#[cfg(target_family = "unix")]
use std::os::unix::prelude::MetadataExt;

#[derive(Debug, Serialize)]
pub(crate) struct FileInfo {
    pub(crate) full_path: String,
    pub(crate) directory: String,
    pub(crate) filename: String,
    pub(crate) extension: String,
    pub(crate) created: i64,
    pub(crate) modified: i64,
    pub(crate) changed: i64,
    pub(crate) accessed: i64,
    pub(crate) size: u64,
    pub(crate) inode: u64,
    pub(crate) mode: u32,
    pub(crate) uid: u32,
    pub(crate) gid: u32,
    pub(crate) md5: String,
    pub(crate) sha1: String,
    pub(crate) sha256: String,
    pub(crate) is_file: bool,
    pub(crate) is_directory: bool,
    pub(crate) is_symlink: bool,
    pub(crate) depth: usize,
    #[cfg(target_os = "macos")]
    pub(crate) binary_info: Vec<MachoInfo>,
    #[cfg(target_os = "windows")]
    pub(crate) binary_info: Vec<PeInfo>,
    #[cfg(target_os = "linux")]
    pub(crate) binary_info: Vec<String>,
}

impl FileInfo {
    /// Get file listing
    pub(crate) fn get_filelist(
        start_directory: &str,
        depth: usize,
        metadata: bool,
        hashes: &Hashes,
        path_filter: &str,
        output: &mut Output,
        filter: &bool,
    ) -> Result<(), FileError> {
        let start_time = time_now();

        let start_walk = WalkDir::new(start_directory).same_file_system(true);
        let begin_walk = start_walk.max_depth(depth);
        let mut filelist_vec: Vec<FileInfo> = Vec::new();

        let path_filter = FileInfo::user_regex(path_filter)?;
        let mut firmlink_paths: Vec<String> = Vec::new();

        #[cfg(target_os = "macos")]
        {
            let firmlink_paths_data = FileInfo::read_firmlinks();
            match firmlink_paths_data {
                Ok(mut firmlinks) => firmlink_paths.append(&mut firmlinks),
                Err(err) => warn!("[files] Failed to read firmlinks file on macOS: {err:?}"),
            }
        }

        for entries in begin_walk
            .into_iter()
            .filter_entry(|f| !FileInfo::skip_firmlinks(f, &firmlink_paths))
        {
            let entry = match entries {
                Ok(result) => result,
                Err(err) => {
                    warn!("[files] Failed to get file info: {err:?}");
                    continue;
                }
            };

            // If Regex does not match then skip file info
            if !regex_check(&path_filter, &entry.path().display().to_string()) {
                continue;
            }

            let file_entry_result = FileInfo::file_metadata(&entry, metadata, hashes);
            let file_entry = match file_entry_result {
                Ok(result) => result,
                Err(err) => {
                    warn!(
                        "[files] Failed to get file {:?} entry data: {err:?}",
                        entry.path()
                    );
                    continue;
                }
            };

            filelist_vec.push(file_entry);
            let max_list = 100000;
            if filelist_vec.len() >= max_list {
                FileInfo::output(&filelist_vec, output, &start_time, filter);
                filelist_vec.clear();
            }
        }
        FileInfo::output(&filelist_vec, output, &start_time, filter);
        Ok(())
    }

    /// Get info on file (or directory)
    fn file_metadata(
        entry: &DirEntry,
        get_executable_info: bool,
        hashes: &Hashes,
    ) -> Result<FileInfo, ioError> {
        let mut file_entry = FileInfo {
            full_path: entry.path().display().to_string(),
            directory: String::new(),
            filename: String::new(),
            extension: String::new(),
            created: 0,
            modified: 0,
            changed: 0,
            accessed: 0,
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
            binary_info: Vec::new(),
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

        #[cfg(target_family = "unix")]
        {
            file_entry.inode = metadata.ino();
            file_entry.mode = metadata.mode();
            file_entry.uid = metadata.uid();
            file_entry.gid = metadata.gid();
        }

        // Skip large files
        if get_executable_info && file_entry.is_file {
            let meta_results = FileInfo::executable_metadata(&entry.path().display().to_string());
            file_entry.binary_info = match meta_results {
                Ok(results) => results,
                Err(_err) => Vec::new(),
            }
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
        let platform = SystemInfo::get_platform();
        if platform == "Darwin" {
            let mut is_firmlink = true;
            for firmlink in firmlink_paths {
                is_firmlink = entry
                    .path()
                    .to_str()
                    .map_or(false, |s| s.starts_with(firmlink));
                if is_firmlink {
                    return is_firmlink;
                }
            }
            return is_firmlink;
        }
        false
    }

    #[cfg(target_os = "macos")]
    /// Read the firmlinks file on disk (holds all default firmlink paths)
    fn read_firmlinks() -> Result<Vec<String>, std::io::Error> {
        use std::{
            fs::File,
            io::{BufRead, BufReader},
        };

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

    #[cfg(target_os = "macos")]
    /// Get executable metadata
    fn executable_metadata(path: &str) -> Result<Vec<MachoInfo>, FileError> {
        use crate::filesystem::files::read_file;
        let buffer_results = read_file(path);
        let buffer = match buffer_results {
            Ok(results) => results,
            Err(_err) => {
                return Err(FileError::ReadFile);
            }
        };

        let binary_results = MachoInfo::parse_macho(&buffer);
        match binary_results {
            Ok(results) => Ok(results),
            Err(err) => {
                error!("[files] Failed to parse executable binary {path}, error: {err:?}");
                Err(FileError::ParseFile)
            }
        }
    }

    #[cfg(target_os = "windows")]
    /// Get executable metadata
    fn executable_metadata(path: &str) -> Result<Vec<PeInfo>, FileError> {
        let info_result = parse_pe_file(path);
        let info = match info_result {
            Ok(result) => result,
            Err(err) => {
                warn!("[files] Could not parse PE file {path}: {err:?}");
                return Err(FileError::ParseFile);
            }
        };
        Ok(vec![info])
    }

    #[cfg(target_os = "linux")]
    /// Get executable metadata
    fn executable_metadata(_path: &str) -> Result<Vec<String>, FileError> {
        return Ok(Vec::new());
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
    fn output(filelist: &[FileInfo], output: &mut Output, start_time: &u64, filter: &bool) {
        let serde_data_result = serde_json::to_value(filelist);
        let serde_data = match serde_data_result {
            Ok(results) => results,
            Err(err) => {
                error!("[files] Failed to serialize filelisting: {err:?}");
                return;
            }
        };

        let output_result = output_data(&serde_data, "files", output, start_time, filter);
        match output_result {
            Ok(_) => {}
            Err(err) => {
                error!("[files] Failed to output filelisting data: {err:?}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FileInfo;
    use crate::{artifacts::os::files::filelisting::Hashes, utils::artemis_toml::Output};
    use walkdir::WalkDir;

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
            compress,
            url: Some(String::new()),
            port: Some(0),
            api_key: Some(String::new()),
            username: Some(String::new()),
            password: Some(String::new()),
            generic_keys: Some(Vec::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
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

        let results = FileInfo::get_filelist(
            &start_location,
            depth,
            metadata,
            &hashes,
            path_filter,
            &mut output,
            &false,
        )
        .unwrap();
        assert_eq!(results, ());
    }

    #[test]
    fn test_output() {
        let mut output = output_options("files_temp", "local", "./tmp", false);
        let info = FileInfo {
            full_path: String::from("/root"),
            directory: String::from("/root"),
            filename: String::new(),
            extension: String::new(),
            created: 0,
            modified: 0,
            changed: 0,
            accessed: 0,
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
            binary_info: Vec::new(),
        };
        FileInfo::output(&vec![info], &mut output, &0, &false)
    }

    #[test]
    fn test_user_regex() {
        let test = r".*/Downloads";
        let reg = FileInfo::user_regex(test).unwrap();
        assert_eq!(reg.as_str(), ".*/Downloads")
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_get_filelist() {
        let start_location = "C:\\Windows";
        let depth = 1;
        let metadata = false;
        let hashes = Hashes {
            md5: true,
            sha1: false,
            sha256: false,
        };
        let path_filter = "";
        let mut output = output_options("files_temp", "local", "./tmp", false);

        let results = FileInfo::get_filelist(
            &start_location,
            depth,
            metadata,
            &hashes,
            path_filter,
            &mut output,
            &false,
        )
        .unwrap();
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

        let results = FileInfo::get_filelist(
            &start_location,
            depth,
            metadata,
            &hashes,
            path_filter,
            &mut output,
            &false,
        )
        .unwrap();
        assert_eq!(results, ());
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_file_metadata() {
        let start_path = WalkDir::new("C:\\Windows\\System32").max_depth(1);
        let metadata = false;
        let hashes = Hashes {
            md5: false,
            sha1: false,
            sha256: false,
        };
        let mut results: Vec<FileInfo> = Vec::new();
        for entries in start_path {
            let entry_data = entries.unwrap();
            let data = FileInfo::file_metadata(&entry_data, metadata, &hashes).unwrap();
            results.push(data);
        }
        assert!(results.len() > 3);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_read_firmlinks() {
        let results = FileInfo::read_firmlinks().unwrap();
        assert!(results.len() > 3);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_skip_firmlinks() {
        let skip_path = WalkDir::new("/Users").max_depth(1);
        let results = FileInfo::read_firmlinks().unwrap();
        assert!(results.len() > 3);

        for entries in skip_path {
            let entry_data = entries.unwrap();
            let is_firmlink = FileInfo::skip_firmlinks(&entry_data, &results);
            assert_eq!(is_firmlink, true);
        }

        let start_path = WalkDir::new("/sbin").max_depth(1);
        for entries in start_path {
            let entry_data = entries.unwrap();
            let is_firmlink = FileInfo::skip_firmlinks(&entry_data, &results);
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
            let data = FileInfo::file_metadata(&entry_data, metadata, &hashes).unwrap();
            results.push(data);
        }
        assert!(results.len() > 3);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_binary_metadata() {
        let test_path = "/bin/ls";
        let results = FileInfo::executable_metadata(test_path).unwrap();

        assert_eq!(results.len(), 2);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_binary_metadata() {
        let test_path = "C:\\Windows\\explorer.exe";
        let results = FileInfo::executable_metadata(test_path).unwrap();

        assert_eq!(results.len(), 1);
    }
}
