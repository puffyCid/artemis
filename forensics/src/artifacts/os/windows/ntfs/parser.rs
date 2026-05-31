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
use super::{
    attributes::{file_data, filename_info, get_ads_names, get_reparse_type, standard_info},
    error::NTFSError,
    indx_slack::get_indx,
    security_ids::SecurityIDs,
};
use crate::{
    artifacts::os::windows::pe::parser::parse_pe_file,
    filesystem::{
        files::file_extension,
        ntfs::{sector_reader::SectorReader, setup::setup_ntfs_parser},
    },
    output2::{config::OutputFormat, manager::OutputManager, record::serialize_records_to_stream},
    structs::artifacts::os::windows::RawFilesOptions,
    utils::{
        regex_options::{create_regex, regex_check},
        strings::strings_contains,
    },
};
use common::files::Hashes;
use common::windows::RawFilelist;
use log::error;
use ntfs::{Ntfs, NtfsError, NtfsFile, structured_values::NtfsFileNamespace};
use regex::Regex;
use std::{collections::HashMap, fs::File, io::BufReader, mem::take};

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
    filelist: Vec<RawFilelist>,
    directory_tracker: Vec<String>,
    sids: HashMap<u32, SecurityIDs>,
}

/// Parse the raw NTFS data and get a file listing
pub(crate) fn ntfs_filelist(
    options: &RawFilesOptions,
    manager: &mut OutputManager,
) -> Result<(), NTFSError> {
    if options.start_path.is_empty() || !options.start_path.starts_with(options.drive_letter) {
        return Err(NTFSError::BadStart);
    }

    let ntfs_parser_result = setup_ntfs_parser(options.drive_letter);
    let mut ntfs_parser = match ntfs_parser_result {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Failed to get NTFS root directory, error: {err:?}");
            return Err(NTFSError::Parser);
        }
    };

    let root_dir_result = ntfs_parser.ntfs.root_directory(&mut ntfs_parser.fs);
    let root_dir = match root_dir_result {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Failed to get NTFS root directory, error: {err:?}");
            return Err(NTFSError::RootDir);
        }
    };

    let path_regex = user_regex(options.path_regex.as_ref().unwrap_or(&String::new()))?;
    let file_regex = user_regex(options.filename_regex.as_ref().unwrap_or(&String::new()))?;

    let mut start_path = options.start_path.clone();
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
    start_path.clone_from(&options.start_path);

    // Before parsing the NTFS data, grab Windows SIDs so we can map files to User and Group SIDs
    let sids = SecurityIDs::get_security_ids(&root_dir, &mut ntfs_parser.fs, &ntfs_parser.ntfs)?;

    let hash_data = Hashes {
        md5: options.md5.unwrap_or(false),
        sha1: options.sha1.unwrap_or(false),
        sha256: options.sha256.unwrap_or(false),
    };
    let mut params = Params {
        start_path_depth,
        start_path,
        depth: options.depth,
        path_regex,
        file_regex,
        recover_indx: options.recover_indx,
        filelist: Vec::new(),
        directory_tracker: vec![format!("{}:", options.drive_letter)],
        sids,
        hash: hash_data,
        metadata: options.metadata.unwrap_or(false),
    };

    let _ = walk_ntfs(
        root_dir,            // Start at NTFS root
        &mut ntfs_parser.fs, // BufReader to read parts of the NTFS
        &ntfs_parser.ntfs,   // Ntfs object
        &mut params, // Used to determinine what NTFS data to return. Ex: paths, starting location
        manager,
        options,
    );

    // Output any remaining file metadata
    raw_output(take(&mut params.filelist), manager, options);
    Ok(())
}

