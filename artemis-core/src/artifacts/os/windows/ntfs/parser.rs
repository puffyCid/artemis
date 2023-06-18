/**
 * Windows NTFS is the default filesystem on Windows devices  
 * This parser leverages the `ntfs` Rust crate to parse the raw filesystem  
 * In addition, this code extends the `ntfs` crate by adding the following extra capabilities:  
 *   Recover deleted IDNX entries  
 *   Lookup SID associated with file entries  
 *   Decompressing `WindowsOverlayFilter` (WOF) data
 *
 * References:  
 *   `https://www.ntfs.com/index.html`  
 *   `https://flatcap.github.io/linux-ntfs/ntfs/`  
 *   `https://github.com/ColinFinck/ntfs`
 *
 * Other Parsers:  
 *  `https://github.com/Velocidex/velociraptor`
 */
use super::{error::NTFSError, security_ids::SecurityIDs};
use crate::{
    artifacts::os::windows::{
        artifacts::output_data,
        pe::parser::{parse_pe_file, PeInfo},
    },
    filesystem::{
        files::{file_extension, Hashes},
        ntfs::{sector_reader::SectorReader, setup::setup_ntfs_parser},
    },
    structs::artifacts::os::windows::RawFilesOptions,
    utils::{
        artemis_toml::Output,
        regex_options::{create_regex, regex_check},
        strings::strings_contains,
        time::time_now,
    },
};
use log::error;
use ntfs::{Ntfs, NtfsFile};
use regex::Regex;
use serde::Serialize;
use std::{collections::HashMap, fs::File, io::BufReader};

#[derive(Debug, Serialize, Clone)]
pub(crate) struct RawFilelist {
    pub(crate) full_path: String,
    pub(crate) directory: String,
    pub(crate) filename: String,
    pub(crate) extension: String,
    pub(crate) created: i64,
    pub(crate) modified: i64,
    pub(crate) changed: i64,
    pub(crate) accessed: i64,
    pub(crate) filename_created: i64,
    pub(crate) filename_modified: i64,
    pub(crate) filename_changed: i64,
    pub(crate) filename_accessed: i64,
    pub(crate) size: u64,
    pub(crate) compressed_size: u64,
    pub(crate) compression_type: CompressionType,
    pub(crate) inode: u64,
    pub(crate) sequence_number: u16,
    pub(crate) parent_mft_reference: u64,
    pub(crate) owner: u32,
    pub(crate) attributes: Vec<String>,
    pub(crate) md5: String,
    pub(crate) sha1: String,
    pub(crate) sha256: String,
    pub(crate) is_file: bool,
    pub(crate) is_directory: bool,
    pub(crate) is_indx: bool,
    pub(crate) depth: usize,
    pub(crate) usn: u64,
    pub(crate) sid: u32,
    pub(crate) user_sid: String,
    pub(crate) group_sid: String,
    pub(crate) drive: String,
    pub(crate) ads_info: Vec<ADSInfo>,
    pub(crate) pe_info: Vec<PeInfo>,
}

#[derive(Debug, Serialize, Clone)]
pub(crate) struct ADSInfo {
    pub(crate) name: String,
    pub(crate) size: u64,
}

#[derive(Debug, Serialize, Clone)]
pub(crate) enum CompressionType {
    NTFSCompressed,
    WofCompressed,
    None,
}

/// Parameters used for determining what NTFS data to return
struct Params {
    start_path_depth: usize,
    start_path: String,
    depth: u8,
    hash: Hashes,
    metadata: bool,
    recover_indx: bool,
    path_regex: Regex,
    file_regex: Regex,
    start_time: u64,
    filelist: Vec<RawFilelist>,
    directory_tracker: Vec<String>,
    sids: HashMap<u32, SecurityIDs>,
    filter: bool,
}

