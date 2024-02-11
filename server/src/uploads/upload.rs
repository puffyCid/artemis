use crate::{
    artifacts::jobs::JobInfo,
    filestore::jobs::update_job,
    server::ServerState,
    utils::{
        filesystem::{create_dirs, write_file},
        uuid::generate_uuid,
    },
};
use axum::{
    extract::{Multipart, State},
    http::StatusCode,
};
use log::{error, warn};

/// Process uploaded data
pub(crate) async fn upload_collection(
    State(state): State<ServerState>,
    mut multipart: Multipart,
) -> Result<(), StatusCode> {
    let mut endpoint_id = String::new();
    let path = state.config.endpoint_server.storage;

    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap_or_default().to_string();

        if name == "endpoint-id" {
            endpoint_id = field.text().await.unwrap_or_default();
        } else if name == "job-info" {
            let data = field.text().await.unwrap_or_default();
            let endpoint_path = format!("{path}/{endpoint_id}");
            update_job_file(&endpoint_path, &data).await?;
        } else if name == "collection" {
            let filename_option = field.file_name();
            let filename = if let Some(result) = filename_option {
                result.to_string()
            } else {
                warn!("[server] Filename not provided in upload. Generated a random one!");
                format!("{}.jsonl.gz", generate_uuid())
            };

            let data = field.bytes().await.unwrap_or_default();
            let endpoint_dir = format!("{path}/{endpoint_id}");
            write_collection(&endpoint_dir, &filename, &data).await?;
        }
    }
    Ok(())
}

/// Update the Job DB using the uploaded job-info data
async fn update_job_file(path: &str, data: &str) -> Result<(), StatusCode> {
    if path.is_empty() {
        error!("[server] No endpoint path provided cannot update jobs.json");
        return Err(StatusCode::BAD_REQUEST);
    }

    let job_result = serde_json::from_str(data);
    let job: JobInfo = match job_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Cannot deserialize Job Info for Endpoint ID {path}: {err:?}");
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    let status = update_job(&job, path).await;
    if status.is_err() {
        error!(
            "[server] Could not update Job for {path}: {:?}",
            status.unwrap_err()
        );
        return Err(StatusCode::BAD_REQUEST);
    }

    Ok(())
}

/// Write data to endpoint storage directory
async fn write_collection(
    endpoint_dir: &str,
    filename: &str,
    data: &[u8],
) -> Result<(), StatusCode> {
    // Endpoint storage directory should have been created upon enrollment. But check in case
    let collections = format!("{endpoint_dir}/collections");
    let status = create_dirs(&collections).await;
    if status.is_err() {
        error!(
            "[server] Could not create {collections} storage directory: {:?}",
            status.unwrap_err()
        );
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Only decompress data smaller than 2GB
    let max_size = 2147483648;
    if data.len() < max_size {
        let decom_name = filename.trim_end_matches(".gz");
        let endpoint_path = format!("{collections}/{decom_name}");
        // Write the data to endpoint directory,  but decompress first
        let status = write_file(data, &endpoint_path, true).await;
        if status.is_err() {
            error!(
                "[server] Could not write data to {endpoint_path} storage directory: {:?}",
                status.unwrap_err()
            );
        } else {
            return Ok(());
        }

        warn!("[server] Could not decompress and write data to {collections}. Trying compressed data!");
    }

    let endpoint_path = format!("{collections}/{filename}");

    // Write the compressed data to endpoint directory
    let status = write_file(data, &endpoint_path, false).await;
    if status.is_err() {
        error!(
            "[server] Could not write data to {endpoint_path} storage directory: {:?}",
            status.unwrap_err()
        );
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::artifacts::jobs::{Action, JobInfo, JobType, Status};
    use crate::uploads::upload::write_collection;
    use crate::utils::filesystem::{create_dirs, write_file};
    use crate::{
        uploads::upload::update_job_file,
        utils::{config::read_config, uuid::generate_uuid},
    };
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_update_job_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/server.toml");
        create_dirs("./tmp/uploads").await.unwrap();

        let mut value = JobInfo {
            id: 1,
            collection: String::from("asdfasdfasdfasd=="),
            created: 1000,
            started: 10001,
            finished: 20000,
            name: String::from("processes"),
            status: Status::NotStarted,
            duration: 0,
            start_time: 0,
            action: Action::Start,
            job_type: JobType::Collection,
        };

        let mut jobs = HashMap::new();
        jobs.insert(1, value.clone());

        write_file(
            &serde_json::to_vec(&jobs).unwrap(),
            "./tmp/uploads/jobs.json",
            false,
        )
        .await
        .unwrap();

        value.status = Status::Failed;

        update_job_file("./tmp/uploads", &serde_json::to_string(&value).unwrap())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_write_collction() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/server.toml");

        let config = read_config(&test_location.display().to_string())
            .await
            .unwrap();
        let endpoint_id = generate_uuid();

        let path = format!("{}/{endpoint_id}", config.endpoint_server.storage);
        let filename = "test.jsonl.gz";
        let data = [
            31, 139, 8, 0, 89, 135, 7, 101, 0, 255, 5, 128, 177, 9, 0, 32, 16, 3, 87, 209, 27, 195,
            121, 20, 44, 2, 129, 111, 190, 16, 119, 15, 143, 123, 36, 179, 6, 237, 210, 158, 252,
            0, 132, 255, 53, 22, 19, 0, 0, 0,
        ];

        write_collection(&path, filename, &data).await.unwrap();
    }
}
