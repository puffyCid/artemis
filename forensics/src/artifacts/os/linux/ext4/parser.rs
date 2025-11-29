/**
 * Linux EXT4 is a common filesystem used on Linux  
 * This parser leverages the `ext4-fs` Rust crate to parse the raw filesystem  
 *
 * References:  
 *   `https://wiki.osdev.org/Ext4`  
 *   `https://metebalci.com/blog/a-minimum-complete-tutorial-of-linux-ext4-file-system/`  
 *   `https://github.com/libyal/libfsext`
 *
 * Other Parsers:  
 *  `https://github.com/Velocidex/velociraptor`
 */
use crate::{
    artifacts::os::{
        linux::{
            artifacts::output_data,
            ext4::{disks::qcow_ext4, error::Ext4Error},
        },
        systeminfo::info::get_disks,
    },
    filesystem::files::file_extension,
    structs::{artifacts::os::linux::Ext4Options, toml::Output},
    utils::{
        regex_options::{create_regex, regex_check},
        strings::strings_contains,
        time::{time_now, unixepoch_nanoseconds_to_iso},
    },
};
use common::linux::Ext4Filelist;
use ext4_fs::{
    extfs::{Ext4Reader, Ext4ReaderAction},
    structs::{Ext4Hash, FileInfo, FileType},
};
use log::error;
use regex::Regex;
use std::{fs::File, io::BufReader};

/// Parse the raw EXT4 data and get a file listing
pub(crate) fn ext4_filelisting(
    params: &Ext4Options,
    output: &mut Output,
    filter: bool,
) -> Result<(), Ext4Error> {
    let user_path_regex = params.path_regex.clone().unwrap_or_default();
    let user_file_regex = params.filename_regex.clone().unwrap_or_default();

    let start_time = time_now();
    if let Some(device) = &params.device {
        let hashing = Ext4Hash {
            md5: params.md5.unwrap_or_default(),
            sha1: params.sha1.unwrap_or_default(),
            sha256: params.sha256.unwrap_or_default(),
        };
        let path_regex = filesystem_regex(&user_path_regex)?;
        let file_regex = filesystem_regex(&user_file_regex)?;
        let mut options = Ext4Params {
            device: device.clone(),
            start_path: params.start_path.clone(),
            start_path_depth: 0,
            depth: params.depth as usize,
            path_regex,
            file_regex,
            cache: Vec::new(),
            filelist: Vec::new(),
            hashing,
            start_time,
            filter,
        };
        if options.device.starts_with("qcow://") {
            return qcow_ext4(&mut options, output, start_time);
        }
        let reader = match File::open(&options.device) {
            Ok(result) => result,
            Err(err) => {
                error!("[forensics] Could not open ext4 device ({device}): {err:?}");
                return Err(Ext4Error::Device);
            }
        };
        let buf = BufReader::new(reader);
        let mut ext_reader = match Ext4Reader::new(buf, 4096, 0) {
            Ok(result) => result,
            Err(err) => {
                error!("[forensics] Could not create ext4 reader for device ({device}): {err:?}");
                return Err(Ext4Error::Device);
            }
        };
        let root = get_root(&mut ext_reader)?;
        options
            .cache
            .push(root.name.trim_end_matches('/').to_string());
        walk_ext4(&root, &mut ext_reader, &mut options, output);
        if !options.filelist.is_empty() {
            ext4_output(&options.filelist, output, start_time, filter);
        }
    } else {
        let disks = get_disks();
        for entry in disks {
            if entry.file_system.to_lowercase() != "ext4" {
                continue;
            }
            let hashing = Ext4Hash {
                md5: params.md5.unwrap_or_default(),
                sha1: params.sha1.unwrap_or_default(),
                sha256: params.sha256.unwrap_or_default(),
            };
            let device = entry.name;
            let path_regex = filesystem_regex(&user_path_regex)?;
            let file_regex = filesystem_regex(&user_file_regex)?;
            let mut options = Ext4Params {
                device: device.clone(),
                start_path: params.start_path.clone(),
                start_path_depth: 0,
                depth: params.depth as usize,
                path_regex,
                file_regex,
                cache: Vec::new(),
                filelist: Vec::new(),
                hashing,
                start_time,
                filter,
            };
            let reader = match File::open(&options.device) {
                Ok(result) => result,
                Err(err) => {
                    error!("[forensics] Could not open ext4 device ({device}): {err:?}");
                    return Err(Ext4Error::Device);
                }
            };
            let buf = BufReader::new(reader);
            let mut ext_reader = match Ext4Reader::new(buf, 4096, 0) {
                Ok(result) => result,
                Err(err) => {
                    error!(
                        "[forensics] Could not create ext4 reader for device ({device}): {err:?}"
                    );
                    return Err(Ext4Error::Device);
                }
            };
            let root = get_root(&mut ext_reader)?;
            options
                .cache
                .push(root.name.trim_end_matches('/').to_string());
            walk_ext4(&root, &mut ext_reader, &mut options, output);
            if !options.filelist.is_empty() {
                ext4_output(&options.filelist, output, start_time, filter);
            }
        }
    }

    Ok(())
}

