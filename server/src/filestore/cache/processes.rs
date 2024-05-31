use crate::{filestore::error::StoreError, utils::filesystem::read_file};
use common::server::jobs::ProcessJob;
use log::error;
use serde_json::Error;

/// Return process listing info
pub(crate) async fn process_list(endpoint_dir: &str) -> Result<ProcessJob, StoreError> {
    let proc_path = format!("{endpoint_dir}/jobs/Processes.json");

    let data_result = read_file(&proc_path).await;
    let data = match data_result {
        Ok(result) => result,
        Err(err) => {
            error!("server] Failed to read file {proc_path}: {err:?}");
            return Err(StoreError::ReadFile);
        }
    };

    let proc_job_result: Result<ProcessJob, Error> = serde_json::from_slice(&data);
    let proc_job = match proc_job_result {
        Ok(result) => result,
        Err(err) => {
            error!("server] Failed to deserialize process job data at {proc_path}: {err:?}");
            return Err(StoreError::Deserialize);
        }
    };

    Ok(proc_job)
}

#[cfg(test)]
mod tests {
    use crate::filestore::cache::processes::process_list;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_process_list() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/3482136c-3176-4272-9bd7-b79f025307d6");
        let results = process_list(test_location.to_str().unwrap()).await.unwrap();
        assert_eq!(results.data.len(), 1);
    }
}
