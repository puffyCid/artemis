use crate::{
    artifacts::enrollment::{EndpointInfo, EndpointStatic},
    filestore::error::StoreError,
    utils::{
        filesystem::{create_dirs, write_file},
        time::time_now,
        uuid::generate_uuid,
    },
};
use common::server::EndpointOS;
use log::error;
use serde::Serialize;

/// Create the endpoint storage directory and generate an ID
pub(crate) async fn create_endpoint_path(
    path: &str,
    endpoint: &EndpointInfo,
) -> Result<String, StoreError> {
    let id = generate_uuid();

    let data = EndpointStatic {
        hostname: endpoint.hostname.clone(),
        platform: endpoint.platform.clone(),
        tags: Vec::new(),
        notes: Vec::new(),
        checkin: time_now(),
        id,
    };

    let serde_result = serde_json::to_vec(&data);
    let value = match serde_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Failed to serialize enrollment: {err:?}");
            return Err(StoreError::Serialize);
        }
    };

    let endpoint_path = format!("{path}/{}/{}", data.platform, data.id);

    let status = create_dirs(&endpoint_path).await;
    if status.is_err() {
        error!(
            "[server] Failed to create endpoint storage directory: {:?}",
            status.unwrap_err()
        );
        return Err(StoreError::CreateDirectory);
    }

    let enroll_file = format!("{endpoint_path}/enroll.json");
    let jobs_file = format!("{endpoint_path}/jobs.json");
    let heartbeat_file = format!("{endpoint_path}/heartbeat.jsonl");
    let pulse_file = format!("{endpoint_path}/pulse.json");

    create_enroll_file(&enroll_file, &value).await?;
    create_enroll_file(&jobs_file, &[]).await?;
    create_enroll_file(&heartbeat_file, &[]).await?;
    create_enroll_file(&pulse_file, &[]).await?;

    Ok(data.id)
}

/// Create enrollment related files
async fn create_enroll_file(path: &str, data: &[u8]) -> Result<(), StoreError> {
    let status = write_file(data, path, false).await;
    if status.is_err() {
        error!(
            "[server] Failed to write endpoint enrollment file {path}: {:?}",
            status.unwrap_err()
        );
        return Err(StoreError::WriteFile);
    }

    Ok(())
}

/// Simple way to count Endpoint OS enrollment
pub(crate) async fn endpoint_count(path: &str, os: &EndpointOS) -> Result<usize, StoreError> {
    let count = match os {
        EndpointOS::All => glob_paths(&format!("{path}/*/*/enroll.json"))?,
        EndpointOS::Linux => glob_paths(&format!("{path}/Linux/*/enroll.json"))?,
        EndpointOS::Darwin => glob_paths(&format!("{path}/Darwin/*/enroll.json"))?,
        EndpointOS::Windows => glob_paths(&format!("{path}/Windows/*/enroll.json"))?,
    };

    Ok(count.len())
}

#[derive(Debug, Serialize)]
pub(crate) struct GlobInfo {
    pub(crate) full_path: String,
    pub(crate) filename: String,
    pub(crate) is_file: bool,
    pub(crate) is_directory: bool,
    pub(crate) is_symlink: bool,
}

/// Execute a provided Glob pattern (Ex: /files/*) and return results
pub(crate) fn glob_paths(glob_pattern: &str) -> Result<Vec<GlobInfo>, StoreError> {
    let mut info = Vec::new();
    let glob_results = glob::glob(glob_pattern);
    let paths = match glob_results {
        Ok(result) => result,
        Err(err) => {
            error!("[artemis-core] Could not glob {glob_pattern}: {err:?}");
            return Err(StoreError::BadGlob);
        }
    };

    for entry in paths.flatten() {
        let glob_info = GlobInfo {
            full_path: entry.to_str().unwrap_or_default().to_string(),
            filename: entry
                .file_name()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default()
                .to_string(),
            is_directory: entry.is_dir(),
            is_file: entry.is_file(),
            is_symlink: entry.is_symlink(),
        };
        info.push(glob_info);
    }

    Ok(info)
}

#[cfg(test)]
mod tests {
    use common::server::EndpointOS;
    use std::path::PathBuf;

    use super::{create_endpoint_path, create_enroll_file};
    use crate::{
        artifacts::{enrollment::EndpointInfo, systeminfo::Memory},
        filestore::endpoints::{endpoint_count, glob_paths},
        utils::{config::read_config, filesystem::create_dirs},
    };

    #[tokio::test]
    async fn test_create_endpoint_path() {
        let path = "./tmp";
        let data = EndpointInfo {
            boot_time: 1111,
            hostname: String::from("hello"),
            os_version: String::from("12.1"),
            uptime: 100,
            kernel_version: String::from("12.11"),
            platform: String::from("linux"),
            cpu: Vec::new(),
            disks: Vec::new(),
            memory: Memory {
                available_memory: 111,
                free_memory: 111,
                free_swap: 111,
                total_memory: 111,
                total_swap: 111,
                used_memory: 111,
                used_swap: 111,
            },
        };

        let result = create_endpoint_path(path, &data).await.unwrap();
        assert!(!result.is_empty());
    }

    #[tokio::test]
    async fn test_create_enroll_file() {
        create_dirs("./tmp").await.unwrap();
        let test = "./tmp/test.json";
        create_enroll_file(&test, b"hello").await.unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "WriteFile")]
    async fn test_create_enroll_file_bad() {
        let test = ".";
        create_enroll_file(&test, b"hello").await.unwrap();
    }

    #[test]
    fn test_glob_paths() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests");

        let _result = glob_paths(&format!("{}/*", test_location.to_str().unwrap())).unwrap();
    }

    #[tokio::test]
    async fn test_endpoint_count() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/server.toml");
        create_dirs("./tmp").await.unwrap();

        let config = read_config(&test_location.display().to_string())
            .await
            .unwrap();

        let _ = endpoint_count(&config.endpoint_server.storage, &EndpointOS::All)
            .await
            .unwrap();
    }
}
