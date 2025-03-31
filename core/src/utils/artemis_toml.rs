use super::error::ArtemisError;
use crate::structs::toml::ArtemisToml;
use log::error;
use std::str::from_utf8;

impl ArtemisToml {
    // Parse the Artemis TOML collector file
    pub(crate) fn parse_artemis_toml(toml_data: &[u8]) -> Result<ArtemisToml, ArtemisError> {
        let toml_results = toml::from_str(from_utf8(toml_data).unwrap_or_default());
        let mut artemis_collector: ArtemisToml = match toml_results {
            Ok(results) => results,
            Err(err) => {
                error!("[core] Artemis failed to parse TOML data. Error: {err:?}");
                return Err(ArtemisError::BadToml);
            }
        };

        // Format is always lowercase
        artemis_collector.output.format = artemis_collector.output.format.to_lowercase();
        Ok(artemis_collector)
    }
}

#[cfg(test)]
mod tests {
    use crate::{filesystem::files::read_file, utils::artemis_toml::ArtemisToml};
    use std::path::PathBuf;

    #[test]
    #[cfg(target_os = "macos")]
    fn test_parse_artemis_toml() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos.toml");

        let buffer = read_file(&test_location.display().to_string()).unwrap();

        let result = ArtemisToml::parse_artemis_toml(&buffer).unwrap();
        assert_eq!(result.output.compress, true);
        assert_eq!(result.output.name, "macos_collection");
        assert_eq!(result.output.directory, "./tmp");
        assert_eq!(result.output.format, "jsonl");
        assert_eq!(result.output.output, "local");

        assert_eq!(result.artifacts[0].artifact_name, "processes");
        assert_eq!(result.artifacts[0].processes.as_ref().unwrap().md5, true);
    }

    #[test]
    #[should_panic(expected = "BadToml")]
    fn test_parse_artemis_bad_toml() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/malformed_tests/badfiles.toml");

        let buffer = read_file(&test_location.display().to_string()).unwrap();

        let result = ArtemisToml::parse_artemis_toml(&buffer).unwrap();
        assert_eq!(result.output.compress, false);
        assert_eq!(result.output.name, "macos_collection");
        assert_eq!(result.output.directory, "./tmp");
        assert_eq!(result.output.format, "local");

        assert_eq!(result.artifacts[0].artifact_name, "processes");
    }
}
