use crate::{
    artifacts::enrollment::{EndpointEnrollment, EndpointInfo},
    filestore::error::StoreError,
    utils::{
        filesystem::{create_dirs, read_file, read_lines, write_file},
        time::time_now,
        uuid::generate_uuid,
    },
};
use common::server::{EndpointList, EndpointOS, EndpointRequest, Heartbeat};
use log::error;
use serde::Serialize;

/// Create the endpoint storage directory and generate an ID
pub(crate) async fn create_endpoint_path(
    path: &str,
    endpoint: &EndpointInfo,
) -> Result<String, StoreError> {
    let id = generate_uuid();

    let data = EndpointEnrollment {
        hostname: endpoint.hostname.clone(),
        platform: endpoint.platform.clone(),
        tags: Vec::new(),
        notes: Vec::new(),
        checkin: time_now(),
        id,
        boot_time: endpoint.boot_time,
        os_version: endpoint.os_version.clone(),
        uptime: endpoint.uptime,
        kernel_version: endpoint.kernel_version.clone(),
        cpu: endpoint.cpu.clone(),
        disks: endpoint.disks.clone(),
        memory: endpoint.memory.clone(),
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

/// Get the most recent heartbeat for endpoint
pub(crate) async fn recent_heartbeat(endpoint_dir: &str) -> Result<Heartbeat, StoreError> {
    let enroll_path = format!("{endpoint_dir}/enroll.json");
    let enroll = read_enroll(&enroll_path).await?;

    let enroll_beat = Heartbeat {
        endpoint_id: enroll.id,
        heartbeat: false,
        jobs_running: 0,
        hostname: enroll.hostname,
        timestamp: enroll.checkin,
        cpu: enroll.cpu,
        disks: enroll.disks,
        memory: enroll.memory,
        boot_time: enroll.boot_time,
        os_version: enroll.os_version,
        uptime: enroll.uptime,
        kernel_version: enroll.kernel_version,
        platform: enroll.platform,
    };

    let path = format!("{endpoint_dir}/heartbeat.jsonl");
    let beat_lines = read_lines(&path).await;
    let mut lines = match beat_lines {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Could not read heartbeat {path}: {err:?}. Returning enrollment data");
            return Ok(enroll_beat);
        }
    };

    let mut heartbeat = String::new();

    while let Ok(line) = lines.next_line().await {
        match line {
            Some(result) => {
                if result.is_empty() {
                    break;
                }
                heartbeat = result;
            }
            None => break,
        }
    }

    if heartbeat.is_empty() {
        return Ok(enroll_beat);
    }

    let beat_result = serde_json::from_str(&heartbeat);
    let beat: Heartbeat = match beat_result {
        Ok(result) => result,
        Err(err) => {
            error!(
                "[server] Could not serialize heartbeat {path}: {err:?}. Returning enrollment data"
            );
            return Ok(enroll_beat);
        }
    };

    Ok(beat)
}

/// Get a list of enrolled endpoints
pub(crate) async fn get_endpoints(
    glob_pattern: &str,
    request: &EndpointRequest,
) -> Result<Vec<EndpointList>, StoreError> {
    let mut globs = glob_paths(glob_pattern)?;
    globs.sort();

    let limit = 50;
    let mut endpoint_entries = Vec::new();
    let mut paged_found = false;

    for entry in globs {
        let enroll_path = entry.full_path;
        let (filter_match, info) = enroll_filter(&enroll_path, request).await?;

        if request.pagination.is_empty() && filter_match {
            endpoint_entries.push(info.clone());
        } else if enroll_path.contains(&request.pagination) {
            paged_found = true;
            continue;
        }

        if paged_found && filter_match {
            endpoint_entries.push(info);
        }

        if endpoint_entries.len() == limit {
            break;
        }
    }

    Ok(endpoint_entries)
}

/// Filter enrollment data based on request
async fn enroll_filter(
    path: &str,
    request: &EndpointRequest,
) -> Result<(bool, EndpointList), StoreError> {
    let enroll = read_enroll(path).await?;
    let entry = EndpointList {
        os: enroll.platform,
        version: enroll.os_version,
        id: enroll.id,
        hostname: enroll.hostname,
        last_pulse: 0,
    };
    let mut filter_match = false;

    // If no filters. Just return the entry
    if request.search.is_empty() && request.tags.is_empty() {
        filter_match = true;
        return Ok((filter_match, entry));
    }

    if !request.search.is_empty()
        && (entry.hostname.contains(&request.search) || !entry.id.contains(&request.search))
    {
        filter_match = true;
        return Ok((filter_match, entry));
    }

    for tag in &request.tags {
        if enroll.tags.contains(tag) {
            filter_match = true;
            return Ok((filter_match, entry));
        }
    }

    Ok((filter_match, entry))
}

/// Read the `enroll.json` file
pub(crate) async fn read_enroll(path: &str) -> Result<EndpointEnrollment, StoreError> {
    let enroll_result = read_file(path).await;
    let enroll_data = match enroll_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Could not read original enroll file: {path}: {err:?}");
            return Err(StoreError::ReadFile);
        }
    };

    let enroll_beat_result = serde_json::from_slice(&enroll_data);
    let enroll: EndpointEnrollment = match enroll_beat_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Could not serialize enroll file: {path}: {err:?}");
            return Err(StoreError::ReadFile);
        }
    };

    Ok(enroll)
}

