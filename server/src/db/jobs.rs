use super::{
    error::DbError,
    tables::{check_write, lookup_table_data, write_table_data},
};
use crate::artifacts::jobs::JobInfo;
use log::error;
use redb::Database;

/// Add a collection Job into the `JobDB`
pub(crate) fn add_job(id: &str, job: JobInfo, db: &Database) -> Result<(), DbError> {
    // Before writing data. Always check and get existing data first. So we dont overwrite any jobs. `write_data` locks the db
    let (write_data, existing_data) = check_write(db, id, "jobs")?;

    let mut jobs = Vec::new();
    if !existing_data.is_empty() {
        let jobs_result = serde_json::from_slice(&existing_data);
        let mut old_jobs: Vec<JobInfo> = match jobs_result {
            Ok(result) => result,
            Err(err) => {
                error!("[server] Could not deserialize old jobs: {err:?}");
                return Err(DbError::Deserialize);
            }
        };

        jobs.append(&mut old_jobs);
    }

    jobs.push(job);

    let serde_result = serde_json::to_vec(&jobs);
    let value = match serde_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Failed to serialize job data into DB table: {err:?}");
            return Err(DbError::Serialize);
        }
    };

    let result = write_table_data(write_data, id, &value, "jobs");
    if result.is_err() {
        return Err(DbError::JobDb);
    }

    Ok(())
}

/// Get all jobs associated with endpoint ID
pub(crate) fn get_jobs(id: &str, db: &Database) -> Result<Vec<JobInfo>, DbError> {
    let value = lookup_table_data("jobs", id, db)?;
    if value.is_empty() {
        return Ok(Vec::new());
    }

    let serde_value = serde_json::from_slice(&value);
    let jobs: Vec<JobInfo> = match serde_value {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Failed to deserialize endpoint data: {err:?}");
            return Err(DbError::Deserialize);
        }
    };

    Ok(jobs)
}

#[cfg(test)]
mod tests {
    use super::{add_job, get_jobs};
    use crate::{
        artifacts::jobs::{JobInfo, Status},
        db::tables::setup_db,
        utils::filesystem::create_dirs,
    };
    use std::path::PathBuf;

    #[test]
    fn test_add_job() {
        create_dirs("./tmp").unwrap();
        let path = "./tmp/jobs.redb";
        let id = "jobkey";
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

        assert_eq!(jobs[0].collection, "c3lzdGVtID0gIndpbmRvd3MiCgpbb3V0cHV0XQpuYW1lID0gInByZWZldGNoX2NvbGxlY3Rpb24iCmRpcmVjdG9yeSA9ICIuL3RtcCIKZm9ybWF0ID0gImpzb24iCmNvbXByZXNzID0gZmFsc2UKZW5kcG9pbnRfaWQgPSAiNmM1MWIxMjMtMTUyMi00NTcyLTlmMmEtMGJkNWFiZDgxYjgyIgpjb2xsZWN0aW9uX2lkID0gMQpvdXRwdXQgPSAibG9jYWwiCgpbW2FydGlmYWN0c11dCmFydGlmYWN0X25hbWUgPSAicHJlZmV0Y2giClthcnRpZmFjdHMucHJlZmV0Y2hdCmFsdF9kcml2ZSA9ICdDJwo=");
        assert_eq!(jobs[0].started, 0);
        assert_eq!(jobs[0].finished, 0);
        assert_eq!(jobs[0].status, Status::NotStarted);
        assert_eq!(jobs[0].created, 10);
        assert_eq!(jobs[0].name, "randomjob");
        assert_eq!(jobs[0].id, 0);
    }
}
