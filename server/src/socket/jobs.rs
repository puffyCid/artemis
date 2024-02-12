use crate::filestore::jobs::cache_job_results;
use common::server::{JobInfo, JobMetadata, JobType};
use log::error;
use serde_json::{Error, Value};

/// Parse Quick Jobs such as process listing
pub(crate) async fn parse_job(data: &str, ip: &str, path: &str) -> Result<(), Error> {
    let job_result: Result<Value, Error> = serde_json::from_str(data);
    let job = match job_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Failed to deserialize job from {ip}: {err:?}");
            return Err(err);
        }
    };

    let meta_opt = job.get("metadata");
    if meta_opt.is_none() {
        error!("[server] No job metadata from {ip}");
        return Ok(());
    }

    let meta_result: Result<JobMetadata, Error> = serde_json::from_value(meta_opt.unwrap().clone());
    let meta = match meta_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Failed to deserialize job metadata from {ip}: {err:?}");
            return Err(err);
        }
    };

    let job_type = get_job_type(&meta.artifact_name).await;
    let job_opt = job.get("job");
    if job_opt.is_none() {
        error!("[server] No job info from {ip}");
        return Ok(());
    }
    let job_info_result: Result<JobInfo, Error> = serde_json::from_value(job_opt.unwrap().clone());
    let job_info = match job_info_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Failed to deserialize job info from {ip}: {err:?}");
            return Err(err);
        }
    };

    let endpoint_path = format!("{path}/{}/{}", meta.platform, meta.endpoint_id);

    let store_result =
        cache_job_results(data.as_bytes(), &job_type, &job_info, &endpoint_path).await;
    if store_result.is_err() {
        error!(
            "[server] Failed to cache job result {:?}",
            store_result.unwrap_err()
        );
    }
    Ok(())
}

/// Get the `JobType` from the websocket data
async fn get_job_type(job_type: &str) -> JobType {
    match job_type {
        "processes" => JobType::Processes,
        "filelist" => JobType::Filelist,
        "script" => JobType::Script,
        _ => JobType::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::get_job_type;
    use crate::filestore::jobs::save_job;
    use crate::socket::jobs::parse_job;
    use crate::utils::filesystem::create_dirs;
    use common::server::{Action, JobInfo, JobType, Status};

    #[tokio::test]
    async fn test_get_job_type() {
        let result = get_job_type("processes").await;
        assert_eq!(result, JobType::Processes);
    }

    #[tokio::test]
    async fn test_parse_job() {
        let test = r#"{"metadata":{"endpoint_id":"8926245d-6ffc-44ff-b446-b5467334a786","uuid":"b4056e0e-1dc0-4eeb-b8b0-c599808793f0","id":1,"artifact_name":"processes","complete_time":1707630990,"start_time":1707630990,"hostname":"aDev-MacBook-Pro.local","os_version":"14.0","platform":"Darwin","kernel_version":"23.0.0","load_performance":{"avg_one_min":2.66357421875,"avg_five_min":5.2529296875,"avg_fifteen_min":4.3095703125}},"job":{"id":1,"name":"processes","created":10000,"started":10001,"finished":20000,"status":"Finished","collection":"adssafasdfsadfs==","duration":10,"start_time":100,"action":"Stop","job_type":"Processes"},"data":[{"full_path":"/System/Library/Frameworks/ApplicationServices.framework/Versions/A/Frameworks/ATS.framework/Versions/A/Support/fontworker","name":"fontworker","path":"","pid":478,"ppid":1,"environment":"","status":"Runnable","arguments":"","memory_usage":11976704,"virtual_memory_usage":34871906304,"start_time":1707627043,"uid":"501","gid":"20","md5":"","sha1":"","sha256":"","binary_info":[]}]}"#;
        create_dirs("./tmp/Darwin/8926245d-6ffc-44ff-b446-b5467334a786")
            .await
            .unwrap();
        let path = "./tmp/Darwin/8926245d-6ffc-44ff-b446-b5467334a786";
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

        save_job(data, &path).await.unwrap();
        parse_job(test, "127.0.0.1", "./tmp").await.unwrap();
    }
}
