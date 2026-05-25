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
use serde_json::Value;

/// Report metadata for a single artifact run
///
/// `ArtifactRunReport` contains metadata associated with each
/// completed artifact run
#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct ArtifactRunReport {
    /// Artifact name
    pub(crate) name: String,
    /// Hash of the artifact run options
    pub(crate) artifact_options_hash: String,
    pub(crate) artifact_options: Value,
    /// Timestamp when the artifact run completed
    pub(crate) last_run: String,
    /// Unix epoch when the artifact run completed
    pub(crate) last_run_epoch: u64,
    /// Number of output files produced by this artifact
    pub(crate) output_count: usize,
    /// Total number of records from this artifact run
    pub(crate) record_count: usize,
    /// Output files created from this artifact run
    pub(crate) output_files: Vec<String>,
    /// Artifact run status: `completed` or `failed`
    pub(crate) status: String,
}

impl ArtifactRunReport {
    /// Create a new artifact run report from execution runtime
    pub(crate) fn new<T: Serialize>(
        name: &str,
        artifact_options: &T,
        output_files: Vec<String>,
        record_count: usize,
        status: &str,
    ) -> Self {
        let last_run_epoch = time_now();
        let output_count = output_files.len();
        let options = serde_json::to_value(artifact_options).unwrap_or_default();
        Self {
            name: name.to_string(),
            artifact_options_hash: hash_artifact_options(artifact_options).unwrap_or_default(),
            artifact_options: options,
            last_run: unixepoch_to_iso(last_run_epoch as i64),
            last_run_epoch,
            output_count,
            output_files,
            record_count,
            status: status.to_string(),
        }
    }

    /// Track each file created from an artifact collection and update the `output_count` and `output_files`
    pub(crate) fn add_output_file(&mut self, output_file: String, record_count: usize) {
        self.output_files.push(output_file);
        self.output_count = self.output_files.len();
        self.record_count += record_count;

        let last_run_epoch = time_now();
        self.last_run = unixepoch_to_iso(last_run_epoch as i64);
        self.last_run_epoch = last_run_epoch;
    }
}

/// Report metadata for the entire Artemis collection
///
/// `CollectionReport` contains metadata associated with the full
/// execution runtime of Artemis
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct CollectionReport {
    /// Collection ID for the Artemis execution
    pub(crate) collection_id: u64,
    /// Endpoint ID for the target system
    pub(crate) endpoint_id: String,
    /// Start time for the Artemis execution
    pub(crate) start_time: String,
    /// When the Artemis execution completed
    pub(crate) end_time: String,
    /// Total number of files created from the Artemis collection
    pub(crate) total_output_files: usize,
    /// Artifacts collected from the Artemis collection
    pub(crate) artifacts: Vec<String>,
    /// Log file associated with the Artemis collection
    pub(crate) log_file: String,
    /// Format of the output files
    pub(crate) output_format: OutputFormat,
    /// Destination where the output files were written
    pub(crate) destination: OutputDestination,
    #[serde(flatten)]
    /// Detailed metadata associated with the target system
    pub(crate) system: SystemInfo,
    /// Run reports for each artifact collected
    pub(crate) artifact_runs: Vec<ArtifactRunReport>,
}

impl CollectionReport {
    /// Create a new collection report from completed Artemis run
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

/// MD5 hash serialized artifact options
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

#[cfg(test)]
mod tests {
    use crate::output2::{
        config::{OutputConfig, OutputFormat},
        context::CollectionContext,
        report::{ArtifactRunReport, CollectionReport, hash_artifact_options},
    };
    use serde_json::Value;
    use std::path::PathBuf;

    #[test]
    fn test_collection_report() {
        let config = OutputConfig::default();
        let context = CollectionContext::new(&config, PathBuf::from("./tmp/file.log"));
        let result = CollectionReport::new(&config, &context, Vec::new(), Vec::new());
        assert_eq!(result.output_format, OutputFormat::Jsonl);
        assert!(!result.system.artemis_version.is_empty())
    }

    #[test]
    fn test_artifact_run_report() {
        let result = ArtifactRunReport::new("test", &String::new(), Vec::new(), 10, "compleed");
        assert!(!result.last_run.is_empty());
        assert_eq!(result.output_count, 0);
    }

    #[test]
    fn test_hash_artifact_options() {
        let result = hash_artifact_options(&Value::String("test".into())).unwrap();
        assert_eq!(result, "303b5c8988601647873b4ffd247d83cb");
    }
}
