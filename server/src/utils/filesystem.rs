use crate::utils::error::UtilServerError;
use flate2::bufread;
use log::{error, info};
use std::path::Path;
use tokio::fs::{create_dir_all, read, OpenOptions};
use tokio::io::Error;
use tokio::{fs::File, io::AsyncWriteExt};

/// Check if path is a file
pub(crate) fn is_file(path: &str) -> bool {
    let file = Path::new(path);
    if file.is_file() {
        return true;
    }
    false
}

/// Get size of a file
pub(crate) fn file_size(path: &str) -> u64 {
    let file = Path::new(path);
    if let Ok(value) = file.symlink_metadata() {
        return value.len();
    }

    0
}

/// Check if path is a directory
pub(crate) fn is_directory(path: &str) -> bool {
    let file = Path::new(path);
    if file.is_dir() {
        return true;
    }
    false
}

/// Read a file into memory
pub(crate) async fn read_file(path: &str) -> Result<Vec<u8>, UtilServerError> {
    // Verify provided path is a file
    if !is_file(path) {
        return Err(UtilServerError::NotFile);
    }

    let read_result = read(path).await;
    match read_result {
        Ok(result) => Ok(result),
        Err(err) => {
            error!("[server] Failed to read file {path}: {err:?}");
            Err(UtilServerError::ReadFile)
        }
    }
}

/// Write data to a file asynchronously. Supports gzip decompression
pub(crate) async fn write_file(data: &[u8], path: &str, decompress: bool) -> Result<(), Error> {
    // Decompression is synchronous
    if decompress {
        use std::{fs::File, io::copy};

        let mut file = File::create(path)?;
        let mut data = bufread::GzDecoder::new(data);
        copy(&mut data, &mut file)?;
        return Ok(());
    }

    let mut file = File::create(path).await?;

    file.write_all(data).await?;

    info!("[server] Wrote {} bytes to {path}", data.len());

    Ok(())
}

/// Append a line to a file. Automatically adds a newline
pub(crate) async fn append_file(data: &str, path: &str, limit: &u64) -> Result<(), Error> {
    let size = file_size(path);
    let append_file = &size < limit;

    let mut file = OpenOptions::new()
        .create(true)
        .append(append_file)
        .open(path)
        .await?;

    file.write_all(format!("{data}\n").as_bytes()).await?;

    Ok(())
}

/// Create a directory and all its parents
pub(crate) async fn create_dirs(path: &str) -> Result<(), UtilServerError> {
    let result = create_dir_all(path).await;
    if result.is_err() {
        error!(
            "[server] Failed to directory {path}: {:?}",
            result.unwrap_err()
        );
        return Err(UtilServerError::CreateDirectory);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::read_file;
    use crate::utils::filesystem::{create_dirs, is_directory, is_file, write_file};
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_read_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/server.toml");
        let config_path = test_location.display().to_string();
        let results = read_file(&config_path).await.unwrap();

        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_write_file() {
        create_dirs("./tmp").await.unwrap();

        let test = b"hello world!";
        write_file(test, "./tmp/test", false).await.unwrap();
    }

    #[tokio::test]
    async fn test_append_file() {
        create_dirs("./tmp").await.unwrap();

        let test = b"hello world!";
        write_file(test, "./tmp/test", false).await.unwrap();
    }

    #[tokio::test]
    async fn test_write_file_decompress() {
        let data = [
            31, 139, 8, 0, 215, 132, 7, 101, 0, 255, 5, 128, 65, 9, 0, 0, 8, 3, 171, 104, 55, 5,
            31, 7, 131, 125, 172, 63, 110, 65, 245, 50, 211, 1, 109, 194, 180, 3, 12, 0, 0, 0,
        ];
        create_dirs("./tmp").await.unwrap();

        let path = "./tmp/data.txt";

        write_file(&data, &path, true).await.unwrap();
    }

    #[test]
    fn test_is_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/server.toml");
        let config_path = test_location.display().to_string();
        let results = is_file(&config_path);

        assert!(results);
    }

    #[test]
    fn test_is_directory() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data");
        let config_path = test_location.display().to_string();
        let results = is_directory(&config_path);

        assert!(results);
    }

    #[tokio::test]
    async fn test_create_dirs() {
        create_dirs(&"./tmp/atest").await.unwrap();
    }
}
