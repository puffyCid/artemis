use super::error::StoreError;
use crate::utils::filesystem::{create_dirs, is_file, read_file, write_file};
use common::server::{JobInfo, JobType};
use log::error;
use std::collections::HashMap;

/**
 * Save `JobInfo` to endpoint `jobs.json` file.
 * Path is full path to endpoint **including** the endpoint ID
 *
 * Only `JobType::Collection` entries are written to disk. Otherwise the Job is sent directly to endpoint
 */
pub(crate) async fn save_job(mut job: JobInfo, path: &str) -> Result<(), StoreError> {
    let job_file = format!("{path}/jobs.json");
    let mut jobs = HashMap::new();

    if is_file(&job_file) {
        let jobs_result = read_file(&job_file).await;
        let job_data = match jobs_result {
            Ok(result) => result,
            Err(err) => {
                error!("[server] Failed to read jobs file for endpoint at {path}: {err:?}");
                return Err(StoreError::ReadFile);
            }
        };

        if !job_data.is_empty() {
            let jobs_result = serde_json::from_slice(&job_data);
            let existing_jobs: HashMap<u64, JobInfo> = match jobs_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[server] Could not deserialize existing jobs: {err:?}");
                    return Err(StoreError::Deserialize);
                }
            };
            jobs = existing_jobs;
        }
    }

    job.id = jobs.len() as u64 + 1;
    jobs.insert(job.id, job);

    let serde_result = serde_json::to_vec(&jobs);
    let value = match serde_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Failed to serialize job data: {err:?}");
            return Err(StoreError::Serialize);
        }
    };

    let status = write_file(&value, &job_file, false).await;
    if status.is_err() {
        error!("[server] Could not write jobs file");
        return Err(StoreError::WriteFile);
    }

    Ok(())
}

/// Return all Jobs for endpoint. Path is full path to endpoint **including** the endpoint ID
pub(crate) async fn get_jobs(path: &str) -> Result<HashMap<u64, JobInfo>, StoreError> {
    let job_file = format!("{path}/jobs.json");

    if !is_file(&job_file) {
        return Ok(HashMap::new());
    }

    let value_result = read_file(&job_file).await;
    let value = match value_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Failed to read {job_file}: {err:?}");
            return Err(StoreError::ReadFile);
        }
    };

    let serde_value = serde_json::from_slice(&value);
    let jobs: HashMap<u64, JobInfo> = match serde_value {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Failed to deserialize job data: {err:?}");
            return Err(StoreError::Deserialize);
        }
    };

    Ok(jobs)
}

/**
 * Update `JobInfo` at endpoint `jobs.json` file.
 * Path is full path to endpoint **including** the endpoint ID
 *
 * Jobs are only updated by the endpoint
 */
pub(crate) async fn update_job(job: &JobInfo, path: &str) -> Result<(), StoreError> {
    let job_file = format!("{path}/jobs.json");
    let mut jobs = HashMap::new();

    if is_file(&job_file) {
        let jobs_result = read_file(&job_file).await;
        let job_data = match jobs_result {
            Ok(result) => result,
            Err(err) => {
                error!("[server] Failed to read jobs file for endpoint at {path} for updating: {err:?}");
                return Err(StoreError::ReadFile);
            }
        };

        let jobs_result = serde_json::from_slice(&job_data);
        let existing_jobs: HashMap<u64, JobInfo> = match jobs_result {
            Ok(result) => result,
            Err(err) => {
                error!("[server] Could not deserialize existing jobs for updating: {err:?}");
                return Err(StoreError::Deserialize);
            }
        };
        jobs = existing_jobs;
    }

    jobs.insert(job.id, job.clone());

    let serde_result = serde_json::to_vec(&jobs);
    let value = match serde_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Failed to serialize job data for update: {err:?}");
            return Err(StoreError::Serialize);
        }
    };

    let status = write_file(&value, &job_file, false).await;
    if status.is_err() {
        error!("[server] Could not update jobs file");
        return Err(StoreError::WriteFile);
    }

    Ok(())
}

