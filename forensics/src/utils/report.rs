use crate::{
    artifacts::os::systeminfo::info::get_info,
    filesystem::files::hash_file_data,
    structs::toml::{Artifacts, Output},
    utils::{
        error::ArtemisError,
        output::final_output,
        time::{time_now, unixepoch_to_iso},
    },
};
use common::files::Hashes;
use log::error;
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Serialize)]
pub(crate) struct ReportRuns {
    pub(crate) name: String,
    pub(crate) hash: String,
    pub(crate) last_run: String,
    pub(crate) unixepoch: u64,
    pub(crate) output_count: u64,
    pub(crate) log_file: String,
    pub(crate) status: String,
}

/// Create a collection report
pub(crate) fn generate_report(
    output: &mut Output,
    artifacts: &[String],
    start: u64,
    runs: &[ReportRuns],
) {
    let info = get_info();

    let mut value = match serde_json::to_value(&info) {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Could not serialize info for report: {err:?}");
            return;
        }
    };
    value["collection_id"] = output.collection_id.into();
    value["endpoint_id"] = output.endpoint_id.clone().into();
    value["start_time"] = unixepoch_to_iso(start as i64).into();
    value["end_time"] = unixepoch_to_iso(time_now() as i64).into();
    value["artifacts"] = json!(artifacts);
    let value_runs = match serde_json::to_value(runs) {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Could not serialize runs for report: {err:?}");
            return;
        }
    };
    value["artifact_runs"] = value_runs;

    let bytes = match serde_json::to_vec(&value) {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Could not serialize report to bytes: {err:?}");
            return;
        }
    };

    if let Err(err) = final_output(&bytes, output, "report") {
        error!("[forensics] Could not output report: {err:?}");
    }
}

/// Create an artifact artifact report
pub(crate) fn generate_artifact_report(
    artifacts: &Artifacts,
    output: &Output,
    status: &str,
) -> Result<ReportRuns, ArtemisError> {
    let artifact_bytes = match serde_json::to_vec(artifacts) {
        Ok(result) => result,
        Err(err) => {
            error!(
                "[forensics] Failed to serialize artifact report for {}: {err:?}",
                artifacts.artifact_name
            );
            return Err(ArtemisError::BadToml);
        }
    };

    let hashes = Hashes {
        md5: true,
        sha1: false,
        sha256: false,
    };
    let (md5, _, _) = hash_file_data(&hashes, &artifact_bytes);
    let time_now = time_now();
    let report = ReportRuns {
        name: artifacts.artifact_name.clone(),
        hash: md5,
        last_run: unixepoch_to_iso(time_now as i64),
        unixepoch: time_now,
        output_count: output.output_count,
        log_file: output.log_file.clone(),
        status: status.to_string(),
    };

    Ok(report)
}

#[cfg(test)]
mod tests {
    use crate::{
        structs::{
            artifacts::os::processes::ProcessOptions,
            toml::{Artifacts, Output},
        },
        utils::report::{generate_artifact_report, generate_report},
    };

    #[test]
    fn test_generate_artifact_report() {
        let out = Output {
            name: String::from("reporting"),
            directory: String::from("tmp"),
            format: String::from("json"),
            endpoint_id: String::from("abcd"),
            output: String::from("local"),
            ..Default::default()
        };

        let art = Artifacts {
            artifact_name: String::from("processes"),
            processes: Some(ProcessOptions {
                md5: true,
                sha1: true,
                sha256: false,
                metadata: true,
            }),
            ..Default::default()
        };

        let report = generate_artifact_report(&art, &out, "completed").unwrap();
        assert_eq!(report.hash, "0bbf67efb22492c6a648d299f26bc3a9");
    }

    #[test]
    fn test_generate_report() {
        let mut out = Output {
            name: String::from("reporting"),
            directory: String::from("tmp"),
            format: String::from("json"),
            endpoint_id: String::from("abcd"),
            output: String::from("local"),
            ..Default::default()
        };

        let art = Artifacts {
            artifact_name: String::from("processes"),
            processes: Some(ProcessOptions {
                md5: true,
                sha1: true,
                sha256: false,
                metadata: true,
            }),
            ..Default::default()
        };

        let report = generate_artifact_report(&art, &out, "completed").unwrap();
        generate_report(&mut out, &vec![String::from("processes")], 0, &vec![report]);
    }
}
