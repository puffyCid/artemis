use super::{
    attributes::{get_attribute_data, get_filename_attribute},
    compression::check_wofcompressed,
    sector_reader::SectorReader,
    setup::setup_ntfs_parser,
};
use crate::{
    artifacts::os::systeminfo::info::get_platform,
    filesystem::{error::FileSystemError, files::read_file_custom},
    utils::{
        regex_options::{create_regex, regex_check},
        strings::strings_contains,
    },
};
use common::files::Hashes;
use log::{error, warn};
use md5::{Digest, Md5};
use ntfs::{
    Ntfs, NtfsError, NtfsFile, NtfsFileReference, NtfsReadSeek, attribute_value::NtfsAttributeValue,
};
use regex::Regex;
use sha1::Sha1;
use sha2::Sha256;
use std::{
    fs::File,
    io::{BufReader, copy},
};

/// Read the whole attribute data. This can be used to read a whole file
pub(crate) fn raw_read_data(
    data_attr_value: &mut NtfsAttributeValue<'_, '_>,
    fs: &mut BufReader<SectorReader<File>>,
) -> Result<Vec<u8>, NtfsError> {
    let mut buff_data: Vec<u8> = Vec::new();
    loop {
        let temp_buff_size = 65536;
        let mut temp_buff: Vec<u8> = vec![0u8; temp_buff_size];
        let bytes = data_attr_value.read(fs, &mut temp_buff)?;

        let finished = 0;
        if bytes == finished {
            return Ok(buff_data);
        }

        // Make sure our temp buff does not any have extra zeros from the intialization
        if bytes < temp_buff_size {
            buff_data.append(&mut temp_buff[0..bytes].to_vec());
        } else {
            buff_data.append(&mut temp_buff);
        }
    }
}

/// Return the file reference number for a file. Can be used to create reader to stream the file
pub(crate) fn raw_reader<'a>(
    path: &str,
    ntfs: &'a Ntfs,
    fs: &mut BufReader<SectorReader<File>>,
) -> Result<NtfsFile<'a>, FileSystemError> {
    let min_path_len = 4;
    if path.len() < min_path_len || !path.contains(':') {
        return Err(FileSystemError::NotFile);
    }

    let drive = &path.chars().next().unwrap(); // Only need Drive letter
    let root_dir_result = ntfs.root_directory(fs);
    let root_dir = match root_dir_result {
        Ok(result) => result,
        Err(err) => {
            error!("[core] Failed to get NTFS root directory, error: {err:?}");
            return Err(FileSystemError::RootDirectory);
        }
    };

    let mut ntfs_options = NtfsOptions {
        start_path: path.to_string(),
        start_path_depth: 0,
        depth: path.split('\\').count(),
        path_regex: create_regex("").unwrap(), // Valid Regex, should never fail
        file_regex: create_regex("").unwrap(), // Valid Regex, should never fail
        filelist: Vec::new(),
        directory_tracker: vec![format!("{drive}:")],
    };

    // Search and iterate through the NTFS system for the file
    let _ = iterate_ntfs(root_dir, fs, ntfs, &mut ntfs_options);

    // Loop through filelisting. It should only have one entry
    for filelist in ntfs_options.filelist {
        if filelist.full_path != path {
            continue;
        }

        let ntfs_file_result = filelist.file.to_file(ntfs, fs);
        let ntfs_file = match ntfs_file_result {
            Ok(result) => result,
            Err(err) => {
                error!("[core] Failed to get NTFS root directory, error: {err:?}");
                return Err(FileSystemError::NtfsSectorReader);
            }
        };

        // Return the file reference
        return Ok(ntfs_file);
    }

    warn!("[core] Could not create reader for {path}");
    Err(FileSystemError::OpenFile)
}

