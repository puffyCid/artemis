use crate::{
    artifacts::os::systeminfo::info::get_disks,
    filesystem::error::FileSystemError,
    utils::{
        regex_options::{create_regex, regex_check},
        strings::strings_contains,
    },
};
use ext4_fs::{
    extfs::{Ext4Reader, Ext4ReaderAction},
    structs::{FileInfo, FileType},
};
use log::error;
use regex::Regex;
use serde::Serialize;
use std::{fs::File, io::BufReader};

/// Read a single file by parsing the EXT4 filesystem. This reads the entire file into memory
/// If we are provided a device path we will use that device to search for and read the file
/// If none is provided we will search all devices attached to the system and try to find the file
pub(crate) fn raw_read_file(path: &str, device: Option<&str>) -> Result<Vec<u8>, FileSystemError> {
    let mut file_data = Vec::new();
    if let Some(dev) = device {
        let mut ext4_options = Ext4Options {
            device: dev.to_string(),
            start_path: path.to_string(),
            depth: path.split("/").count(),
            start_path_depth: 0,
            path_regex: create_regex("").unwrap(), // Valid Regex, should never fail
            file_regex: create_regex("").unwrap(), // Valid Regex, should never fail
            filelist: Vec::new(),
            cache: Vec::new(),
        };

        let reader = match File::open(&ext4_options.device) {
            Ok(result) => result,
            Err(err) => {
                error!("[forensics] Could not open ext4 device ({dev}): {err:?}");
                return Err(FileSystemError::OpenFile);
            }
        };
        let buf = BufReader::new(reader);
        let mut ext_reader = match Ext4Reader::new(buf, 4096, 0) {
            Ok(result) => result,
            Err(err) => {
                error!("[forensics] Could not create ext4 reader for device ({dev}): {err:?}");
                return Err(FileSystemError::OpenFile);
            }
        };
        let root = get_root(&mut ext_reader)?;

        ext4_options
            .cache
            .push(root.name.trim_end_matches('/').to_string());

        iterate_ext4(&root, &mut ext_reader, &mut ext4_options);

        // Return the first found file that matched the provided path
        // There should only be one
        for entry in ext4_options.filelist {
            if entry.full_path != path {
                continue;
            }

            file_data = raw_read_inode(entry.inode, &mut ext_reader)?;
            break;
        }
        return Ok(file_data);
    }

    // We were not provided a device path
    // Now we will look at all attached ext4 devices on the system
    // First match we get, we return
    let disks = get_disks();
    for entry in disks {
        if entry.file_system.to_lowercase() != "ext4" {
            continue;
        }
        let dev = entry.name;
        let mut ext4_options = Ext4Options {
            device: dev.clone(),
            start_path: path.to_string(),
            depth: path.split("/").count(),
            start_path_depth: 0,
            path_regex: create_regex("").unwrap(), // Valid Regex, should never fail
            file_regex: create_regex("").unwrap(), // Valid Regex, should never fail
            filelist: Vec::new(),
            cache: Vec::new(),
        };

        let reader = match File::open(&ext4_options.device) {
            Ok(result) => result,
            Err(err) => {
                error!("[forensics] Could not open ext4 device ({dev}): {err:?}");
                return Err(FileSystemError::OpenFile);
            }
        };
        let buf = BufReader::new(reader);
        let mut ext_reader = match Ext4Reader::new(buf, 4096, 0) {
            Ok(result) => result,
            Err(err) => {
                error!("[forensics] Could not create ext4 reader for device ({dev}): {err:?}");
                return Err(FileSystemError::OpenFile);
            }
        };
        let root = get_root(&mut ext_reader)?;

        ext4_options
            .cache
            .push(root.name.trim_end_matches('/').to_string());

        iterate_ext4(&root, &mut ext_reader, &mut ext4_options);

        // Return the first found file that matched the provided path
        // There should only be one
        for file in ext4_options.filelist {
            if file.full_path != path {
                continue;
            }

            file_data = raw_read_inode(file.inode, &mut ext_reader)?;
            break;
        }
        // We read some bytes, now we are done
        if !file_data.is_empty() {
            break;
        }
    }

    Ok(file_data)
}

/// Read a **file** by its inode number. If you provide an inode number that is not associated with a file.
/// You will get an error returned
pub(crate) fn raw_read_inode<T: std::io::Seek + std::io::Read>(
    inode: u32,
    reader: &mut Ext4Reader<T>,
) -> Result<Vec<u8>, FileSystemError> {
    let bytes = match reader.read(inode) {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Could not read indoe ({inode}): {err:?}");
            return Err(FileSystemError::ReadFile);
        }
    };

    Ok(bytes)
}