/// Apply any regex to our filelisting
fn filesystem_regex(regex_string: &str) -> Result<Regex, Ext4Error> {
    let value = match create_regex(regex_string) {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Bad regex provided ({regex_string}): {err:?}");
            return Err(Ext4Error::Regex);
        }
    };
    Ok(value)
}

/// Get the root directory for the ext4 filesystem
pub(crate) fn get_root<T: std::io::Seek + std::io::Read>(
    reader: &mut Ext4Reader<T>,
) -> Result<FileInfo, Ext4Error> {
    let root = match reader.root() {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Could not read the root ext4 directory: {err:?}");
            return Err(Ext4Error::RootDir);
        }
    };

    Ok(root)
}

/// Setup options when reading the ext4 filesystem
pub(crate) struct Ext4Params {
    /// We need a device path. Ex: /dev/sda1. If none is provided, we attempt to get a list using `get_disk`.
    /// We then attempt to iterate through all ext4 disks
    pub(crate) device: String,
    /// Start path may get updated if an ext4 image was mounted.  
    /// If a /home partition is mounted to /run/media and then unmounted.  
    /// The `start_path` would become /run/media because the ext4 header last mount path was updated.  
    /// Not applicable for live systems
    pub(crate) start_path: String,
    pub(crate) start_path_depth: usize,
    pub(crate) depth: usize,
    pub(crate) path_regex: Regex,
    pub(crate) file_regex: Regex,
    /// Cache the directories we are iterating through
    pub(crate) cache: Vec<String>,
    pub(crate) filelist: Vec<Ext4Filelist>,
    pub(crate) hashing: Ext4Hash,
    pub(crate) start_time: u64,
    pub(crate) filter: bool,
}

/// Walk the entire ext4 filesystem
pub(crate) fn walk_ext4<T: std::io::Seek + std::io::Read>(
    info: &FileInfo,
    reader: &mut Ext4Reader<T>,
    params: &mut Ext4Params,
    output: &mut Output,
) {
    for entry in &info.children {
        if entry.name == "." || entry.name == ".." {
            continue;
        }
        let path = params.cache.join("/");
        let filename = entry.name.trim_end_matches('/');
        let full_path = format!("{path}/{}", filename);

        if full_path.starts_with(&params.start_path)
            && regex_check(&params.path_regex, &full_path)
            && regex_check(&params.file_regex, filename)
        {
            let meta = match reader.stat(entry.inode) {
                Ok(result) => result,
                Err(err) => {
                    error!("[forensics] Could not stat the file {full_path}: {err:?}");
                    continue;
                }
            };
            let mut ext4_entry = Ext4Filelist {
                full_path: full_path.clone(),
                inode: entry.inode as u64,
                file_type: entry.file_type,
                directory: path,
                filename: filename.to_string(),
                extension: file_extension(filename),
                created: unixepoch_nanoseconds_to_iso(meta.created),
                modified: unixepoch_nanoseconds_to_iso(meta.modified),
                changed: unixepoch_nanoseconds_to_iso(meta.changed),
                accessed: unixepoch_nanoseconds_to_iso(meta.accessed),
                size: meta.size,
                uid: meta.uid,
                gid: meta.gid,
                is_sparse: meta.is_sparse,
                permissions: meta.permission,
                hard_links: meta.hard_links,
                extended_attributes: meta.extended_attributes,
                inode_type: meta.inode_type,
                md5: String::new(),
                sha1: String::new(),
                sha256: String::new(),
            };
            if (params.hashing.md5 || params.hashing.sha256 || params.hashing.sha1)
                && entry.file_type == FileType::File
            {
                let hashes = match reader.hash(entry.inode, &params.hashing) {
                    Ok(result) => result,
                    Err(err) => {
                        error!("[forensics] Could not hash the file {full_path}: {err:?}");
                        continue;
                    }
                };
                ext4_entry.md5 = hashes.md5;
                ext4_entry.sha1 = hashes.sha1;
                ext4_entry.sha256 = hashes.sha256;
            }

            let max_size = 10000;
            params.filelist.push(ext4_entry);
            if params.filelist.len() >= max_size {
                ext4_output(&params.filelist, output, params.start_time, params.filter);
                params.filelist = Vec::new();
            }
        }

        if entry.file_type == FileType::Directory
            && params.cache.len() < (params.depth + params.start_path_depth)
            && strings_contains(&params.start_path, &full_path)
        {
            params.cache.push(filename.to_string());
            let dir_info = match reader.read_dir(entry.inode) {
                Ok(value) => value,
                Err(err) => {
                    error!("[forensics] Failed to read ext4 directory, error: {err:?}");
                    continue;
                }
            };
            walk_ext4(&dir_info, reader, params, output);
            params.cache.pop();
        }
    }
}