/// Given a file $DATA attribute, read and hash the data
pub(crate) fn raw_hash_data(
    data_attr_value: &mut NtfsAttributeValue<'_, '_>,
    fs: &mut BufReader<SectorReader<File>>,
    hash_data: &Hashes,
) -> (String, String, String) {
    let mut md5 = Md5::new();
    let mut sha1 = Sha1::new();
    let mut sha256 = Sha256::new();
    loop {
        let temp_buff_size = 65536;
        let mut temp_buff: Vec<u8> = vec![0u8; temp_buff_size];
        let bytes_result = data_attr_value.read(fs, &mut temp_buff);
        let bytes = match bytes_result {
            Ok(result) => result,
            Err(err) => {
                error!("[core] Failed to read data for hashing: {err:?}");
                break;
            }
        };
        let finished = 0;
        if bytes == finished {
            break;
        }

        // Make sure our temp buff does not have any extra zeros from the intialization
        if bytes < temp_buff_size {
            temp_buff = temp_buff[0..bytes].to_vec();
        }

        if hash_data.md5 {
            let _ = copy(&mut temp_buff.as_slice(), &mut md5);
        }
        if hash_data.sha1 {
            let _ = copy(&mut temp_buff.as_slice(), &mut sha1);
        }
        if hash_data.sha256 {
            let _ = copy(&mut temp_buff.as_slice(), &mut sha256);
        }
    }

    let mut md5_string = String::new();
    let mut sha1_string = String::new();
    let mut sha256_string = String::new();

    if hash_data.md5 {
        let hash = md5.finalize();
        md5_string = format!("{hash:x}");
    }
    if hash_data.sha1 {
        let hash = sha1.finalize();
        sha1_string = format!("{hash:x}");
    }
    if hash_data.sha256 {
        let hash = sha256.finalize();
        sha256_string = format!("{hash:x}");
    }

    (md5_string, sha1_string, sha256_string)
}

/// Read a single file by parsing the NTFS system
pub(crate) fn raw_read_file(path: &str) -> Result<Vec<u8>, FileSystemError> {
    // Raw file access only works on Windows. For all other platforms redirect to normal file access
    let platform = get_platform();
    if platform != "Windows" {
        // 3GB limit
        let max_size = 3221225472;
        return read_file_custom(path, max_size);
    }

    let min_path_len = 4;
    if path.len() < min_path_len || !path.contains(':') {
        return Err(FileSystemError::NotFile);
    }

    let drive = path.chars().next().unwrap(); // Only need Drive letter
    let mut ntfs_parser = setup_ntfs_parser(drive)?;
    let root_dir_result = ntfs_parser.ntfs.root_directory(&mut ntfs_parser.fs);
    let root_dir = match root_dir_result {
        Ok(result) => result,
        Err(err) => {
            error!("[core] Failed to get NTFS root directory, error: {err:?}");
            return Err(FileSystemError::RootDirectory);
        }
    };

    let mut ntfs_options = NtfsOptions {
        start_path: path.to_string(),
        start_path_depth: 0,
        depth: path.split('\\').count(),
        path_regex: create_regex("").unwrap(), // Valid Regex, should never fail
        file_regex: create_regex("").unwrap(), // Valid Regex, should never fail
        filelist: Vec::new(),
        directory_tracker: vec![format!("{drive}:")],
    };

    // Search and iterate through the NTFS system for the file
    let _ = iterate_ntfs(
        root_dir,
        &mut ntfs_parser.fs,
        &ntfs_parser.ntfs,
        &mut ntfs_options,
    );

    let mut file_data: Vec<u8> = Vec::new();
    for filelist in ntfs_options.filelist {
        if filelist.full_path != path {
            continue;
        }

        file_data = raw_read_by_file_ref(filelist.file, &ntfs_parser.ntfs, &mut ntfs_parser.fs)?;
        break;
    }

    Ok(file_data)
}

