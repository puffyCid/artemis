use std::collections::HashMap;

use super::{
    error::DbError,
    tables::{check_write, lookup_table_data, write_table_data},
};
use crate::artifacts::jobs::JobInfo;
use log::{error, warn};
use redb::Database;

/// Add a collection Job into the `JobDB`
pub(crate) fn add_job(id: &str, mut job: JobInfo, db: &Database) -> Result<(), DbError> {
    // Before writing data. Always check and get existing data first. So we dont overwrite any jobs. `write_data` locks the db
    let (write_data, existing_data) = check_write(db, id, "jobs")?;

    let mut jobs = HashMap::new();
    if !existing_data.is_empty() {
        let jobs_result = serde_json::from_slice(&existing_data);
        let old_jobs: HashMap<u64, JobInfo> = match jobs_result {
            Ok(result) => result,
            Err(err) => {
                error!("[server] Could not deserialize old jobs: {err:?}");
                return Err(DbError::Deserialize);
            }
        };

        jobs = old_jobs;
    }

    // Increment Job ID counter
    job.id = jobs.len() as u64 + 1;
    jobs.insert(job.id, job);

    let serde_result = serde_json::to_string(&jobs);
    let value = match serde_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Failed to serialize job data into DB table: {err:?}");
            return Err(DbError::Serialize);
        }
    };

    let result = write_table_data(write_data, id, value.as_bytes(), "jobs");
    if result.is_err() {
        return Err(DbError::JobDb);
    }

    Ok(())
}

/// Get all jobs associated with endpoint ID
pub(crate) fn get_jobs(id: &str, db: &Database) -> Result<HashMap<u64, JobInfo>, DbError> {
    let value = lookup_table_data("jobs", id, db)?;
    if value.is_empty() {
        return Ok(HashMap::new());
    }

    let serde_value = serde_json::from_slice(&value);
    let jobs: HashMap<u64, JobInfo> = match serde_value {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Failed to deserialize endpoint data: {err:?}");
            return Err(DbError::Deserialize);
        }
    };

    Ok(jobs)
}

