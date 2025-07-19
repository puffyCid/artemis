use super::{directory::is_directory, error::FileSystemError, metadata::get_metadata};
use common::files::Hashes;
use log::{error, warn};
use md5::{Digest, Md5};
use sha1::Sha1;
use sha2::Sha256;
use std::fs::read_to_string;
use std::io::{BufRead, BufReader, Lines};
use std::{
    fs::{File, read, read_dir},
    io::{Read, copy},
    path::Path,
};

/// Get a list of all files in a provided directory. Use `list_directories` to get only directories. Use `list_files_directories` to get both files and directories
pub(crate) fn list_files(path: &str) -> Result<Vec<String>, FileSystemError> {
    let data = list_files_directories(path)?;
    let mut files: Vec<String> = Vec::new();

    for entry in data {
        if !is_file(&entry) {
            continue;
        }
        files.push(entry);
    }
    Ok(files)
}

/// Get a list of all files and directories in a provided directory. Use `list_directories` to get only directories. Use `list_files` to get only files
pub(crate) fn list_files_directories(path: &str) -> Result<Vec<String>, FileSystemError> {
    let mut data: Vec<String> = Vec::new();
    if !is_directory(path) {
        return Err(FileSystemError::NotDirectory);
    }
    let dir_result = read_dir(path);
    let dir = match dir_result {
        Ok(result) => result,
        Err(err) => {
            error!("[core] Failed to get directory contents: {err:?}");
            return Err(FileSystemError::ReadDirectory);
        }
    };

    // Loop and get all files in provided directory
    for entry_result in dir {
        let entry = match entry_result {
            Ok(result) => result,
            Err(err) => {
                error!("[core] Failed to get directory entry: {err:?}");
                continue;
            }
        };

        let full_path = entry.path().display().to_string();
        data.push(full_path);
    }

    Ok(data)
}

/// Check if path is a file
pub(crate) fn is_file(path: &str) -> bool {
    let file = Path::new(path);
    if file.is_file() {
        return true;
    }
    false
}

/// Read a file that is less than 2GB in size
/// Use `read_file_custom` to use a custom max size limit
pub(crate) fn read_file(path: &str) -> Result<Vec<u8>, FileSystemError> {
    if file_too_large(path) {
        return Err(FileSystemError::LargeFile);
    }
    file_read(path)
}

/// Read a text file that is less that 2GB in size
pub(crate) fn read_text_file(path: &str) -> Result<String, FileSystemError> {
    if file_too_large(path) {
        return Err(FileSystemError::LargeFile);
    }
    file_read_text(path)
}

/// Return a `Lines<BufReader>` to iterate through a text file smaller than 2GB
pub(crate) fn file_lines(path: &str) -> Result<Lines<BufReader<File>>, FileSystemError> {
    if file_too_large(path) {
        return Err(FileSystemError::LargeFile);
    }
    let reader = file_reader(path)?;
    let buf_reader = BufReader::new(reader);
    Ok(buf_reader.lines())
}

/// Create a `File` object that can be used to read a file
pub(crate) fn file_reader(path: &str) -> Result<File, FileSystemError> {
    // Verify provided path is a file
    if !is_file(path) {
        return Err(FileSystemError::NotFile);
    }

    let read_result = File::open(path);
    let reader = match read_result {
        Ok(result) => result,
        Err(err) => {
            error!("[core] Failed to open file {path}: {err:?}");
            return Err(FileSystemError::OpenFile);
        }
    };

    Ok(reader)
}

/// Read a file that is less than the provided size in bytes
/// Use `read_file_large` to read a file of any size or use `read_file` to use the the default size limit of 2GB
pub(crate) fn read_file_custom(path: &str, size: u64) -> Result<Vec<u8>, FileSystemError> {
    if file_too_large_custom(path, size) {
        return Err(FileSystemError::LargeFile);
    }
    file_read(path)
}

/// Read a file into memory
fn file_read(path: &str) -> Result<Vec<u8>, FileSystemError> {
    // Verify provided path is a file
    if !is_file(path) {
        return Err(FileSystemError::NotFile);
    }

    let read_result = read(path);
    match read_result {
        Ok(result) => Ok(result),
        Err(err) => {
            error!("[core] Failed to read file {path}: {err:?}");
            Err(FileSystemError::ReadFile)
        }
    }
}

/// Read a whole text file into a string
fn file_read_text(path: &str) -> Result<String, FileSystemError> {
    // Verify provided path is a file
    if !is_file(path) {
        return Err(FileSystemError::NotFile);
    }

    let data = read_to_string(path);
    match data {
        Ok(result) => Ok(result),
        Err(err) => {
            error!("[core] Failed to read text file {path}: {err:?}");
            Err(FileSystemError::ReadFile)
        }
    }
}