/**
* Read raw file by file reference
* This function will check if the data is compressed
* NTFS supports two (2) types of compression:
*   NTFS native compression - File data compressed via the NTFS (uses lzxpress huffman)
*   `WofCompression` - File data compressed via OS. Only occurs on Windows 10+ (uses lzxpress huffman)

* NTFS compression can be detected by checking the standard attributes for the value `COMPRESSED`
* `WofCompression` can be detected by checking for the alternative data stream (ADS) attribute `WofCompressedData`
* `raw_read_by_file_ref` can decompress the data and return a hash of the uncompressed data
*/
pub(crate) fn raw_read_by_file_ref(
    ntfs_ref: NtfsFileReference,
    ntfs: &Ntfs,
    fs: &mut BufReader<SectorReader<File>>,
) -> Result<Vec<u8>, FileSystemError> {
    let compress_check = check_wofcompressed(ntfs_ref, ntfs, fs);
    match compress_check {
        Ok((is_compressed, uncompressed_data, _compressed_size)) => {
            if is_compressed {
                return Ok(uncompressed_data);
            }
        }
        Err(err) => {
            error!(
                "[core] Could not check for decompression error: {err:?}. Returning regular data."
            );
        }
    }

    let ntfs_file_result = ntfs_ref.to_file(ntfs, fs);
    let ntfs_file = match ntfs_file_result {
        Ok(result) => result,
        Err(err) => {
            error!("[core] Failed to get NTFS file, error: {err:?}");
            return Err(FileSystemError::NotFile);
        }
    };
    let data_name = "";
    let ntfs_data_option = ntfs_file.data(fs, data_name);
    let ntfs_data_result = match ntfs_data_option {
        Some(result) => result,
        None => return Err(FileSystemError::FileData), // Some files do not have data stored under "". Ex: $UsnJrnl data is stored under "$J"
    };

    let ntfs_data = match ntfs_data_result {
        Ok(result) => result,
        Err(err) => {
            error!("[core] Failed to get NTFS data error: {err:?}");
            return Err(FileSystemError::FileData);
        }
    };

    let ntfs_attribute_result = ntfs_data.to_attribute();
    let ntfs_attribute = match ntfs_attribute_result {
        Ok(result) => result,
        Err(err) => {
            error!("[core] Failed to get NTFS attribute error: {err:?}");
            return Err(FileSystemError::NoAttribute);
        }
    };

    let data_result = ntfs_attribute.value(fs);
    let mut data_attr_value = match data_result {
        Ok(result) => result,
        Err(err) => {
            error!("[core] Failed to get NTFS attribute data error: {err:?}");
            return Err(FileSystemError::NoDataAttributeValue);
        }
    };

    let file_data_result = raw_read_data(&mut data_attr_value, fs);
    let file_data = match file_data_result {
        Ok(result) => result,
        Err(err) => {
            error!("[core] Could not read file error: {err:?}");
            return Err(FileSystemError::ReadFile);
        }
    };
    Ok(file_data)
}

/// Read a provided NTFS attribute. Can be used to read non-resident Alternative Data Streams (ADS)
pub(crate) fn read_attribute(path: &str, attribute: &str) -> Result<Vec<u8>, FileSystemError> {
    let min_path_len = 4;
    if path.len() < min_path_len || !path.contains(':') {
        return Err(FileSystemError::NotFile);
    }
    let drive = path.chars().next().unwrap(); // Only need Drive letter
    let mut ntfs_parser = setup_ntfs_parser(drive)?;

    let root_dir_result = ntfs_parser.ntfs.root_directory(&mut ntfs_parser.fs);
    let root_dir = match root_dir_result {
        Ok(result) => result,
        Err(err) => {
            error!("[core] Failed to get NTFS root directory, error: {err:?}");
            return Err(FileSystemError::RootDirectory);
        }
    };

    let mut ntfs_options = NtfsOptions {
        start_path: path.to_string(),
        start_path_depth: 0,
        depth: path.split('\\').count(),
        path_regex: create_regex("").unwrap(), // Valid Regex, should never fail
        file_regex: create_regex("").unwrap(), // Valid Regex, should never fail
        filelist: Vec::new(),
        directory_tracker: vec![format!("{drive}:")],
    };

    // Search and iterate through the NTFS system for the file
    let _ = iterate_ntfs(
        root_dir,
        &mut ntfs_parser.fs,
        &ntfs_parser.ntfs,
        &mut ntfs_options,
    );

    for filelist in ntfs_options.filelist {
        if filelist.full_path != path {
            continue;
        }

        let data_result = get_attribute_data(
            filelist.file,
            &ntfs_parser.ntfs,
            &mut ntfs_parser.fs,
            attribute,
        );
        match data_result {
            Ok(result) => return Ok(result),
            Err(err) => {
                error!("[core] Could not get data for attribute {attribute} at {path}: {err:?}");
                break;
            }
        }
    }

    Err(FileSystemError::NoAttribute)
}