impl RawFilelist {
    /// Parse the raw NTFS data and get a file listing
    pub(crate) fn raw_filelist(
        rawfile_params: &RawFilesOptions,
        output: &mut Output,
        filter: &bool,
    ) -> Result<(), NTFSError> {
        if rawfile_params.start_path.is_empty()
            || !rawfile_params
                .start_path
                .starts_with(rawfile_params.drive_letter)
        {
            return Err(NTFSError::BadStart);
        }

        let ntfs_parser_result = setup_ntfs_parser(&rawfile_params.drive_letter);
        let mut ntfs_parser = match ntfs_parser_result {
            Ok(result) => result,
            Err(err) => {
                error!("[ntfs] Failed to get NTFS root directory, error: {err:?}");
                return Err(NTFSError::Parser);
            }
        };

        let root_dir_result = ntfs_parser.ntfs.root_directory(&mut ntfs_parser.fs);
        let root_dir = match root_dir_result {
            Ok(result) => result,
            Err(err) => {
                error!("[ntfs] Failed to get NTFS root directory, error: {err:?}");
                return Err(NTFSError::RootDir);
            }
        };

        let start_time = time_now();
        let path_regex =
            RawFilelist::user_regex(rawfile_params.path_regex.as_ref().unwrap_or(&String::new()))?;
        let file_regex = RawFilelist::user_regex(
            rawfile_params
                .filename_regex
                .as_ref()
                .unwrap_or(&String::new()),
        )?;

        let mut start_path = rawfile_params.start_path.clone();
        start_path = start_path
            .strip_prefix("C:")
            .unwrap_or(&start_path)
            .to_string();

        if !start_path.ends_with('\\') {
            start_path = format!("{start_path}\\");
        }

        let mut start_path_depth = 0;
        // Adjust total depth based on starting path depth
        for path in start_path.split('\\') {
            if path.is_empty() {
                continue;
            }
            start_path_depth += 1;
        }
        // restore original start path
        start_path = rawfile_params.start_path.clone();

        // Before parsing the NTFS data, grab Windows SIDs so we can map files to User and Group SIDs
        let sids =
            SecurityIDs::get_security_ids(&root_dir, &mut ntfs_parser.fs, &ntfs_parser.ntfs)?;

        let hash_data = Hashes {
            md5: rawfile_params.md5.unwrap_or(false),
            sha1: rawfile_params.sha1.unwrap_or(false),
            sha256: rawfile_params.sha256.unwrap_or(false),
        };
        let mut params = Params {
            start_path_depth,
            start_path,
            depth: rawfile_params.depth,
            path_regex,
            file_regex,
            recover_indx: rawfile_params.recover_indx,
            start_time,
            filelist: Vec::new(),
            directory_tracker: vec![format!("{}:", rawfile_params.drive_letter)],
            sids,
            hash: hash_data,
            metadata: rawfile_params.metadata.unwrap_or(false),
            filter: *filter,
        };

        let _ = RawFilelist::walk_ntfs(
            root_dir,            // Start at NTFS root
            &mut ntfs_parser.fs, // BufReader to read parts of the NTFS
            &ntfs_parser.ntfs,   // Ntfs object
            &mut params, // Used to determinine what NTFS data to return. Ex: paths, starting location
            output,
        );

        // Output any remaining file metadata
        RawFilelist::output(&params.filelist, output, &start_time, &params.filter);
        Ok(())
    }

    /// Create Regex based on provided input
    fn user_regex(input: &str) -> Result<Regex, NTFSError> {
        let reg_result = create_regex(input);
        match reg_result {
            Ok(result) => Ok(result),
            Err(err) => {
                error!("[ntfs] Bad regex: {input}, error: {err:?}");
                Err(NTFSError::Regex)
            }
        }
    }

    /// Iterate through NTFS files and directories
    fn walk_ntfs(
        root_dir: NtfsFile<'_>,
        fs: &mut BufReader<SectorReader<File>>,
        ntfs: &Ntfs,
        params: &mut Params,
        output: &mut Output,
    ) -> Result<(), NTFSError> {
        let index_result = root_dir.directory_index(fs);
        let index = match index_result {
            Ok(result) => result,
            Err(err) => {
                error!("[ntfs] Failed to get NTFS index directory, error: {err:?}");
                return Err(NTFSError::IndexDir);
            }
        };
        let mut iter = index.entries();
        while let Some(entry) = iter.next(fs) {
            let mut file_info = RawFilelist {
                full_path: String::new(),
                directory: String::new(),
                filename: String::new(),
                extension: String::new(),
                created: 0,
                modified: 0,
                changed: 0,
                accessed: 0,
                filename_created: 0,
                filename_modified: 0,
                filename_changed: 0,
                filename_accessed: 0,
                size: 0,
                compressed_size: 0,
                compression_type: CompressionType::None,
                inode: 0,
                sequence_number: 0,
                parent_mft_reference: 0,
                owner: 0,
                attributes: Vec::new(),
                md5: String::new(),
                sha1: String::new(),
                sha256: String::new(),
                is_file: false,
                is_directory: false,
                is_indx: false,
                depth: params.directory_tracker.len(),
                usn: 0,
                sid: 0,
                user_sid: String::new(),
                group_sid: String::new(),
                drive: params.directory_tracker[0].clone(),
                ads_info: Vec::new(),
                pe_info: Vec::new(),
            };

            let entry_result = entry;
            let entry_index = match entry_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[ntfs] Failed to get NTFS entry index, error: {err:?}");
                    continue;
                }
            };

