use crate::{
    artifacts::os::systeminfo::info::get_info_metadata,
    structs::toml::OutputConfig,
    utils::{
        time::{time_now, unixepoch_to_iso},
        uuid::generate_uuid,
    },
};
use common::system::SystemInfoMetadata;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Context shared across an entire Artemis collection
///
/// `CollectionContext` has collection metadata
/// for the entire Artemis execution
#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct CollectionContext {
    /// Collection ID for the Artemis execution
    pub(crate) collection_id: u64,
    /// Endpoint ID for the target system
    pub(crate) endpoint_id: String,
    /// Name of the collection
    pub(crate) collection_name: String,
    /// Start time for the Artemis execution
    pub(crate) start_time: String,
    /// Unix epoch start time for the Artemis execution
    pub(crate) start_time_epoch: u64,
    /// Log file associated with the Artemis collection
    pub(crate) log_file: PathBuf,
    /// Metadata associated with the target system
    pub(crate) system: SystemInfoMetadata,
}

/// Context for each artifact run
///
/// `ArtifactContext` has collection metadata for artifact run
#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct ArtifactContext {
    /// Artifact name that was collected
    pub(crate) artifact_name: String,
    /// Endpoint ID for the target system
    pub(crate) endpoint_id: String,
    /// UUID shared for entire artifact run
    pub(crate) metadata_uuid: String,
    /// Collection ID for the Artemis execution
    pub(crate) collection_id: u64,
    /// Name of the collection
    pub(crate) collection_name: String,
    /// Start time for the Artemis execution
    pub(crate) start_time: String,
    /// Unix epoch start time for the Artemis execution
    pub(crate) start_time_epoch: u64,
    /// Completion time for the Artemis artifact run
    pub(crate) complete_time: String,
    /// Filter out results with time before start time
    pub(crate) start_time_filter: Option<String>,
    /// Filter out results with time after end time
    pub(crate) end_time_filter: Option<String>,
    /// Metadata associated with the target system
    pub(crate) system: SystemInfoMetadata,
}

impl CollectionContext {
    /// Creates collection context at the start of an Artemis execution
    pub(crate) fn new(config: &OutputConfig, log_file: PathBuf) -> Self {
        let start_time = time_now();
        Self {
            endpoint_id: config.endpoint_id.clone(),
            collection_id: config.collection_id,
            collection_name: config.name.clone(),
            start_time: unixepoch_to_iso(start_time as i64),
            start_time_epoch: start_time,
            log_file,
            system: get_info_metadata(),
        }
    }

    /// Creates artifact context for records created per artifact output
    pub(crate) fn artifact(
        &self,
        artifact_name: &str,
        start_time_filter: &Option<String>,
        end_time_filter: &Option<String>,
    ) -> ArtifactContext {
        let complete = time_now();
        ArtifactContext {
            artifact_name: artifact_name.to_string(),
            endpoint_id: self.endpoint_id.clone(),
            metadata_uuid: generate_uuid(),
            collection_id: self.collection_id,
            collection_name: self.collection_name.clone(),
            start_time: self.start_time.clone(),
            start_time_epoch: self.start_time_epoch,
            complete_time: unixepoch_to_iso(complete as i64),
            system: self.system.clone(),
            start_time_filter: start_time_filter.clone(),
            end_time_filter: end_time_filter.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{output2::context::CollectionContext, structs::toml::OutputConfig};
    use std::path::PathBuf;

    #[test]
    fn test_output_context() {
        let out = OutputConfig::default();

        let context = CollectionContext::new(&out, PathBuf::from("./tmp"));
        assert_eq!(context.collection_name, "");
        assert!(!context.start_time.is_empty());

        let artifact = context.artifact("processes", &out.start_time_filter, &out.end_time_filter);
        assert_eq!(artifact.collection_name, "");
        assert_eq!(artifact.artifact_name, "processes");
    }
}