/// Struct containing File references to a User registry file (either NTUSER.dat or UsrClass.dat)
#[derive(Debug)]
pub(crate) struct UserRegistryFiles {
    pub(crate) reg_reference: NtfsFileReference,
    pub(crate) full_path: String,
    pub(crate) filename: String, // Either NTUSER.DAT or UsrClass.dat
}

/// Get paths and NTFS file references for all user Registry files on a drive (NTUSER.DAT and UsrClass.dat)
pub(crate) fn get_user_registry_files(
    drive: char,
) -> Result<Vec<UserRegistryFiles>, FileSystemError> {
    let mut ntfs_parser = setup_ntfs_parser(drive)?;
    let root_dir_result = ntfs_parser.ntfs.root_directory(&mut ntfs_parser.fs);
    let root_dir = match root_dir_result {
        Ok(result) => result,
        Err(err) => {
            error!("[core] Failed to get NTFS root directory, error: {err:?}");
            return Err(FileSystemError::RootDirectory);
        }
    };

    let mut ntfs_options = NtfsOptions {
        start_path: format!("{drive}:\\Users"),
        start_path_depth: 1,
        depth: 6,
        path_regex: create_regex("").unwrap(), // Valid Regex, should never fail
        file_regex: create_regex("(?i)(NTUSER|UsrClass)\\.DAT$").unwrap(), // Valid Regex, should never fail
        filelist: Vec::new(),
        directory_tracker: vec![format!("{drive}:")],
    };

    // Search and iterate through the NTFS system for User Registry files
    let _ = iterate_ntfs(
        root_dir,
        &mut ntfs_parser.fs,
        &ntfs_parser.ntfs,
        &mut ntfs_options,
    );

    let mut user_reg_files: Vec<UserRegistryFiles> = Vec::new();

    // Remove any possible false postives
    for entries in ntfs_options.filelist {
        let ntuser_depth = 4;
        let mut reg_file = UserRegistryFiles {
            reg_reference: entries.file,
            full_path: entries.full_path,
            filename: String::new(),
        };
        let usrclass_path = "\\appdata\\local\\microsoft\\windows\\usrclass.dat";
        println!(
            "{}-{}",
            reg_file.full_path,
            reg_file.full_path.split('\\').count()
        );
        if reg_file.full_path.to_lowercase().ends_with("ntuser.dat")
            && reg_file.full_path.split('\\').count() == ntuser_depth
        {
            reg_file.filename = String::from("NTUSER.DAT");
            user_reg_files.push(reg_file);
        } else if reg_file.full_path.to_lowercase().ends_with(usrclass_path) {
            reg_file.filename = String::from("UsrClass.dat");
            user_reg_files.push(reg_file);
        }
    }

    Ok(user_reg_files)
}

/// Options for iterating the NTFS system
pub(crate) struct NtfsOptions {
    pub(crate) start_path: String,
    pub(crate) start_path_depth: usize,
    pub(crate) depth: usize,
    pub(crate) path_regex: Regex,
    pub(crate) file_regex: Regex,
    pub(crate) filelist: Vec<NtfsEntry>,
    pub(crate) directory_tracker: Vec<String>,
}

#[derive(Debug)]
/// Store all File References we want
pub(crate) struct NtfsEntry {
    pub(crate) full_path: String,
    pub(crate) file: NtfsFileReference,
}