#[derive(Debug, Serialize, Ord, PartialEq, Eq, PartialOrd)]
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
    use super::{create_endpoint_path, create_enroll_file};
    use crate::{
        artifacts::enrollment::EndpointInfo,
        filestore::endpoints::{
            endpoint_count, enroll_filter, get_endpoints, glob_paths, read_enroll, recent_heartbeat,
        },
        utils::{config::read_config, filesystem::create_dirs},
    };
    use common::{
        server::{EndpointOS, EndpointRequest},
        system::Memory,
    };
    use std::path::PathBuf;

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

    #[tokio::test]
    async fn test_recent_heartbeat() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/3482136c-3176-4272-9bd7-b79f025307d6");

        let results = recent_heartbeat(&test_location.to_str().unwrap())
            .await
            .unwrap();

        assert_eq!(results.boot_time, 0);
    }

    #[tokio::test]
    async fn test_enroll_filter() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/3482136c-3176-4272-9bd7-b79f025307d6/enroll.json");

        let request = EndpointRequest {
            pagination: String::new(),
            filter: EndpointOS::All,
            tags: Vec::new(),
            search: String::new(),
        };

        let path = test_location.to_str().unwrap();

        let (filer_match, result) = enroll_filter(path, &request).await.unwrap();
        assert!(filer_match);
        assert_eq!(result.hostname, "hello");
    }

    #[tokio::test]
    async fn test_read_enroll() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/3482136c-3176-4272-9bd7-b79f025307d6/enroll.json");

        let path = test_location.to_str().unwrap();

        let result = read_enroll(path).await.unwrap();
        assert_eq!(result.hostname, "hello");
    }

    #[tokio::test]
    async fn test_get_endpoints_paged() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/*/enroll.json");

        let request = EndpointRequest {
            pagination: String::from("3482136c-3176-4272-9bd7-b79f025307d6"),
            filter: EndpointOS::All,
            tags: Vec::new(),
            search: String::new(),
        };

        let pattern = test_location.to_str().unwrap();

        let results = get_endpoints(pattern, &request).await.unwrap();
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_get_endpoints() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/*/enroll.json");

        let request = EndpointRequest {
            pagination: String::new(),
            filter: EndpointOS::All,
            tags: Vec::new(),
            search: String::new(),
        };

        let pattern = test_location.to_str().unwrap();

        let results = get_endpoints(pattern, &request).await.unwrap();
        assert_eq!(results[0].os, "linux");
    }
}
