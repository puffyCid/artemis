use super::error::ArtemisError;
use crate::structs::{artifacts::triage::ArtemisTriage, toml::ArtemisToml};
use log::error;
use reqwest::blocking::Client;

impl ArtemisToml {
    /// Parse a remotely hosted TOML file
    pub(crate) fn remote_artemis_toml(url: &str) -> Result<ArtemisToml, ArtemisError> {
        let client = match Client::builder().build() {
            Ok(result) => result,
            Err(err) => {
                error!("[forensics] Could not create HTTP client for remote toml: {err:?}");
                return Err(ArtemisError::Remote);
            }
        };
        let mut request = client.get(url);
        let version = format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        request = request.header("User-Agent", version);
        let response = match request.send() {
            Ok(result) => result,
            Err(err) => {
                error!("[forensics] Could not parse response from remote TOML: {err:?}");
                return Err(ArtemisError::Remote);
            }
        };

        ArtemisToml::parse_artemis_toml(&response.bytes().unwrap_or_default())
    }

    /// Parse the Artemis TOML collector file
    pub(crate) fn parse_artemis_toml(toml_data: &[u8]) -> Result<ArtemisToml, ArtemisError> {
        let toml_results = toml::from_slice(toml_data);
        let mut artemis_collector: ArtemisToml = match toml_results {
            Ok(results) => results,
            Err(err) => {
                error!("[forensics] Artemis failed to parse TOML data. Error: {err:?}");
                return Err(ArtemisError::BadToml);
            }
        };

        // Format is always lowercase
        artemis_collector.output.format = artemis_collector.output.format.to_lowercase();
        Ok(artemis_collector)
    }

    /// Parse the KAPE TOML triage format
    pub(crate) fn parse_triage_toml(toml_data: &[u8]) -> Result<ArtemisTriage, ArtemisError> {
        let toml_results = toml::from_slice(toml_data);
        let triage: ArtemisTriage = match toml_results {
            Ok(results) => results,
            Err(err) => {
                println!("[forensics] Artemis failed to parse TOML triage data. Error: {err:?}");
                return Err(ArtemisError::BadToml);
            }
        };
        Ok(triage)
    }
}

#[cfg(test)]
mod tests {
    use crate::{filesystem::files::read_file, utils::artemis_toml::ArtemisToml};
    use httpmock::{Method::GET, MockServer};
    use std::path::PathBuf;

    #[test]
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
    fn test_mock_remote_toml() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos.toml");

        let buffer = read_file(&test_location.display().to_string()).unwrap();

        let server = MockServer::start();
        let port = server.port();
        let mock_me = server.mock(|when, then| {
            when.method(GET);
            then.status(200).body(&buffer);
        });

        let url = format!("http://127.0.0.1:{port}");
        let value = ArtemisToml::remote_artemis_toml(&url).unwrap();
        assert_eq!(value.output.name, "macos_collection");
        assert_eq!(value.artifacts.len(), 10);
        mock_me.assert();
    }

    #[test]
    fn test_remote_toml_github() {
        let value = ArtemisToml::remote_artemis_toml("https://raw.githubusercontent.com/puffyCid/artemis/refs/heads/main/forensics/tests/test_data/linux.toml").unwrap();
        assert_eq!(value.output.name, "linux_collection");
        assert_eq!(value.artifacts.len(), 3);
    }

    #[test]
    #[should_panic(expected = "BadToml")]
    fn test_remote_bad_toml_github() {
        let _ = ArtemisToml::remote_artemis_toml("https://raw.githubusercontent.com/puffyCid/artemis/refs/heads/main/forensics/tests/test_data/fake.toml").unwrap();
    }

    #[test]
    #[should_panic(expected = "Remote")]
    fn test_remote_bad_toml_domain() {
        let _ = ArtemisToml::remote_artemis_toml("https://raw.google.com/puffyCid/artemis/refs/heads/main/forensics/tests/test_data/fake.toml").unwrap();
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

    #[test]
    fn test_parse_triage_toml() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/triage/windows/Chrome.toml");

        let buffer = read_file(&test_location.display().to_string()).unwrap();

        let result = ArtemisToml::parse_triage_toml(&buffer).unwrap();
        assert_eq!(result.targets.len(), 47);

        assert_eq!(result.targets[15].name, "Chrome Cookies");
        assert_eq!(result.targets[32].file_mask, "QuotaManager*")
    }

    #[test]
    #[should_panic(expected = "BadToml")]
    fn test_bad_triage_toml() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/triage/malformed/bad.toml");

        let buffer = read_file(&test_location.display().to_string()).unwrap();

        let _ = ArtemisToml::parse_triage_toml(&buffer).unwrap();
    }
}