/// Iterate through the NTFS system and return entries based on provided start path and any regexes. Can be used to search for a file(s)
pub(crate) fn iterate_ntfs(
    root_dir: NtfsFile<'_>,
    fs: &mut BufReader<SectorReader<File>>,
    ntfs: &Ntfs,
    params: &mut NtfsOptions,
) -> Result<(), FileSystemError> {
    let index_result = root_dir.directory_index(fs);
    let index = match index_result {
        Ok(result) => result,
        Err(err) => {
            error!("[core] Failed to get NTFS index directory, error: {err:?}");
            return Err(FileSystemError::IndexDirectory);
        }
    };
    let mut iter = index.entries();
    // Go through all files in a directory. If we find another directory iterate that directory if conditions are true
    while let Some(entry) = iter.next(fs) {
        let entry_result = entry;
        let entry_index = match entry_result {
            Ok(result) => result,
            Err(err) => {
                error!("[core] Failed to get NTFS entry index, error: {err:?}");
                continue;
            }
        };

        let filename_result = entry_index.key();
        // Get $FILENAME attribute data.
        let filename = match filename_result {
            Some(result) => get_filename_attribute(&result)?,
            None => continue,
        };

        // Skip root directory loopback or DOS type names
        if filename.name() == "."
            || filename
                .name()
                .to_string()
                .unwrap_or_default()
                .contains('~')
        {
            continue;
        }

        let ntfs_file_ref = entry_index.file_reference();

        let ntfs_file_result = entry_index.file_reference().to_file(ntfs, fs);
        let ntfs_file = match ntfs_file_result {
            Ok(result) => result,
            Err(err) => {
                error!("[core] Failed to get NTFS file, error: {err:?}");
                continue;
            }
        };

        let directory = params.directory_tracker.join("\\");
        let full_path = format!("{}\\{}", directory, filename.name());
        let name = filename.name();

        // Add to file metadata to Vec<RawFilelist> if it matches our start path and any optional regex
        if full_path.starts_with(&params.start_path)
            && regex_check(&params.path_regex, &full_path)
            && regex_check(
                &params.file_regex,
                &filename.name().to_string().unwrap_or_default(),
            )
        {
            let ntfs_entry = NtfsEntry {
                full_path: full_path.clone(),
                file: ntfs_file_ref,
            };

            params.filelist.push(ntfs_entry);
        }

        // Begin the recursive file listing. But respect any provided max depth
        if ntfs_file.is_directory()
            && params.directory_tracker.len() < (params.depth + params.start_path_depth)
            && strings_contains(&params.start_path, &full_path)
        {
            // Track directories so we can build paths while recursing
            params
                .directory_tracker
                .push(name.to_string().unwrap_or_default());
            iterate_ntfs(ntfs_file, fs, ntfs, params)?;
        }
    }
    // At end of recursion remove directories we are done with
    params.directory_tracker.pop();
    Ok(())
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::{NtfsOptions, get_user_registry_files, iterate_ntfs, raw_reader};
    use crate::{
        filesystem::ntfs::{
            raw_files::{
                raw_hash_data, raw_read_by_file_ref, raw_read_data, raw_read_file, read_attribute,
            },
            sector_reader::SectorReader,
            setup::setup_ntfs_parser,
        },
        utils::regex_options::create_regex,
    };
    use common::files::Hashes;
    use ntfs::Ntfs;
    use std::{fs::File, io::BufReader, path::PathBuf};

    #[test]
    fn test_hash_data() {
        let drive_path = "\\\\.\\C:";
        let fs = File::open(drive_path).unwrap();

        let reader_sector_size = 4096;
        let sector_reader = SectorReader::new(fs, reader_sector_size).unwrap();
        let mut fs = BufReader::new(sector_reader);
        let ntfs = Ntfs::new(&mut fs).unwrap();
        let root_dir = ntfs.root_directory(&mut fs).unwrap();

        let index = root_dir.directory_index(&mut fs).unwrap();
        let mut iter = index.entries();
        let hashes = Hashes {
            md5: true,
            sha1: true,
            sha256: true,
        };

        while let Some(entry) = iter.next(&mut fs) {
            let entry_index = entry.unwrap();

            let ntfs_file = entry_index
                .file_reference()
                .to_file(&ntfs, &mut fs)
                .unwrap();

            if !ntfs_file.is_directory() {
                let ntfs_data = ntfs_file.data(&mut fs, "").unwrap().unwrap();
                let ntfs_attribute = ntfs_data.to_attribute().unwrap();
                let mut data_attr_value = ntfs_attribute.value(&mut fs).unwrap();
                let (md5, sha1, sha256) = raw_hash_data(&mut data_attr_value, &mut fs, &hashes);

                assert_eq!(md5.is_empty(), false);
                assert_eq!(sha1.is_empty(), false);
                assert_eq!(sha256.is_empty(), false);
                break;
            }
        }
    }

    #[test]
    fn test_read_data() {
        let drive_path = "\\\\.\\C:";
        let fs = File::open(drive_path).unwrap();

        let reader_sector_size = 4096;
        let sector_reader = SectorReader::new(fs, reader_sector_size).unwrap();
        let mut fs = BufReader::new(sector_reader);
        let ntfs = Ntfs::new(&mut fs).unwrap();
        let root_dir = ntfs.root_directory(&mut fs).unwrap();

        let index = root_dir.directory_index(&mut fs).unwrap();
        let mut iter = index.entries();
        while let Some(entry) = iter.next(&mut fs) {
            let entry_index = entry.unwrap();

            let ntfs_file = entry_index
                .file_reference()
                .to_file(&ntfs, &mut fs)
                .unwrap();

            if !ntfs_file.is_directory() {
                let ntfs_data = ntfs_file.data(&mut fs, "").unwrap().unwrap();
                let ntfs_attribute = ntfs_data.to_attribute().unwrap();
                let mut data_attr_value = ntfs_attribute.value(&mut fs).unwrap();

                let buffer = raw_read_data(&mut data_attr_value, &mut fs).unwrap();
                assert_eq!(buffer.is_empty(), false);

                break;
            }
        }
    }

    #[test]
    fn test_get_user_registry_files() {
        let result = get_user_registry_files('C').unwrap();

        // Should at least have three (3). User (NTUSER and UsrClass), Default (NTUSER)
        assert!(result.len() >= 3);
        let mut default = false;
        for entry in result {
            if entry.full_path.contains("Default") {
                default = true;
            }
        }
        assert_eq!(default, true)
    }

    #[test]
    fn test_raw_read_by_file_ref() {
        let result = get_user_registry_files('C').unwrap();

        // Should at least have three (3). User (NTUSER and UsrClass), Default (NTUSER)
        assert!(result.len() >= 3);
        let mut default = false;
        let mut ntfs_parser = setup_ntfs_parser('C').unwrap();
        for entry in result {
            if entry.full_path.contains("Default") {
                default = true;
            }
            let buffer_result =
                raw_read_by_file_ref(entry.reg_reference, &ntfs_parser.ntfs, &mut ntfs_parser.fs)
                    .unwrap();
            assert!(buffer_result.len() > 10000);
        }
        assert_eq!(default, true)
    }

    #[test]
    fn test_raw_read_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\system\\files\\test.txt");

        let result = raw_read_file(&test_location.display().to_string()).unwrap();
        assert_eq!(result.len(), 23);
    }

    #[test]
    fn test_read_attribute() {
        let result = read_attribute("C:\\$Extend\\$UsnJrnl", "$J").unwrap();
        assert!(result.len() > 10)
    }

    #[test]
    fn test_read_wofcompressed_file() {
        // explorer.exe may be compressed in Win10+
        let result = raw_read_file("C:\\Windows\\explorer.exe").unwrap();
        assert!(result.len() > 1000);
    }

    #[test]
    fn test_iterate_ntfs() {
        let mut ntfs_parser = setup_ntfs_parser('C').unwrap();
        let root_dir = ntfs_parser
            .ntfs
            .root_directory(&mut ntfs_parser.fs)
            .unwrap();

        let mut ntfs_options = NtfsOptions {
            start_path: String::from("C:\\"),
            start_path_depth: 0,
            depth: 1,
            path_regex: create_regex("").unwrap(), // Valid Regex, should never fail
            file_regex: create_regex("").unwrap(), // Valid Regex, should never fail
            filelist: Vec::new(),
            directory_tracker: vec![String::from("C:")],
        };

        let _ = iterate_ntfs(
            root_dir,
            &mut ntfs_parser.fs,
            &ntfs_parser.ntfs,
            &mut ntfs_options,
        );

        assert!(ntfs_options.filelist.len() > 0);
    }

    #[test]
    fn test_raw_reader() {
        let mut ntfs_parser = setup_ntfs_parser('C').unwrap();
        let result = raw_reader(
            "C:\\Windows\\explorer.exe",
            &ntfs_parser.ntfs,
            &mut ntfs_parser.fs,
        )
        .unwrap();
        assert!(result.file_record_number() > 5);
    }
}