/// Cache Quick Job results
pub(crate) async fn cache_job_results(
    data: &[u8],
    job_type: &JobType,
    info: &JobInfo,
    endpoint_path: &str,
) -> Result<(), StoreError> {
    println!("{endpoint_path}");
    update_job(info, endpoint_path).await?;
    let job_storage = format!("{endpoint_path}/jobs");
    let storage_result = create_dirs(&job_storage).await;
    if storage_result.is_err() {
        error!(
            "[server] Could not create job storage directory: {:?}",
            storage_result.unwrap_err()
        );
        return Err(StoreError::CreateDirectory);
    }
    let result_file = format!("{job_storage}/{job_type:?}.json");
    let status = write_file(data, &result_file, false).await;
    if status.is_err() {
        error!(
            "[server] Could not write jobs file: {:?}",
            status.unwrap_err()
        );
        return Err(StoreError::WriteFile);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::filestore::jobs::{cache_job_results, get_jobs, save_job, update_job};
    use crate::utils::filesystem::create_dirs;
    use common::server::{Action, JobInfo, JobType, Status};
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_save_job() {
        create_dirs("./tmp/save").await.unwrap();
        let path = "./tmp/save";
        let data = JobInfo {
            id: 0,
            name: String::from("randomjob"),
            created: 10,
            started: 0,
            finished: 0,
            status: Status::NotStarted,
            duration: 0,
            start_time: 0,
            action: Action::Start,
            job_type: JobType::Collection,
            collection: String::from("c3lzdGVtID0gIndpbmRvd3MiCgpbb3V0cHV0XQpuYW1lID0gInByZWZldGNoX2NvbGxlY3Rpb24iCmRpcmVjdG9yeSA9ICIuL3RtcCIKZm9ybWF0ID0gImpzb24iCmNvbXByZXNzID0gZmFsc2UKZW5kcG9pbnRfaWQgPSAiNmM1MWIxMjMtMTUyMi00NTcyLTlmMmEtMGJkNWFiZDgxYjgyIgpjb2xsZWN0aW9uX2lkID0gMQpvdXRwdXQgPSAibG9jYWwiCgpbW2FydGlmYWN0c11dCmFydGlmYWN0X25hbWUgPSAicHJlZmV0Y2giClthcnRpZmFjdHMucHJlZmV0Y2hdCmFsdF9kcml2ZSA9ICdDJwo="),
        };

        save_job(data, &path).await.unwrap();
    }

    #[tokio::test]
    async fn test_get_jobs() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/3482136c-3176-4272-9bd7-b79f025307d6");

        let result = get_jobs(&test_location.display().to_string())
            .await
            .unwrap();

        assert_eq!(result.get(&1).unwrap().id, 1);
        assert_eq!(result.get(&1).unwrap().name, "randomjob");
        assert_eq!(result.get(&1).unwrap().created, 10);
        assert_eq!(result.get(&1).unwrap().started, 0);
        assert_eq!(result.get(&1).unwrap().finished, 0);
        assert_eq!(result.get(&1).unwrap().status, Status::NotStarted);
        assert_eq!(result.get(&1).unwrap().collection.len(), 372);
    }

    #[tokio::test]
    async fn test_update_job() {
        create_dirs("./tmp").await.unwrap();
        let path = "./tmp";

        let mut data = JobInfo {
            id: 0,
            name: String::from("randomjob"),
            created: 10,
            started: 0,
            finished: 0,
            status: Status::NotStarted,
            duration: 0,
            start_time: 0,
            action: Action::Start,
            job_type: JobType::Collection,
            collection: String::from("c3lzdGVtID0gIndpbmRvd3MiCgpbb3V0cHV0XQpuYW1lID0gInByZWZldGNoX2NvbGxlY3Rpb24iCmRpcmVjdG9yeSA9ICIuL3RtcCIKZm9ybWF0ID0gImpzb24iCmNvbXByZXNzID0gZmFsc2UKZW5kcG9pbnRfaWQgPSAiNmM1MWIxMjMtMTUyMi00NTcyLTlmMmEtMGJkNWFiZDgxYjgyIgpjb2xsZWN0aW9uX2lkID0gMQpvdXRwdXQgPSAibG9jYWwiCgpbW2FydGlmYWN0c11dCmFydGlmYWN0X25hbWUgPSAicHJlZmV0Y2giClthcnRpZmFjdHMucHJlZmV0Y2hdCmFsdF9kcml2ZSA9ICdDJwo="),
        };

        save_job(data.clone(), path).await.unwrap();
        data.status = Status::Finished;
        update_job(&data, path).await.unwrap();
    }

    #[tokio::test]
    async fn test_cache_job_results() {
        create_dirs("./tmp/save").await.unwrap();
        let path = "./tmp/save";
        let data = JobInfo {
            id: 0,
            name: String::from("randomjob"),
            created: 10,
            started: 0,
            finished: 0,
            status: Status::NotStarted,
            duration: 0,
            start_time: 0,
            action: Action::Start,
            job_type: JobType::Processes,
            collection: String::from("c3lzdGVtID0gIndpbmRvd3MiCgpbb3V0cHV0XQpuYW1lID0gInByZWZldGNoX2NvbGxlY3Rpb24iCmRpcmVjdG9yeSA9ICIuL3RtcCIKZm9ybWF0ID0gImpzb24iCmNvbXByZXNzID0gZmFsc2UKZW5kcG9pbnRfaWQgPSAiNmM1MWIxMjMtMTUyMi00NTcyLTlmMmEtMGJkNWFiZDgxYjgyIgpjb2xsZWN0aW9uX2lkID0gMQpvdXRwdXQgPSAibG9jYWwiCgpbW2FydGlmYWN0c11dCmFydGlmYWN0X25hbWUgPSAicHJlZmV0Y2giClthcnRpZmFjdHMucHJlZmV0Y2hdCmFsdF9kcml2ZSA9ICdDJwo="),
        };

        save_job(data.clone(), &path).await.unwrap();
        cache_job_results(&[], &JobType::Processes, &data, &path)
            .await
            .unwrap();
    }
}
