use crate::{
    artifacts::os::systeminfo::info::get_info,
    filesystem::files::hash_file_data,
    output2::{
        config::{OutputConfig, OutputDestination, OutputFormat},
        context::CollectionContext,
        error::OutputResult,
    },
    utils::time::{time_now, unixepoch_to_iso},
};
use common::{files::Hashes, system::SystemInfo};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct ArtifactRunReport {
    pub(crate) name: String,
    pub(crate) artifact_options_hash: String,
    pub(crate) last_run: String,
    pub(crate) last_run_epoch: u64,
    pub(crate) output_count: usize,
    pub(crate) output_files: Vec<String>,
    pub(crate) status: String,
}

impl ArtifactRunReport {
    pub(crate) fn new(
        name: &str,
        artifact_options_hash: String,
        output_count: usize,
        output_files: Vec<String>,
        status: &str,
    ) -> Self {
        let last_run_epoch = time_now();
        Self {
            name: name.to_string(),
            artifact_options_hash,
            last_run: unixepoch_to_iso(last_run_epoch as i64),
            last_run_epoch,
            output_count,
            output_files,
            status: status.to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct CollectionReport {
    pub(crate) collection_id: u64,
    pub(crate) endpoint_id: String,
    pub(crate) start_time: String,
    pub(crate) end_time: String,
    pub(crate) total_output_files: usize,
    pub(crate) artifacts: Vec<String>,
    pub(crate) log_file: String,
    pub(crate) output_format: OutputFormat,
    pub(crate) destination: OutputDestination,
    #[serde(flatten)]
    pub(crate) system: SystemInfo,
    pub(crate) artifact_runs: Vec<ArtifactRunReport>,
}

impl CollectionReport {
    pub(crate) fn new(
        config: &OutputConfig,
        context: &CollectionContext,
        artifacts: Vec<String>,
        artifact_runs: Vec<ArtifactRunReport>,
    ) -> Self {
        let total_output_files = artifact_runs.iter().map(|run| run.output_files.len()).sum();
        Self {
            collection_id: context.collection_id,
            endpoint_id: context.endpoint_id.clone(),
            start_time: unixepoch_to_iso(context.start_time_epoch as i64),
            end_time: unixepoch_to_iso(time_now() as i64),
            total_output_files,
            artifacts,
            log_file: context.log_file.display().to_string(),
            output_format: config.format,
            destination: config.destination,
            system: get_info(),
            artifact_runs,
        }
    }
}

pub(crate) fn hash_artifact_options<T: Serialize>(options: &T) -> OutputResult<String> {
    let bytes = serde_json::to_vec(options)?;
    let hashes = Hashes {
        md5: true,
        sha1: false,
        sha256: false,
    };
    let (md5, _, _) = hash_file_data(&hashes, &bytes);
    Ok(md5)
}
