use super::error::FileSystemError;
use log::{error, info};
use std::path::Path;
use tokio::{
    fs::{File, OpenOptions, read},
    io::AsyncWriteExt,
};

/// Read a file into memory
pub(crate) async fn read_file(path: &str) -> Result<Vec<u8>, FileSystemError> {
    // Verify provided path is a file
    if !is_file(path) {
        return Err(FileSystemError::NotFile);
    }

    let read_result = read(path).await;
    match read_result {
        Ok(result) => Ok(result),
        Err(err) => {
            error!("[client] Failed to read file {path}: {err:?}");
            Err(FileSystemError::ReadFile)
        }
    }
}

/// Check if path is a file
pub(crate) fn is_file(path: &str) -> bool {
    let file = Path::new(path);
    if file.is_file() {
        return true;
    }
    false
}

/// Write data to a file asynchronously
pub(crate) async fn write_file(data: &[u8], path: &str) -> Result<(), FileSystemError> {
    let file_result = File::create(path).await;
    let mut file = match file_result {
        Ok(result) => result,
        Err(err) => {
            error!("[client] Failed to create file {path}: {err:?}");
            return Err(FileSystemError::CreateFile);
        }
    };

    let status = file.write_all(data).await;
    if status.is_err() {
        error!(
            "[client] Failed to write file {path}: {:?}",
            status.unwrap_err()
        );
        return Err(FileSystemError::WriteFile);
    }
    info!("[client] Wrote {} bytes to {path}", data.len());

    Ok(())
}

/// Append data to a target file
pub(crate) async fn append_file(data: &[u8], path: &str) -> Result<(), FileSystemError> {
    let file_result = OpenOptions::new().append(true).open(path).await;
    let mut file = match file_result {
        Ok(result) => result,
        Err(err) => {
            error!("[client] Failed to append file {path}: {err:?}");
            return Err(FileSystemError::AppendFile);
        }
    };

    let status = file.write_all(data).await;
    if status.is_err() {
        error!(
            "[client] Failed to write file {path}: {:?}",
            status.unwrap_err()
        );
        return Err(FileSystemError::WriteFile);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::read_file;
    use crate::filesystem::directory::create_dirs;
    use crate::filesystem::files::{append_file, is_file, write_file};
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_read_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/client.toml");
        let config_path = test_location.display().to_string();
        let results = read_file(&config_path).await.unwrap();

        assert!(!results.is_empty());
    }

    #[test]
    fn test_is_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/client.toml");
        let config_path = test_location.display().to_string();
        let results = is_file(&config_path);

        assert!(results);
    }

    #[tokio::test]
    async fn test_write_file() {
        create_dirs("./tmp").await.unwrap();

        let test = b"hello world!";
        write_file(test, "./tmp/test").await.unwrap();
    }

    #[tokio::test]
    async fn test_append_file() {
        create_dirs("./tmp").await.unwrap();

        let test = b"hello world!";
        write_file(test, "./tmp/test").await.unwrap();
        append_file(test, "./tmp/test").await.unwrap();
    }
}
