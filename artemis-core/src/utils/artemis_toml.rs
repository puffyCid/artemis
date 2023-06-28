use super::error::ArtemisError;
use crate::structs::artifacts::{
    os::{files::FileOptions, processes::ProcessOptions},
    runtime::script::JSScript,
};
use log::error;
use serde::Deserialize;
use std::str::from_utf8;

// Target specific dependencies
#[cfg(target_os = "windows")]
use crate::structs::artifacts::os::windows::{
    AmcacheOptions, BitsOptions, EventLogsOptions, PrefetchOptions, RawFilesOptions,
    RegistryOptions, SearchOptions, ShellbagsOptions, ShimcacheOptions, ShimdbOptions,
    ShortcutOptions, SrumOptions, UserAssistOptions, UserOptions, UsnJrnlOptions,
};

#[cfg(target_os = "macos")]
use crate::structs::artifacts::os::macos::UnifiedLogsOptions;

#[derive(Debug, Deserialize)]
pub(crate) struct ArtemisToml {
    pub(crate) system: String,
    pub(crate) output: Output,
    pub(crate) artifacts: Vec<Artifacts>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Output {
    pub(crate) name: String,
    pub(crate) endpoint_id: String,
    pub(crate) collection_id: u64,
    pub(crate) directory: String,
    pub(crate) output: String,
    pub(crate) format: String,
    pub(crate) compress: bool,
    pub(crate) filter_name: Option<String>,
    pub(crate) filter_script: Option<String>,
    pub(crate) url: Option<String>,
    pub(crate) api_key: Option<String>,
    pub(crate) logging: Option<String>,
}

#[derive(Debug, Deserialize)]
#[cfg(target_os = "macos")]
pub(crate) struct Artifacts {
    /**Based on artifact parse one of the artifact types */
    pub(crate) artifact_name: String,
    /**Specify whether to filter the parsed data */
    pub(crate) filter: Option<bool>,
    pub(crate) processes: Option<ProcessOptions>,
    pub(crate) files: Option<FileOptions>,
    pub(crate) unifiedlogs: Option<UnifiedLogsOptions>,
    pub(crate) script: Option<JSScript>,
}

#[derive(Debug, Deserialize)]
#[cfg(target_os = "windows")]
pub(crate) struct Artifacts {
    /**Based on artifact parse one of the artifact types */
    pub(crate) artifact_name: String,
    /**Specify whether to filter the parsed data */
    pub(crate) filter: Option<bool>,
    pub(crate) eventlogs: Option<EventLogsOptions>,
    pub(crate) prefetch: Option<PrefetchOptions>,
    pub(crate) processes: Option<ProcessOptions>,
    pub(crate) rawfiles: Option<RawFilesOptions>,
    pub(crate) files: Option<FileOptions>,
    pub(crate) shimdb: Option<ShimdbOptions>,
    pub(crate) registry: Option<RegistryOptions>,
    pub(crate) userassist: Option<UserAssistOptions>,
    pub(crate) shimcache: Option<ShimcacheOptions>,
    pub(crate) shellbags: Option<ShellbagsOptions>,
    pub(crate) amcache: Option<AmcacheOptions>,
    pub(crate) script: Option<JSScript>,
    pub(crate) shortcuts: Option<ShortcutOptions>,
    pub(crate) usnjrnl: Option<UsnJrnlOptions>,
    pub(crate) bits: Option<BitsOptions>,
    pub(crate) srum: Option<SrumOptions>,
    pub(crate) users: Option<UserOptions>,
    pub(crate) search: Option<SearchOptions>,
}

#[derive(Debug, Deserialize)]
#[cfg(target_os = "linux")]
pub(crate) struct Artifacts {
    /**Based on artifact parse one of the artifact types */
    pub(crate) artifact_name: String,
    /**Specify whether to filter the parsed data */
    pub(crate) filter: Option<bool>,
    pub(crate) processes: Option<ProcessOptions>,
    pub(crate) files: Option<FileOptions>,
    pub(crate) script: Option<JSScript>,
}

impl ArtemisToml {
    // Parse the Artemis TOML collector file
    pub(crate) fn parse_artemis_toml_data(toml_data: &[u8]) -> Result<ArtemisToml, ArtemisError> {
        let toml_results = toml::from_str(from_utf8(toml_data).unwrap_or_default());
        let artemis_collector: ArtemisToml = match toml_results {
            Ok(results) => results,
            Err(err) => {
                error!("[artemis-core] Artemis failed to parse TOML data. Error: {err:?}");
                return Err(ArtemisError::BadToml);
            }
        };
        Ok(artemis_collector)
    }
}

#[cfg(test)]
mod tests {
    use crate::{filesystem::files::read_file, utils::artemis_toml::ArtemisToml};
    use std::path::PathBuf;

    #[test]
    #[cfg(target_os = "macos")]
    fn test_parse_artemis_toml_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos.toml");

        let buffer = read_file(&test_location.display().to_string()).unwrap();

        let result = ArtemisToml::parse_artemis_toml_data(&buffer).unwrap();
        assert_eq!(result.output.compress, true);
        assert_eq!(result.output.name, "macos_collection");
        assert_eq!(result.output.directory, "./tmp");
        assert_eq!(result.output.format, "jsonl");
        assert_eq!(result.system, "macos");
        assert_eq!(result.output.output, "local");

        assert_eq!(result.artifacts[0].artifact_name, "processes");
        assert_eq!(result.artifacts[0].processes.as_ref().unwrap().md5, true);
    }

    #[test]
    #[should_panic(expected = "BadToml")]
    fn test_parse_artemis_bad_filetoml_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/malformed_tests/badfiles.toml");

        let buffer = read_file(&test_location.display().to_string()).unwrap();

        let result = ArtemisToml::parse_artemis_toml_data(&buffer).unwrap();
        assert_eq!(result.output.compress, false);
        assert_eq!(result.output.name, "macos_collection");
        assert_eq!(result.output.directory, "./tmp");
        assert_eq!(result.output.format, "local");
        assert_eq!(result.system, "macos");

        assert_eq!(result.artifacts[0].artifact_name, "processes");
    }
}