/// Read a provided directory. The path may include regex. This function is similar to globbing
/// You can use it to pattern match for specific files or directories or other ext4 file types
pub(crate) fn raw_read_dir(
    path: &str,
    start: &str,
    device: Option<&str>,
) -> Result<Vec<Ext4Entry>, FileSystemError> {
    let mut files = Vec::new();
    if let Some(dev) = device {
        let path_regex = match create_regex(path) {
            Ok(result) => result,
            Err(err) => {
                error!("[forensics] Could not setup regex for reading directory ({path}): {err:?}");
                return Err(FileSystemError::ReadDirectory);
            }
        };
        let mut ext4_options = Ext4Options {
            device: dev.to_string(),
            start_path: start.to_string(),
            depth: path.split("/").count(),
            start_path_depth: 0,
            path_regex,
            file_regex: create_regex("").unwrap(), // Valid Regex, should never fail
            filelist: Vec::new(),
            cache: Vec::new(),
        };

        let reader = match File::open(&ext4_options.device) {
            Ok(result) => result,
            Err(err) => {
                error!("[forensics] Could not open ext4 device ({dev}): {err:?}");
                return Err(FileSystemError::OpenFile);
            }
        };
        let buf = BufReader::new(reader);
        let mut ext_reader = match Ext4Reader::new(buf, 4096, 0) {
            Ok(result) => result,
            Err(err) => {
                error!("[forensics] Could not create ext4 reader for device ({dev}): {err:?}");
                return Err(FileSystemError::OpenFile);
            }
        };
        let root = get_root(&mut ext_reader)?;

        ext4_options
            .cache
            .push(root.name.trim_end_matches('/').to_string());

        iterate_ext4(&root, &mut ext_reader, &mut ext4_options);

        files = ext4_options.filelist;
        return Ok(files);
    }

    // We were not provided a device path
    // Now we will look at all attached ext4 devices on the system
    // First match we get, we return
    let disks = get_disks();
    for entry in disks {
        if entry.file_system.to_lowercase() != "ext4" {
            continue;
        }
        let path_regex = match create_regex(path) {
            Ok(result) => result,
            Err(err) => {
                error!("[forensics] Could not setup regex for reading directory ({path}): {err:?}");
                return Err(FileSystemError::ReadDirectory);
            }
        };
        let dev = entry.name;
        let mut ext4_options = Ext4Options {
            device: dev.clone(),
            start_path: start.to_string(),
            depth: path.split("/").count(),
            start_path_depth: 0,
            path_regex,
            file_regex: create_regex("").unwrap(), // Valid Regex, should never fail
            filelist: Vec::new(),
            cache: Vec::new(),
        };

        let reader = match File::open(&ext4_options.device) {
            Ok(result) => result,
            Err(err) => {
                error!("[forensics] Could not open ext4 device ({dev}): {err:?}");
                return Err(FileSystemError::OpenFile);
            }
        };
        let buf = BufReader::new(reader);
        let mut ext_reader = match Ext4Reader::new(buf, 4096, 0) {
            Ok(result) => result,
            Err(err) => {
                error!("[forensics] Could not create ext4 reader for device ({dev}): {err:?}");
                return Err(FileSystemError::OpenFile);
            }
        };
        let root = get_root(&mut ext_reader)?;
        ext4_options
            .cache
            .push(root.name.trim_end_matches('/').to_string());

        iterate_ext4(&root, &mut ext_reader, &mut ext4_options);
        files.append(&mut ext4_options.filelist);
    }

    Ok(files)
}

/// Returns an inode for the provided file path
/// The inode can later be used to create a reader for the file (using `Ext4Reader.reader`) which can be used to used to stream the file
pub(crate) fn raw_reader<T: std::io::Seek + std::io::Read>(
    path: &str,
    reader: &mut Ext4Reader<T>,
) -> Result<u32, FileSystemError> {
    let mut ext4_options = Ext4Options {
        device: String::new(),
        start_path: path.to_string(),
        depth: path.split("/").count(),
        start_path_depth: 0,
        path_regex: create_regex("").unwrap(), // Valid Regex, should never fail
        file_regex: create_regex("").unwrap(), // Valid Regex, should never fail
        filelist: Vec::new(),
        cache: Vec::new(),
    };
    let root = get_root(reader)?;
    ext4_options
        .cache
        .push(root.name.trim_end_matches('/').to_string());
    iterate_ext4(&root, reader, &mut ext4_options);
    for file in ext4_options.filelist {
        if file.full_path != path {
            continue;
        }

        return Ok(file.inode);
    }

    error!("[forensics] Could not find inode for file ({path}).");
    Err(FileSystemError::ReadFile)
}

/// Setup options when reading the ext4 filesystem
pub(crate) struct Ext4Options {
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
    pub(crate) filelist: Vec<Ext4Entry>,
}

#[derive(Debug, Serialize)]
pub(crate) struct Ext4Entry {
    pub(crate) full_path: String,
    pub(crate) inode: u32,
    pub(crate) file_type: FileType,
}

