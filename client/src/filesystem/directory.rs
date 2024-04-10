use super::error::FileSystemError;
use log::error;
use tokio::fs::create_dir_all;

/// Create a directory and all its parents
pub(crate) async fn create_dirs(path: &str) -> Result<(), FileSystemError> {
    let result = create_dir_all(path).await;
    if result.is_err() {
        error!(
            "[client] Failed to create directory {path}: {:?}",
            result.unwrap_err()
        );
        return Err(FileSystemError::CreateDirectory);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::filesystem::directory::create_dirs;

    #[tokio::test]
    async fn test_create_dirs() {
        create_dirs(&"./tmp/atest").await.unwrap();
    }
}
