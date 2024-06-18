use super::error::SocketError;
use crate::filesystem::files::{append_file, is_file, write_file};
use common::server::jobs::{JobInfo, JobType};
use log::error;

/// Parse server job commands
pub(crate) async fn parse_server_job(
    job_command: &str,
    storage_path: &str,
) -> Result<(), SocketError> {
    let info_result = serde_json::from_str(job_command);

    let info: JobInfo = match info_result {
        Ok(result) => result,
        Err(err) => {
            error!("[client] Could not deserialize job from server: {err:?}");
            return Err(SocketError::ParseJob);
        }
    };

    match info.job_type {
        JobType::Collection => save_job(&info, storage_path).await?,
        JobType::Processes => println!("{info:?}"),
        JobType::Filelist => todo!(),
        JobType::Unknown => todo!(),
    }

    Ok(())
}

/// Save collection jobs to a file
pub(crate) async fn save_job(job: &JobInfo, storage_path: &str) -> Result<(), SocketError> {
    let job_path = format!("{storage_path}/jobs.jsonl");
    let bytes_result = serde_json::to_vec(&job);
    let mut bytes = match bytes_result {
        Ok(result) => result,
        Err(err) => {
            error!("[client] Could not serialize job to bytes: {err:?}");
            return Err(SocketError::SaveJob);
        }
    };

    bytes.push(b'\n');

    if !is_file(&job_path) {
        let status = write_file(&bytes, &job_path).await;
        if status.is_err() {
            error!(
                "[client] Could not write job to file: {:?}",
                status.unwrap_err()
            );
            return Err(SocketError::SaveJob);
        }

        return Ok(());
    }

    let _ = append_file(&bytes, &job_path).await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use common::server::jobs::{Action, JobInfo, JobType, Status};

    use crate::filesystem::directory::create_dirs;
    use crate::socket::jobs::{parse_server_job, save_job};

    #[tokio::test]
    async fn test_parse_server_job() {
        let test = "{\"id\":1,\"name\":\"processes\",\"created\":10000,\"started\":10001,\"finished\":20000,\"status\":\"NotStarted\",\"collection\":\"adssafasdfsadfs==\",\"duration\":10,\"start_time\":100,\"action\":\"Start\",\"job_type\":\"Processes\"}";
        create_dirs("./tmp").await.unwrap();
        parse_server_job(test, "./tmp/").await.unwrap();
    }

    #[tokio::test]
    async fn test_save_job() {
        create_dirs("./tmp").await.unwrap();
        let test = JobInfo {
            id: 0,
            name: String::from("test"),
            created: 0,
            started: 0,
            finished: 0,
            status: Status::NotStarted,
            collection: String::from("asdfsadfxcv"),
            start_time: 0,
            duration: 0,
            action: Action::Start,
            job_type: JobType::Collection,
        };
        save_job(&test, "./tmp/").await.unwrap();
    }
}