/// Hash the data of an already read file
pub(crate) fn hash_file_data(hashes: &Hashes, data: &[u8]) -> (String, String, String) {
    let mut md5_string = String::new();
    let mut sha1_string = String::new();
    let mut sha256_string = String::new();

    let mut md5 = Md5::new();
    let mut sha1 = Sha1::new();
    let mut sha256 = Sha256::new();

    if hashes.md5 {
        md5.update(data);
        let hash = md5.finalize();
        md5_string = format!("{hash:x}");
    }
    if hashes.sha1 {
        sha1.update(data);
        let hash = sha1.finalize();
        sha1_string = format!("{hash:x}");
    }
    if hashes.sha256 {
        sha256.update(data);
        let hash = sha256.finalize();
        sha256_string = format!("{hash:x}");
    }

    (md5_string, sha1_string, sha256_string)
}

/// Read a file in chunks and hash its contents. Returns MD5, SHA1, and/or SHA256 hashes
pub(crate) fn hash_file(hashes: &Hashes, path: &str) -> (String, String, String) {
    let mut md5_string = String::new();
    let mut sha1_string = String::new();
    let mut sha256_string = String::new();

    // Verify provided path is a file
    if !is_file(path) {
        return (md5_string, sha1_string, sha256_string);
    }

    let mut md5 = Md5::new();
    let mut sha1 = Sha1::new();
    let mut sha256 = Sha256::new();
    let file_open = File::open(path);
    let mut file = match file_open {
        Ok(result) => result,
        Err(err) => {
            error!("[core] Failed to hash file {path}: {err:?}");
            return (md5_string, sha1_string, sha256_string);
        }
    };

    // Read file in chunks so we do not read large files all into memory
    loop {
        let temp_buff_size = 65536;
        let mut temp_buff: Vec<u8> = vec![0u8; temp_buff_size];
        let bytes_result = file.read(&mut temp_buff);
        let bytes = match bytes_result {
            Ok(result) => result,
            Err(err) => {
                error!("[core] Failed to read file: {err:?}");
                return (md5_string, sha1_string, sha256_string);
            }
        };
        let finished = 0;
        if bytes == finished {
            break;
        }

        // Make sure our temp buff does not have any extra zeros from the initialization
        if bytes < temp_buff_size {
            temp_buff = temp_buff[0..bytes].to_vec();
        }

        if hashes.md5 {
            let _ = copy(&mut temp_buff.as_slice(), &mut md5);
        }
        if hashes.sha1 {
            let _ = copy(&mut temp_buff.as_slice(), &mut sha1);
        }
        if hashes.sha256 {
            let _ = copy(&mut temp_buff.as_slice(), &mut sha256);
        }
    }

    if hashes.md5 {
        let hash = md5.finalize();
        md5_string = format!("{hash:x}");
    }
    if hashes.sha1 {
        let hash = sha1.finalize();
        sha1_string = format!("{hash:x}");
    }
    if hashes.sha256 {
        let hash = sha256.finalize();
        sha256_string = format!("{hash:x}");
    }

    (md5_string, sha1_string, sha256_string)
}

/// Get the extension of a file if any
pub(crate) fn file_extension(path: &str) -> String {
    let file = Path::new(path);
    let extension_osstr = file.extension();

    let extension = match extension_osstr {
        Some(result) => result.to_str().unwrap_or(""),
        _ => "",
    };
    extension.to_string()
}

/// Get the file size
pub(crate) fn get_file_size(path: &str) -> u64 {
    if !is_file(path) {
        return 0;
    }

    let meta = get_metadata(path);
    match meta {
        Ok(result) => result.len(),
        Err(err) => {
            error!("[core] Failed to get file size: {err:?}");
            0
        }
    }
}

/// Check if a provided file is too large than the default acceptable size (2GB).
/// Use `file_too_large_custom` if you want to increase/decrease the default acceptable size
pub(crate) fn file_too_large(path: &str) -> bool {
    let size = get_file_size(path);
    let max_size = 2147483648; // 2GB
    if size < max_size {
        return false;
    }
    true
}

/// Check if a provided file is too large than the a custom acceptable size
fn file_too_large_custom(path: &str, max_size: u64) -> bool {
    let size = get_file_size(path);
    if size < max_size {
        return false;
    }
    true
}

/// Get last component of provided path. Will be filename or directory or empty string if final component cannot be determined
pub(crate) fn get_filename(path: &str) -> String {
    if !path.contains(['/', '\\']) {
        return path.to_string();
    }

    let entry_opt = if path.contains('/') {
        path.rsplit_once('/')
    } else {
        path.rsplit_once('\\')
    };

    if entry_opt.is_none() {
        warn!("[core] Failed to get filename from: {path}");
        return path.to_string();
    }

    let (_, name) = entry_opt.unwrap_or_default();
    name.to_string()
}

#[cfg(test)]
mod tests {
    use crate::filesystem::files::{
        file_extension, file_lines, file_read_text, file_reader, file_too_large,
        file_too_large_custom, get_file_size, get_filename, hash_file, hash_file_data, is_file,
        list_files, list_files_directories, read_file, read_file_custom, read_text_file,
    };
    use common::files::Hashes;
    use std::path::PathBuf;

