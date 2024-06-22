use crate::{
    filestore::error::StoreError,
    utils::filesystem::{read_file, write_file},
};
use common::system::Processes;
use log::error;
use serde_json::{Error, Value};

/// Return process listing info
pub(crate) async fn process_list(endpoint_dir: &str) -> Result<Vec<Processes>, StoreError> {
    let proc_path = format!("{endpoint_dir}/qc/processes.json");

    let data_result = read_file(&proc_path).await;
    let data = match data_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Failed to read file {proc_path}: {err:?}");
            return Err(StoreError::ReadFile);
        }
    };

    let proc_job_result: Result<Vec<Processes>, Error> = serde_json::from_slice(&data);
    let proc_job = match proc_job_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Failed to deserialize process data at {proc_path}: {err:?}");
            return Err(StoreError::Deserialize);
        }
    };

    Ok(proc_job)
}

/// Save process quick collections to disk
pub(crate) async fn save_processes(procs: &[Value], endpoint_dir: &str) {
    let proc_path = format!("{endpoint_dir}/qc/processes.json");

    let data_result = write_file(
        &serde_json::to_vec(procs).unwrap_or_default(),
        &proc_path,
        false,
    )
    .await;

    if data_result.is_err() {
        error!(
            "[server] Failed to write file {proc_path}: {:?}",
            data_result.unwrap_err()
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        filestore::cache::processes::{process_list, save_processes},
        utils::filesystem::create_dirs,
    };
    use common::system::Processes;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_process_list() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/3482136c-3176-4272-9bd7-b79f025307d6");
        let results = process_list(test_location.to_str().unwrap()).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_save_processes() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("./tmp");
        create_dirs("./tmp/qc").await.unwrap();

        let procs = Processes {
            full_path: String::from("test/test"),
            name: String::from("test"),
            path: String::from("test"),
            pid: 0,
            ppid: 10,
            environment: String::from("test"),
            status: String::from("test"),
            arguments: String::from("test"),
            memory_usage: 10,
            virtual_memory_usage: 10,
            start_time: String::from("2020-12-03"),
            uid: String::from("501"),
            gid: String::from("201"),
            md5: String::new(),
            sha1: String::new(),
            sha256: String::new(),
            binary_info: Vec::new(),
        };

        save_processes(
            &vec![serde_json::to_value(procs).unwrap()],
            test_location.to_str().unwrap(),
        )
        .await;
    }
}
