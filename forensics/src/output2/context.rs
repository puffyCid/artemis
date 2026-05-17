use crate::{
    artifacts::os::systeminfo::info::get_info_metadata,
    output2::config::OutputConfig,
    utils::{
        time::{time_now, unixepoch_to_iso},
        uuid::generate_uuid,
    },
};
use common::system::SystemInfoMetadata;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct CollectionContext {
    pub(crate) endpoint_id: String,
    pub(crate) collection_id: u64,
    pub(crate) collection_name: String,
    pub(crate) start_time: String,
    pub(crate) start_time_epoch: u64,
    pub(crate) log_file: PathBuf,
    pub(crate) system: SystemInfoMetadata,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub(crate) struct ArtifactContext {
    pub(crate) artifact_name: String,
    pub(crate) endpoint_id: String,
    pub(crate) metadata_uuid: String,
    pub(crate) collection_id: u64,
    pub(crate) collection_name: String,
    pub(crate) start_time: String,
    pub(crate) start_time_epoch: u64,
    pub(crate) complete_time: String,
    pub(crate) system: SystemInfoMetadata,
}

/**
 * Setup the entire Artemis collection context
 * Contains metadata associated with the entire collection workflow
 */
impl CollectionContext {
    /**
     * Collect the initial metadata at the start of the artemis execution
     */
    pub(crate) fn new(config: &OutputConfig, start_time: u64, log_file: PathBuf) -> Self {
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

    /**
     * Metadata associated with each artifact value record
     */
    pub(crate) fn artifact(&self, artifact_name: &str) -> ArtifactContext {
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
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{
        output2::{config::OutputConfig, context::CollectionContext},
        structs::toml::Output,
        utils::time::time_now,
    };

    #[test]
    fn test_output_context() {
        let out = Output::default();
        let out_ng = OutputConfig::try_from(out).unwrap();

        let context = CollectionContext::new(&out_ng, time_now(), PathBuf::from("./tmp"));
        assert_eq!(context.collection_name, "");
        assert!(!context.start_time.is_empty());

        let artifact = context.artifact("processes");
        assert_eq!(artifact.collection_name, "");
        assert_eq!(artifact.artifact_name, "processes");
    }
}