/// Create Regex based on provided input
fn user_regex(input: &str) -> Result<Regex, NTFSError> {
    let reg_result = create_regex(input);
    match reg_result {
        Ok(result) => Ok(result),
        Err(err) => {
            error!("[forensics] Bad regex: {input}, error: {err:?}");
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
    manager: &mut OutputManager,
    options: &RawFilesOptions,
) -> Result<(), NtfsError> {
    let index = root_dir.directory_index(fs)?;
    let mut iter = index.entries();
    while let Some(Ok(entry_index)) = iter.next(fs) {
        let mut file_info = RawFilelist {
            depth: params.directory_tracker.len(),
            drive: params.directory_tracker[0].clone(),
            inode: root_dir.file_record_number(),
            sequence_number: root_dir.sequence_number(),
            is_file: !root_dir.is_directory(),
            is_directory: root_dir.is_directory(),
            directory: params.directory_tracker.join("\\"),
            ..Default::default()
        };

        if let Some(Ok(value)) = entry_index.key()
            && value.namespace() == NtfsFileNamespace::Dos
        {
            continue;
        }

        let ntfs_file = entry_index.file_reference().to_file(ntfs, fs)?;
        filename_info(fs, &ntfs_file, &mut file_info)?;

        // Skip root directory loopback
        if file_info.filename == "." {
            continue;
        }

        // Get $STANDARD_INFORMATION attribute data. (4 timestamps, size, sid, owner, usn, attributes)
        let standard_result = ntfs_file.info()?;
        standard_info(&standard_result, &mut file_info);

        file_info.directory = params.directory_tracker.join("\\");
        file_info.full_path = format!("{}\\{}", file_info.directory, file_info.filename);
        file_info.is_file = !ntfs_file.is_directory();
        file_info.is_directory = ntfs_file.is_directory();
        file_info.inode = ntfs_file.file_record_number();
        file_info.sequence_number = ntfs_file.sequence_number();

        // Lookup traditional SID information (S-1-5-XXXXX) via the NTFS sid value
        (file_info.user_sid, file_info.group_sid) =
            SecurityIDs::lookup_sids(file_info.sid, &params.sids);

        let dir_name = file_info.filename.clone();

        if file_info.is_file && file_info.full_path.starts_with(&params.start_path) {
            file_info.extension = file_extension(&file_info.filename);

            // Grab file data for hashing
            let _attribute_result = file_data(&ntfs_file, &mut file_info, fs, ntfs, &params.hash);

            // Grab any alternative data streams (ADS)
            file_info.ads_info = get_ads_names(&ntfs_file, ntfs, fs)?;
        }

        if file_info
            .attributes
            .contains(&String::from("REPARSE_POINT"))
        {
            let result = get_reparse_type(&ntfs_file, ntfs, fs)?;
            file_info.attributes.push(format!("{result:?}"));
        }

        // Add to file metadata to Vec<RawFilelist> if it matches our start path and any optional regex
        if file_info.full_path.starts_with(&params.start_path)
            && regex_check(&params.path_regex, &file_info.full_path)
            && regex_check(&params.file_regex, &file_info.filename)
        {
            if params.metadata
                && file_info.is_file
                && let Ok(result) = parse_pe_file(&file_info.full_path)
            {
                file_info.pe_info.push(result);
            }

            params.filelist.push(file_info.clone());

            // Grab IDX records in slack space if we have a directory and user selected to recover them
            // Recovering INDX records in slack space can return a verbose amount of data and increases processing time
            if ntfs_file.is_directory() && params.recover_indx {
                params.filelist.append(&mut get_indx(
                    fs,
                    &ntfs_file,
                    &file_info.full_path,
                    file_info.depth,
                ));
            }
        }

        // If we are not parsing binary data and not timelining our limit is 10k, otherwise set limit to 1k
        let max_list = if !params.metadata && manager.config.format != OutputFormat::Timeline {
            10000
        } else {
            1000
        };
        // To keep memory usage small we only keep 10,000 files in the vec at a time
        if params.filelist.len() >= max_list {
            raw_output(take(&mut params.filelist), manager, options);
        }

        // Begin the recursive file listing. But respect any provided max depth
        if ntfs_file.is_directory()
            && params.directory_tracker.len() < (params.depth as usize + params.start_path_depth)
            && strings_contains(&params.start_path, &file_info.full_path)
        {
            // Track directories so we can build paths while recursing
            params.directory_tracker.push(dir_name);
            walk_ntfs(ntfs_file, fs, ntfs, params, manager, options)?;
        }
    }
    // At end of recursion remove directories we are done with
    params.directory_tracker.pop();

    Ok(())
}

/// Send raw file data to configured output preference based on `OutputManager` parameter
fn raw_output(entries: Vec<RawFilelist>, manager: &mut OutputManager, options: &RawFilesOptions) {
    let mut records = match serialize_records_to_stream(entries) {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Failed to serialize raw files: {err:?}");
            return;
        }
    };

    let artifact_name = "rawfiles";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("[forensics] Failed to output raw files data: {err:?}");
    }
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use crate::{
        artifacts::os::windows::ntfs::parser::{
            Hashes, Params, ntfs_filelist, raw_output, user_regex, walk_ntfs,
        },
        filesystem::ntfs::setup::setup_ntfs_parser,
        output2::{
            config::{OutputConfig, OutputDestination, OutputFormat},
            manager::OutputManager,
        },
        structs::artifacts::os::windows::RawFilesOptions,
    };
    use regex::Regex;
    use std::{collections::HashMap, mem::take, path::PathBuf};

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
    fn test_ntfs_filelist() {
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
        let mut output = output_options("rawfiles_temp", "./tmp", false);

        let result = ntfs_filelist(&test_path, &mut output).unwrap();
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
        let mut output = output_options("rawfiles_temp", "./tmp", false);

        let result = ntfs_filelist(&test_path, &mut output).unwrap();
        assert_eq!(result, ())
    }

    #[test]
    #[should_panic(expected = "BadStart")]
    fn test_ntfs_filelist_bad_start() {
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
        let mut output = output_options("rawfiles_temp", "./tmp", false);

        let result = ntfs_filelist(&test_path, &mut output).unwrap();
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
        let mut output = output_options("rawfiles_temp", "./tmp", false);
        let result = ntfs_filelist(&test_path, &mut output).unwrap();

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
        let mut output = output_options("rawfiles_temp", "./tmp", true);
        let result = ntfs_filelist(&test_path, &mut output).unwrap();

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
        let mut output = output_options("rawfiles_temp", "./tmp", true);
        let result = ntfs_filelist(&test_path, &mut output).unwrap();

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
        let mut ntfs_parser = setup_ntfs_parser(test_path.drive_letter).unwrap();
        let root_dir = ntfs_parser
            .ntfs
            .root_directory(&mut ntfs_parser.fs)
            .unwrap();

        let mut output = output_options("rawfiles_temp", "./tmp", false);
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
            filelist: Vec::new(),
            directory_tracker: vec![format!("{}:", test_path.drive_letter)],
            sids: HashMap::new(),
            hash: hash_data,
            metadata: false,
        };
        let result = walk_ntfs(
            root_dir,
            &mut ntfs_parser.fs,
            &ntfs_parser.ntfs,
            &mut params,
            &mut output,
            &test_path,
        )
        .unwrap();

        assert_eq!(result, ());
        assert!(params.filelist.len() > 1);
    }

    #[test]
    fn test_raw_output() {
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
        let mut ntfs_parser = setup_ntfs_parser(test_path.drive_letter).unwrap();
        let root_dir = ntfs_parser
            .ntfs
            .root_directory(&mut ntfs_parser.fs)
            .unwrap();

        let mut output = output_options("rawfiles_temp", "./tmp", false);

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
            filelist: Vec::new(),
            directory_tracker: vec![format!("{}:", test_path.drive_letter)],
            sids: HashMap::new(),
            hash: hash_data,
            metadata: false,
        };
        walk_ntfs(
            root_dir,
            &mut ntfs_parser.fs,
            &ntfs_parser.ntfs,
            &mut params,
            &mut output,
            &test_path,
        )
        .unwrap();

        raw_output(take(&mut params.filelist), &mut output, &test_path)
    }

    #[test]
    fn test_user_regex() {
        let reg = String::from(r".*");
        let regex = user_regex(&reg).unwrap();
        assert_eq!(regex.as_str(), ".*");
    }
}