/// Get the root directory for the ext4 filesystem
fn get_root<T: std::io::Seek + std::io::Read>(
    reader: &mut Ext4Reader<T>,
) -> Result<FileInfo, FileSystemError> {
    let root = match reader.root() {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Could not read the root ext4 directory: {err:?}");
            return Err(FileSystemError::RootDirectory);
        }
    };

    Ok(root)
}

/// Iterate through the EXT4 system and return entries based on provided start path and any regexes. Can be used to search for a file(s)
fn iterate_ext4<T: std::io::Seek + std::io::Read>(
    info: &FileInfo,
    reader: &mut Ext4Reader<T>,
    params: &mut Ext4Options,
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
            let ext4_entry = Ext4Entry {
                full_path: full_path.clone(),
                inode: entry.inode,
                file_type: entry.file_type,
            };

            params.filelist.push(ext4_entry);
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
            iterate_ext4(&dir_info, reader, params);
            params.cache.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::systeminfo::info::get_info_metadata,
        filesystem::ext4::raw_files::{
            Ext4Options, get_root, iterate_ext4, raw_read_dir, raw_read_file, raw_read_inode,
            raw_reader,
        },
        utils::regex_options::create_regex,
    };
    use ext4_fs::{
        extfs::{Ext4Reader, Ext4ReaderAction},
        structs::FileType,
    };
    use std::{
        fs::File,
        io::{BufReader, Read},
        path::PathBuf,
    };

    #[test]
    fn test_iterate_ext4() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/images/ext4/test.img");
        let mut options = Ext4Options {
            device: test_location.display().to_string(),
            start_path: String::from("/"),
            start_path_depth: 0,
            depth: 99,
            path_regex: create_regex("").unwrap(), // Valid Regex, should never fail
            file_regex: create_regex("").unwrap(), // Valid Regex, should never fail
            cache: Vec::new(),
            filelist: Vec::new(),
        };

        let reader = File::open(&options.device).unwrap();
        let buf = BufReader::new(reader);
        let mut ext_reader = Ext4Reader::new(buf, 4096, 0).unwrap();
        let root = ext_reader.root().unwrap();
        options
            .cache
            .push(root.name.trim_end_matches('/').to_string());

        iterate_ext4(&root, &mut ext_reader, &mut options);
        assert_eq!(options.filelist.len(), 7);

        for entry in options.filelist {
            if entry.full_path.contains("/ls") {
                assert_eq!(entry.inode, 17);
                assert!(entry.full_path.ends_with("/test/nest/ls"));
            } else if entry.full_path.contains("hello.txt") {
                assert_eq!(entry.inode, 14);
                assert!(entry.full_path.ends_with("/test/hello.txt"));
                assert_eq!(entry.file_type, FileType::File);
            }
        }
    }

    #[test]
    fn test_iterate_ext4_filter() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/images/ext4/test.img");
        let mut options = Ext4Options {
            device: test_location.display().to_string(),
            start_path: String::from("/"),
            start_path_depth: 0,
            depth: 99,
            path_regex: create_regex("").unwrap(), // Valid Regex, should never fail
            file_regex: create_regex("hello.txt").unwrap(), // Valid Regex, should never fail
            cache: Vec::new(),
            filelist: Vec::new(),
        };

        let reader = File::open(&options.device).unwrap();
        let buf = BufReader::new(reader);
        let mut ext_reader = Ext4Reader::new(buf, 4096, 0).unwrap();
        let root = ext_reader.root().unwrap();
        options
            .cache
            .push(root.name.trim_end_matches('/').to_string());

        iterate_ext4(&root, &mut ext_reader, &mut options);
        assert_eq!(options.filelist.len(), 1);
    }

    #[test]
    fn test_iterate_ext4_filter_dir() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/images/ext4/test.img");
        let mut options = Ext4Options {
            device: test_location.display().to_string(),
            start_path: String::from("/"),
            start_path_depth: 0,
            depth: 99,
            path_regex: create_regex("nest/findme").unwrap(), // Valid Regex, should never fail
            file_regex: create_regex("").unwrap(),            // Valid Regex, should never fail
            cache: Vec::new(),
            filelist: Vec::new(),
        };

        let reader = File::open(&options.device).unwrap();
        let buf = BufReader::new(reader);
        let mut ext_reader = Ext4Reader::new(buf, 4096, 0).unwrap();
        let root = ext_reader.root().unwrap();
        options
            .cache
            .push(root.name.trim_end_matches('/').to_string());

        iterate_ext4(&root, &mut ext_reader, &mut options);
        assert_eq!(options.filelist.len(), 1);
    }

    #[test]
    fn test_iterate_ext4_no_results() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/images/ext4/test.img");
        let mut options = Ext4Options {
            device: test_location.display().to_string(),
            start_path: String::from("/"),
            start_path_depth: 0,
            depth: 99,
            path_regex: create_regex("asdfasdfsdaf").unwrap(), // Valid Regex, should never fail
            file_regex: create_regex("").unwrap(),             // Valid Regex, should never fail
            cache: Vec::new(),
            filelist: Vec::new(),
        };

        let reader = File::open(&options.device).unwrap();
        let buf = BufReader::new(reader);
        let mut ext_reader = Ext4Reader::new(buf, 4096, 0).unwrap();
        let root = ext_reader.root().unwrap();
        options
            .cache
            .push(root.name.trim_end_matches('/').to_string());

        iterate_ext4(&root, &mut ext_reader, &mut options);
        assert_eq!(options.filelist.len(), 0);
    }

    #[test]
    fn test_iterate_ext4_bad_start() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/images/ext4/test.img");
        let mut options = Ext4Options {
            device: test_location.display().to_string(),
            start_path: String::from("/adfasdfsadf"),
            start_path_depth: 0,
            depth: 99,
            path_regex: create_regex("").unwrap(), // Valid Regex, should never fail
            file_regex: create_regex("").unwrap(), // Valid Regex, should never fail
            cache: Vec::new(),
            filelist: Vec::new(),
        };

        let reader = File::open(&options.device).unwrap();
        let buf = BufReader::new(reader);
        let mut ext_reader = Ext4Reader::new(buf, 4096, 0).unwrap();
        let root = ext_reader.root().unwrap();
        options
            .cache
            .push(root.name.trim_end_matches('/').to_string());

        iterate_ext4(&root, &mut ext_reader, &mut options);
        assert_eq!(options.filelist.len(), 0);
    }

    #[test]
    fn test_raw_read_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/images/ext4/test.img");

        let bytes = raw_read_file(
            "/run/media/puffycid/d32162ac-f1a7-487a-88ef-10c9ad4e5fff/test/hello.txt",
            Some(&test_location.display().to_string()),
        )
        .unwrap();
        assert_eq!(bytes.len(), 13);
    }

    #[test]
    fn test_raw_read_inode() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/images/ext4/test.img");
        let reader = File::open(&test_location.to_str().unwrap()).unwrap();
        let buf = BufReader::new(reader);
        let mut ext_reader = Ext4Reader::new(buf, 4096, 0).unwrap();
        let bytes = raw_read_inode(17, &mut ext_reader).unwrap();
        assert_eq!(bytes.len(), 145312);
    }

    #[test]
    fn test_raw_reader() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/images/ext4/test.img");
        let reader = File::open(&test_location.to_str().unwrap()).unwrap();
        let buf = BufReader::new(reader);
        let mut ext_reader = Ext4Reader::new(buf, 4096, 0).unwrap();
        let inode = raw_reader(
            "/run/media/puffycid/d32162ac-f1a7-487a-88ef-10c9ad4e5fff/test/nest/ls",
            &mut ext_reader,
        )
        .unwrap();
        let mut file_reader = ext_reader.reader(inode).unwrap();
        let mut buf = [0; 145312];
        file_reader.read_exact(&mut buf).unwrap();
        assert_ne!(buf, [0; 145312])
    }

    #[test]
    fn test_raw_read_dir() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/images/ext4/test.img");

        let start = "/run/media/";
        let path = ".*/.*.txt";
        let files = raw_read_dir(path, start, Some(&test_location.to_str().unwrap())).unwrap();
        assert_eq!(files.len(), 1);
    }

    #[test]
    #[should_panic(expected = "ReadFile")]
    fn test_raw_read_inode_bad() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/images/ext4/test.img");
        let reader = File::open(&test_location.to_str().unwrap()).unwrap();
        let buf = BufReader::new(reader);
        let mut ext_reader = Ext4Reader::new(buf, 4096, 0).unwrap();
        let _ = raw_read_inode(2, &mut ext_reader).unwrap();
    }

    #[test]
    #[should_panic(expected = "OpenFile")]
    fn test_raw_read_file_gibberish() {
        let _ = raw_read_file("dsfasdfsadf", Some("asdfasdfsdf")).unwrap();
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
    fn test_read_dir_live() {
        // Run test only in Github CI. Parsing the ext4 filesystem requires root
        if !get_info_metadata().kernel_version.contains("azure") {
            return;
        }
        let start = "/boot";
        let files = raw_read_dir("", start, None).unwrap();
        assert!(!files.is_empty());
    }

    #[test]
    #[should_panic(expected = "OpenFile")]
    fn test_raw_read_live_gibberish() {
        // Run test only in Github CI. Parsing the ext4 filesystem requires root
        if !get_info_metadata().kernel_version.contains("azure") {
            return;
        }
        let files = raw_read_file("sadfsadfsd", None).unwrap();
        assert!(!files.is_empty());
    }
}