            let filename_result = entry_index.key();
            // Get $FILENAME attribute data. (4 timestamps and name)
            let filename = match filename_result {
                Some(result) => RawFilelist::filename_info(&result, &mut file_info),
                None => Ok(()),
            };
            match filename {
                Ok(()) => {}
                Err(err) => {
                    if err == NTFSError::Dos {
                        // Skip DOS entries, they point to the same info as non-DOS name entries
                        continue;
                    }
                    return Err(err);
                }
            }
            // Skip root directory loopback
            if file_info.filename == "." {
                continue;
            }

            let ntfs_file_result = entry_index.file_reference().to_file(ntfs, fs);
            let ntfs_file = match ntfs_file_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[ntfs] Failed to get NTFS file, error: {err:?}");
                    continue;
                }
            };

            // Get $STANDARD_INFORMATION attribute data. (4 timestamps, size, sid, owner, usn, attributes)
            let standard_result = ntfs_file.info();
            match standard_result {
                Ok(result) => RawFilelist::standard_info(&result, &mut file_info),
                Err(err) => {
                    error!("[ntfs] Failed to get NTFS standard info, error: {err:?}");
                    continue;
                }
            }

            file_info.directory = params.directory_tracker.join("\\");
            file_info.full_path = format!("{}\\{}", file_info.directory, file_info.filename);
            file_info.is_file = !ntfs_file.is_directory();
            file_info.is_directory = ntfs_file.is_directory();
            file_info.inode = ntfs_file.file_record_number();
            file_info.sequence_number = ntfs_file.sequence_number();

            // Lookup traditional SID information (S-1-5-XXXXX) via the NTFS sid value
            (file_info.user_sid, file_info.group_sid) =
                SecurityIDs::lookup_sids(&file_info.sid, &params.sids);

            let dir_name = file_info.filename.clone();

            if file_info.is_file && file_info.full_path.starts_with(&params.start_path) {
                file_info.extension = file_extension(&file_info.filename);

                // Grab file data for hashing
                let _attribute_result = RawFilelist::file_data(
                    &ntfs_file,
                    &entry_index.file_reference(),
                    &mut file_info,
                    fs,
                    ntfs,
                    &params.hash,
                );

                // Grab any alternative data streams (ADS)
                let ads_result =
                    RawFilelist::get_ads_names(&entry_index.file_reference(), ntfs, fs);
                match ads_result {
                    Ok(result) => file_info.ads_info = result,
                    Err(err) => {
                        error!("[ntfs] Failed to grab ADS information: {err:?}");
                    }
                }
            }

            // Add to file metadata to Vec<RawFilelist> if it matches our start path and any optional regex
            if file_info.full_path.starts_with(&params.start_path)
                && regex_check(&params.path_regex, &file_info.full_path)
                && regex_check(&params.file_regex, &file_info.filename)
            {
                if params.metadata && file_info.is_file {
                    if let Ok(result) = parse_pe_file(&file_info.full_path) {
                        file_info.pe_info.push(result);
                    }
                }
                params.filelist.push(file_info.clone());

                // Grab IDX records in slack space if we have a directory and user selected to recover them
                // Recovering INDX records in slack space can return a verbose amount of data and increases procesing time
                if ntfs_file.is_directory() && params.recover_indx {
                    params.filelist.append(&mut RawFilelist::get_indx(
                        fs,
                        &ntfs_file,
                        &file_info.full_path,
                        &file_info.depth,
                    ));
                }
            }

            let max_list = 100000;
            // To keep memory usage small we only keep 100,000 files in the vec at a time
            if params.filelist.len() >= max_list {
                RawFilelist::output(&params.filelist, output, &params.start_time, &params.filter);
                params.filelist.clear();
            }

            // Begin the recursive file listing. But respect any provided max depth
            if ntfs_file.is_directory()
                && params.directory_tracker.len()
                    < (params.depth as usize + params.start_path_depth)
                && strings_contains(&params.start_path, &file_info.full_path)
            {
                // Track directories so we can build paths while recursing
                params.directory_tracker.push(dir_name);
                RawFilelist::walk_ntfs(ntfs_file, fs, ntfs, params, output)?;
            }
        }
        // At end of recursion remove directories we are done with
        params.directory_tracker.pop();
        Ok(())
    }

    /// Send raw file data to configured output preference based on `Output` parameter
    fn output(filelist: &[RawFilelist], output: &mut Output, start_time: &u64, filter: &bool) {
        let serde_data_result = serde_json::to_value(filelist);
        let serde_data = match serde_data_result {
            Ok(results) => results,
            Err(err) => {
                error!("[ntfs] Failed to serialize raw files: {err:?}");
                return;
            }
        };

        let output_result = output_data(&serde_data, "rawfiles", output, start_time, filter);
        match output_result {
            Ok(_) => {}
            Err(err) => {
                error!("[ntfs] Failed to output raw files data: {err:?}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RawFilelist;
    use crate::{
        artifacts::os::windows::ntfs::parser::{Hashes, Params},
        filesystem::ntfs::setup::setup_ntfs_parser,
        structs::artifacts::os::windows::RawFilesOptions,
        utils::{artemis_toml::Output, time::time_now},
    };
    use regex::Regex;
    use std::{collections::HashMap, path::PathBuf};

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
            compress,
            url: Some(String::new()),

            api_key: Some(String::new()),

            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: None,
            filter_script: None,
        }
    }

    #[test]
    fn test_raw_filelist() {
        let test_path = RawFilesOptions {
            drive_letter: 'C',
            start_path: String::from("C:\\"),
            depth: 1,
            recover_indx: true,
            md5: Some(false),
            sha1: Some(false),
            sha256: Some(false),
            metadata: Some(false),
            path_regex: Some(String::new()),
            filename_regex: Some(String::new()),
        };
        let mut output = output_options("rawfiles_temp", "local", "./tmp", false);

        let result = RawFilelist::raw_filelist(&test_path, &mut output, &false).unwrap();
        assert_eq!(result, ())
    }

    #[test]
    #[ignore = "Full file listing"]
    fn test_full_filelist() {
        let test_path = RawFilesOptions {
            drive_letter: 'C',
            start_path: String::from("C:\\"),
            depth: 99,
            recover_indx: false,
            md5: Some(false),
            sha1: Some(false),
            sha256: Some(false),
            metadata: Some(false),
            path_regex: Some(String::new()),
            filename_regex: Some(String::new()),
        };
        let mut output = output_options("rawfiles_temp", "local", "./tmp", false);

        let result = RawFilelist::raw_filelist(&test_path, &mut output, &false).unwrap();
        assert_eq!(result, ())
    }

    #[test]
    #[should_panic = "BadStart"]
    #[ignore = "Requires admin"]
    fn test_raw_filelist_bad_start() {
        let test_path = RawFilesOptions {
            drive_letter: 'C',
            start_path: String::from("I:\\"),
            depth: 8,
            recover_indx: true,
            md5: Some(true),
            sha1: Some(false),
            sha256: Some(false),
            metadata: Some(false),
            path_regex: Some(String::new()),
            filename_regex: Some(String::new()),
        };
        let mut output = output_options("rawfiles_temp", "local", "./tmp", false);

        let result = RawFilelist::raw_filelist(&test_path, &mut output, &false).unwrap();
        assert_eq!(result, ())
    }

    #[test]
    fn test_get_users() {
        let test_path = RawFilesOptions {
            drive_letter: 'C',
            start_path: String::from("C:\\Users"),
            depth: 1,
            recover_indx: true,
            md5: Some(true),
            sha1: Some(false),
            sha256: Some(false),
            metadata: Some(false),
            path_regex: Some(String::new()),
            filename_regex: Some(String::new()),
        };
        let mut output = output_options("rawfiles_temp", "local", "./tmp", false);
        let result = RawFilelist::raw_filelist(&test_path, &mut output, &false).unwrap();

        assert_eq!(result, ());
    }

    #[test]
    fn test_get_users_downloads() {
        let test_path = RawFilesOptions {
            drive_letter: 'C',
            start_path: String::from("C:\\Users"),
            depth: 3,
            recover_indx: true,
            md5: Some(true),
            sha1: Some(false),
            sha256: Some(false),
            metadata: Some(false),
            path_regex: Some(String::from(".*\\Downloads\\.*")),
            filename_regex: Some(String::new()),
        };
        let mut output = output_options("rawfiles_temp", "local", "./tmp", false);
        let result = RawFilelist::raw_filelist(&test_path, &mut output, &false).unwrap();

        assert_eq!(result, ());
    }

    #[test]
    fn test_get_rust_files() {
        let test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let letter = test_location.display().to_string().chars().next().unwrap();

        let test_path = RawFilesOptions {
            drive_letter: letter,
            start_path: test_location.display().to_string(),
            depth: 3,
            recover_indx: true,
            md5: Some(true),
            sha1: Some(false),
            sha256: Some(false),
            metadata: Some(false),
            path_regex: Some(String::new()),
            filename_regex: Some(String::from(r".*\.rs")),
        };
        let mut output = output_options("rawfiles_temp", "local", "./tmp", false);
        let result = RawFilelist::raw_filelist(&test_path, &mut output, &false).unwrap();

        assert_eq!(result, ());
    }

    #[test]
    fn test_walk_ntfs() {
        let test_path = RawFilesOptions {
            drive_letter: 'C',
            start_path: String::from("C:\\"),
            depth: 1,
            recover_indx: false,
            md5: Some(false),
            sha1: Some(false),
            sha256: Some(false),
            metadata: Some(false),
            path_regex: Some(String::new()),
            filename_regex: Some(String::new()),
        };
        let mut ntfs_parser = setup_ntfs_parser(&test_path.drive_letter).unwrap();
        let root_dir = ntfs_parser
            .ntfs
            .root_directory(&mut ntfs_parser.fs)
            .unwrap();

        let start_time = time_now();
        let mut output = output_options("rawfiles_temp", "local", "./tmp", false);
        let path_regex = Regex::new(&test_path.path_regex.as_ref().unwrap()).unwrap();
        let file_regex = Regex::new(&test_path.path_regex.as_ref().unwrap()).unwrap();
        let start_path_depth = test_path.start_path.split('\\').count();

        let hash_data = Hashes {
            md5: false,
            sha1: false,
            sha256: false,
        };
        let mut params = Params {
            start_path_depth,
            start_path: String::from("C:\\"),
            depth: 1,
            path_regex,
            file_regex,
            recover_indx: test_path.recover_indx,
            start_time,
            filelist: Vec::new(),
            directory_tracker: vec![format!("{}:", test_path.drive_letter)],
            sids: HashMap::new(),
            hash: hash_data,
            metadata: false,
            filter: false,
        };
        let result = RawFilelist::walk_ntfs(
            root_dir,
            &mut ntfs_parser.fs,
            &ntfs_parser.ntfs,
            &mut params,
            &mut output,
        )
        .unwrap();

        assert_eq!(result, ());
        assert!(params.filelist.len() > 1);
    }

    #[test]
    fn test_output() {
        let test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let letter = test_location.display().to_string().chars().next().unwrap();

        let test_path = RawFilesOptions {
            drive_letter: letter,
            start_path: String::new(),
            depth: 1,
            recover_indx: true,
            md5: Some(true),
            sha1: Some(false),
            sha256: Some(false),
            metadata: Some(false),
            path_regex: Some(String::new()),
            filename_regex: Some(String::new()),
        };
        let mut ntfs_parser = setup_ntfs_parser(&test_path.drive_letter).unwrap();
        let root_dir = ntfs_parser
            .ntfs
            .root_directory(&mut ntfs_parser.fs)
            .unwrap();

        let start_time = time_now();
        let mut output = output_options("rawfiles_temp", "local", "./tmp", false);

        let path_regex = Regex::new(&test_path.path_regex.as_ref().unwrap()).unwrap();
        let file_regex = Regex::new(&test_path.path_regex.as_ref().unwrap()).unwrap();
        let start_path_depth = test_path
            .start_path
            .split('\\')
            .map(|s| s.to_string())
            .count();
        let hash_data = Hashes {
            md5: true,
            sha1: false,
            sha256: false,
        };

        let mut params = Params {
            start_path_depth,
            start_path: String::from("C:\\"),
            depth: 2,
            path_regex,
            recover_indx: test_path.recover_indx,
            file_regex,
            start_time,
            filelist: Vec::new(),
            directory_tracker: vec![format!("{}:", test_path.drive_letter)],
            sids: HashMap::new(),
            hash: hash_data,
            metadata: false,
            filter: false,
        };
        let result = RawFilelist::walk_ntfs(
            root_dir,
            &mut ntfs_parser.fs,
            &ntfs_parser.ntfs,
            &mut params,
            &mut output,
        )
        .unwrap();

        assert_eq!(result, ());
        RawFilelist::output(&params.filelist, &mut output, &start_time, &false)
    }

    #[test]
    fn test_user_regex() {
        let reg = String::from(r".*");
        let regex = RawFilelist::user_regex(&reg).unwrap();
        assert_eq!(regex.as_str(), ".*");
    }
}