/// Every 10k files we output the results
pub(crate) fn ext4_output(
    filelist: &[Ext4Filelist],
    output: &mut Output,
    start_time: u64,
    filter: bool,
) {
    let serde_data_result = serde_json::to_value(filelist);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[forensics] Failed to serialize ext4 files: {err:?}");
            return;
        }
    };

    let output_result = output_data(&mut serde_data, "ext4files", output, start_time, filter);
    match output_result {
        Ok(_) => {}
        Err(err) => {
            error!("[forensics] Failed to output ext4 files data: {err:?}");
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::linux::ext4::parser::{
            Ext4Params, ext4_filelisting, ext4_output, filesystem_regex, get_root, walk_ext4,
        },
        structs::{artifacts::os::linux::Ext4Options, toml::Output},
        utils::regex_options::create_regex,
    };
    use ext4_fs::{
        extfs::{Ext4Reader, Ext4ReaderAction},
        structs::{Ext4Hash, FileType},
    };
    use std::{fs::File, io::BufReader, path::PathBuf};

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
            compress,
            timeline: false,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: None,
            filter_script: None,
            logging: None,
        }
    }

    #[test]
    fn test_ext4_filelisting() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/images/ext4/test.img");
        let mut output = output_options("ext4_files_temp", "local", "./tmp", false);
        let options = Ext4Options {
            start_path: String::from("/"),
            depth: 99,
            device: Some(test_location.display().to_string()),
            md5: None,
            sha1: None,
            sha256: None,
            path_regex: None,
            filename_regex: None,
        };
        ext4_filelisting(&options, &mut output, false).unwrap();
    }

    #[test]
    fn test_walk_ext4() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/images/ext4/test.img");
        let mut output = output_options("ext4_files_temp", "local", "./tmp", false);

        let mut params = Ext4Params {
            device: test_location.display().to_string(),
            start_path: String::from("/"),
            start_path_depth: 0,
            depth: 99,
            path_regex: create_regex("").unwrap(), // Valid Regex, should never fail
            file_regex: create_regex("").unwrap(), // Valid Regex, should never fail
            cache: Vec::new(),
            filelist: Vec::new(),
            hashing: Ext4Hash {
                md5: true,
                sha1: false,
                sha256: false,
            },
            start_time: 0,
            filter: false,
        };

        let reader = File::open(&params.device).unwrap();
        let buf = BufReader::new(reader);
        let mut ext_reader = Ext4Reader::new(buf, 4096, 0).unwrap();
        let root = ext_reader.root().unwrap();
        params
            .cache
            .push(root.name.trim_end_matches('/').to_string());
        walk_ext4(&root, &mut ext_reader, &mut params, &mut output);

        assert_eq!(params.filelist.len(), 7);
        for entry in params.filelist {
            if entry.file_type == FileType::File {
                assert!(!entry.md5.is_empty())
            } else if entry.inode == 16 {
                assert!(
                    format!("{:?}", entry.extended_attributes)
                        .contains("unconfined_u:object_r:unlabeled_t:s0")
                )
            }
        }
    }

    #[test]
    #[should_panic(expected = "Regex")]
    fn test_filesystem_regex_bad_input() {
        let _ = filesystem_regex("[").unwrap();
    }

    #[test]
    fn test_get_root() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/images/ext4/test.img");
        let reader = File::open(&test_location.to_str().unwrap()).unwrap();
        let buf = BufReader::new(reader);
        let mut ext_reader = Ext4Reader::new(buf, 4096, 0).unwrap();
        let root = get_root(&mut ext_reader).unwrap();
        assert_eq!(root.inode, 2);
    }

    #[test]
    fn test_ext4_output() {
        let mut output = output_options("ext4_files_none", "local", "./tmp", false);
        ext4_output(&[], &mut output, 0, false);
    }
}