    #[test]
    fn test_list_files_directories() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests");
        let result = list_files_directories(&test_location.display().to_string()).unwrap();

        assert!(result.len() > 3);
        let mut emond = false;
        let mut ntfs = false;
        let mut test_data = false;
        for entry in result {
            if entry.ends_with("ntfs_tester.rs") {
                ntfs = true;
            } else if entry.ends_with("emond_tester.rs") {
                emond = true;
            } else if entry.ends_with("test_data") {
                test_data = true;
            }
        }

        assert_eq!(emond, true);
        assert_eq!(ntfs, true);
        assert_eq!(test_data, true);
    }

    #[test]
    fn test_file_lines() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/dfir/LICENSE");

        let results = file_lines(&test_location.display().to_string()).unwrap();
        assert_eq!(results.count(), 21);
    }

    #[test]
    fn test_hash_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/system/files/test.txt");
        let hashes = Hashes {
            md5: true,
            sha1: true,
            sha256: true,
        };

        let (md5, sha1, sha256) = hash_file(&hashes, &test_location.display().to_string());

        assert_eq!(md5, "220c0b91fd1d000ad08441675dab02c8");
        assert_eq!(sha1, "9962ed200cdca61a0daad6a045c920e09ffdea50");
        assert_eq!(
            sha256,
            "717c544adc58fef85c134ab5283226d175bce1b236b305415b501ad9043470b5"
        );
    }

    #[test]
    fn test_file_extension() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/system/files/test.txt");

        let result = file_extension(&test_location.display().to_string());

        assert_eq!(result, "txt");
    }

    #[test]
    fn test_read_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/system/files/test.txt");

        let result = read_file(&test_location.display().to_string()).unwrap();

        assert_eq!(result.len(), 23);
    }

    #[test]
    fn test_read_text_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/system/files/test.txt");

        let result = read_text_file(&test_location.display().to_string()).unwrap();

        assert_eq!(result, "hello, world! Its Rust!");
    }

    #[test]
    fn test_file_read_text() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/system/files/test.txt");

        let result = file_read_text(&test_location.display().to_string()).unwrap();

        assert_eq!(result, "hello, world! Its Rust!");
    }

    #[test]
    fn test_get_file_size() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/system/files/test.txt");

        let result = get_file_size(&test_location.display().to_string());

        assert_eq!(result, 23);
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn test_get_filename() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/fsevents_tester.rs");
        let result = get_filename(&test_location.display().to_string());
        assert_eq!(result, "fsevents_tester.rs");
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_get_filename() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\fsevents_tester.rs");
        let result = get_filename(&test_location.display().to_string());
        assert_eq!(result, "fsevents_tester.rs");
    }

    #[test]
    fn test_is_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/fsevents_tester.rs");
        let result = is_file(&test_location.display().to_string());
        assert_eq!(result, true);
    }

    #[test]
    fn test_list_files() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests");
        let result = list_files(&test_location.display().to_string()).unwrap();

        assert!(result.len() > 3);
        let mut emond = false;
        let mut ntfs = false;
        let mut test_data = false;
        for entry in result {
            if entry.ends_with("ntfs_tester.rs") {
                ntfs = true;
            } else if entry.ends_with("emond_tester.rs") {
                emond = true;
            } else if entry.ends_with("test_data") {
                test_data = true;
            }
        }

        assert_eq!(emond, true);
        assert_eq!(ntfs, true);
        assert_eq!(test_data, false);
    }

    #[test]
    fn test_file_too_large() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/system/files/test.txt");

        let result = file_too_large(&test_location.display().to_string());
        assert_eq!(result, false)
    }

    #[test]

    fn test_file_too_large_custom() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/system/files/test.txt");

        let result = file_too_large_custom(&test_location.display().to_string(), 10000);
        assert_eq!(result, false)
    }

    #[test]

    fn test_read_file_custom() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/system/files/test.txt");

        let result = read_file_custom(&test_location.display().to_string(), 10000).unwrap();
        assert_eq!(result.len(), 23);
    }

    #[test]
    fn test_hash_file_data() {
        let test = b"rust is nice";
        let hashes = Hashes {
            md5: true,
            sha1: true,
            sha256: true,
        };
        let (md5, sha1, sha256) = hash_file_data(&hashes, test);
        assert_eq!(md5, "ea89b88102ee16fb2de0f7e655edf085");
        assert_eq!(sha1, "c5ffe7432430f67b88eec45136ad4e9baaf1aa84");
        assert_eq!(
            sha256,
            "e50231ef2d3836b4c27010f87a9b463df336123982f1612f2414df60d3d58560"
        );
    }

    #[test]
    fn test_file_reader() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/system/files/test.txt");
        let _result = file_reader(&test_location.display().to_string()).unwrap();
    }
}