/// Update the Job for an endpoint
pub(crate) fn update_job(id: &str, job: JobInfo, db: &Database) -> Result<(), DbError> {
    // Before writing data. Always check and get existing data first. So we dont overwrite any jobs. `write_data` locks the db
    let (write_data, existing_data) = check_write(db, id, "jobs")?;

    let mut jobs;
    if !existing_data.is_empty() {
        let jobs_result = serde_json::from_slice(&existing_data);
        let old_jobs: HashMap<u64, JobInfo> = match jobs_result {
            Ok(result) => result,
            Err(err) => {
                error!("[server] Could not deserialize old jobs: {err:?}");
                return Err(DbError::Deserialize);
            }
        };

        jobs = old_jobs;
    } else {
        warn!(
            "[server] Could not find Job ID {} for endpoint id {id}",
            job.id
        );
        return Err(DbError::Insert);
    }

    // Insert updated Job info. This should update our hashmap
    let status = jobs.insert(job.id, job);
    if status.is_none() {
        warn!("[server] Expected old job data. Instead got none");
    }

    let serde_result = serde_json::to_string(&jobs);
    let value = match serde_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Failed to serialize job data into DB table: {err:?}");
            return Err(DbError::Serialize);
        }
    };

    let result = write_table_data(write_data, id, value.as_bytes(), "jobs");
    if result.is_err() {
        return Err(DbError::JobDb);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{add_job, get_jobs, update_job};
    use crate::{
        artifacts::jobs::{JobInfo, Status},
        db::tables::setup_db,
        utils::filesystem::create_dirs,
    };
    use std::path::PathBuf;

    #[test]
    fn test_add_job() {
        create_dirs("./tmp").unwrap();
        let path = "./tmp/jobsadding.redb";
        let id = "1cacf8ac-c98d-45cb-a69b-166338aabe9a";
        let data = JobInfo {
            id: 0,
            name: String::from("randomjob"),
            created: 10,
            started: 0,
            finished: 0,
            status: Status::NotStarted,
            collection: String::from("c3lzdGVtID0gIndpbmRvd3MiCgpbb3V0cHV0XQpuYW1lID0gInByZWZldGNoX2NvbGxlY3Rpb24iCmRpcmVjdG9yeSA9ICIuL3RtcCIKZm9ybWF0ID0gImpzb24iCmNvbXByZXNzID0gZmFsc2UKZW5kcG9pbnRfaWQgPSAiNmM1MWIxMjMtMTUyMi00NTcyLTlmMmEtMGJkNWFiZDgxYjgyIgpjb2xsZWN0aW9uX2lkID0gMQpvdXRwdXQgPSAibG9jYWwiCgpbW2FydGlmYWN0c11dCmFydGlmYWN0X25hbWUgPSAicHJlZmV0Y2giClthcnRpZmFjdHMucHJlZmV0Y2hdCmFsdF9kcml2ZSA9ICdDJwo="),
        };

        let db = setup_db(path).unwrap();

        add_job(&id, data, &db).unwrap();
        let data_up = JobInfo {
            id: 0,
            name: String::from("randomjob"),
            created: 10,
            started: 0,
            finished: 0,
            status: Status::Finished,
            collection: String::from("c3lzdGVtID0gIndpbmRvd3MiCgpbb3V0cHV0XQpuYW1lID0gInByZWZldGNoX2NvbGxlY3Rpb24iCmRpcmVjdG9yeSA9ICIuL3RtcCIKZm9ybWF0ID0gImpzb24iCmNvbXByZXNzID0gZmFsc2UKZW5kcG9pbnRfaWQgPSAiNmM1MWIxMjMtMTUyMi00NTcyLTlmMmEtMGJkNWFiZDgxYjgyIgpjb2xsZWN0aW9uX2lkID0gMQpvdXRwdXQgPSAibG9jYWwiCgpbW2FydGlmYWN0c11dCmFydGlmYWN0X25hbWUgPSAicHJlZmV0Y2giClthcnRpZmFjdHMucHJlZmV0Y2hdCmFsdF9kcml2ZSA9ICdDJwo="),
        };
        update_job(&id, data_up, &db).unwrap();
    }

    #[test]
    #[should_panic(expected = "Insert")]
    fn test_bad_update_job() {
        create_dirs("./tmp").unwrap();
        let path = "./tmp/jobsnone.redb";
        let id = "not there";
        let data = JobInfo {
            id: 0,
            name: String::from("randomjob"),
            created: 10,
            started: 0,
            finished: 0,
            status: Status::Finished,
            collection: String::from("c3lzdGVtID0gIndpbmRvd3MiCgpbb3V0cHV0XQpuYW1lID0gInByZWZldGNoX2NvbGxlY3Rpb24iCmRpcmVjdG9yeSA9ICIuL3RtcCIKZm9ybWF0ID0gImpzb24iCmNvbXByZXNzID0gZmFsc2UKZW5kcG9pbnRfaWQgPSAiNmM1MWIxMjMtMTUyMi00NTcyLTlmMmEtMGJkNWFiZDgxYjgyIgpjb2xsZWN0aW9uX2lkID0gMQpvdXRwdXQgPSAibG9jYWwiCgpbW2FydGlmYWN0c11dCmFydGlmYWN0X25hbWUgPSAicHJlZmV0Y2giClthcnRpZmFjdHMucHJlZmV0Y2hdCmFsdF9kcml2ZSA9ICdDJwo="),
        };

        let db = setup_db(path).unwrap();

        update_job(&id, data, &db).unwrap();
    }

    #[test]
    fn test_get_jobs() {
        create_dirs("./tmp").unwrap();
        let path = "./tmp/jobsnofound.redb";
        let id = "1cacf8ac-c98d-45cb-a69b-166338aabe9a";
        let db = setup_db(path).unwrap();

        let jobs = get_jobs(&id, &db).unwrap();
        assert!(jobs.is_empty());
    }

    #[test]
    fn test_get_jobs_id() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/jobs.redb");
        let path = test_location.display().to_string();

        let id = "1cacf8ac-c98d-45cb-a69b-166338aabe9a";
        let db = setup_db(&path).unwrap();

        let jobs = get_jobs(id, &db).unwrap();

        assert!(!jobs.is_empty());

        assert_eq!(jobs.get(&1).unwrap().collection, "c3lzdGVtID0gIndpbmRvd3MiCgpbb3V0cHV0XQpuYW1lID0gInByZWZldGNoX2NvbGxlY3Rpb24iCmRpcmVjdG9yeSA9ICIuL3RtcCIKZm9ybWF0ID0gImpzb24iCmNvbXByZXNzID0gZmFsc2UKZW5kcG9pbnRfaWQgPSAiNmM1MWIxMjMtMTUyMi00NTcyLTlmMmEtMGJkNWFiZDgxYjgyIgpjb2xsZWN0aW9uX2lkID0gMQpvdXRwdXQgPSAibG9jYWwiCgpbW2FydGlmYWN0c11dCmFydGlmYWN0X25hbWUgPSAicHJlZmV0Y2giClthcnRpZmFjdHMucHJlZmV0Y2hdCmFsdF9kcml2ZSA9ICdDJwo=");
        assert_eq!(jobs.get(&1).unwrap().started, 0);
        assert_eq!(jobs.get(&1).unwrap().finished, 0);
        assert_eq!(jobs.get(&1).unwrap().status, Status::NotStarted);
        assert_eq!(jobs.get(&1).unwrap().created, 10);
        assert_eq!(jobs.get(&1).unwrap().name, "randomjob");
        assert_eq!(jobs.get(&1).unwrap().id, 1);
    }
}
