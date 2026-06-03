use crate::{output2::error::OutputError, structs::toml::Output};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Output configuration for output workflow
///
/// `OutputConfig` describes how artifact results should be encoded, written,
/// filtered, and logged.
#[derive(Debug, Deserialize, Serialize, Default)]
pub(crate) struct OutputConfig {
    /// Name for output folder
    pub name: String,
    /// Endpoint ID for the target system
    pub endpoint_id: String,
    /// Collection ID for the Artemis execution
    pub collection_id: u64,
    /// Folder to store the output data. The `name` folder will be created here
    pub directory: PathBuf,
    /// Output type: local, aws, gcp, azure, or api
    pub destination: OutputDestination,
    /// Output format: json, jsonl, or csv
    pub format: OutputFormat,
    /// Whether to compress the results with gzip. The local output type is then compressed with zip
    pub compress: bool,
    /// Filter out results with time before start time
    pub start_time_filter: Option<String>,
    /// Filter out results with time after end time
    pub end_time_filter: Option<String>,
    /// Apply a filter script before outputting data
    pub filter_name: Option<String>,
    /// Run parsed data through provided filter script
    pub filter_script: Option<String>,
    /// URL for remote uploads
    pub url: Option<String>,
    /// API used for remote uploads
    pub api_key: Option<String>,
    /// Set logging setting. Default is `warn`. Options include: error, warn, info, debug
    pub logging: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Default, PartialEq, Copy, Clone)]
#[serde(rename_all = "lowercase")]
pub(crate) enum OutputFormat {
    Json,
    #[default]
    Jsonl,
    Csv,
    Timeline,
    /// Plaintext output for `BoaJS` runtime data
    Text,
}

/// Determine where our data should be sent
#[derive(Debug, Deserialize, Serialize, Default, PartialEq, Copy, Clone)]
#[serde(rename_all = "lowercase")]
pub(crate) enum OutputDestination {
    /// Local filesystem
    #[default]
    Local,
    /// Upload to an API server
    #[cfg(feature = "api")]
    Api,
    /// Upload to AWS bucket
    #[cfg(feature = "aws")]
    Aws,
    /// Upload to Azure bucket
    #[cfg(feature = "azure")]
    Azure,
    /// Upload to GCP bucket
    #[cfg(feature = "gcp")]
    Gcp,
}

impl TryFrom<Output> for OutputConfig {
    type Error = OutputError;
    /// Convert legacy `Output` structure to modern `OutputConfig` structure
    fn try_from(value: Output) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name,
            endpoint_id: value.endpoint_id,
            collection_id: value.collection_id,
            directory: PathBuf::from(value.directory),
            destination: OutputDestination::parse(&value.output)?,
            format: OutputFormat::parse(&value.format)?,
            compress: value.compress,
            start_time_filter: value.start_time,
            end_time_filter: value.end_time,
            filter_name: value.filter_name,
            filter_script: value.filter_script,
            url: value.url,
            api_key: value.api_key,
            logging: value.logging,
        })
    }
}

impl OutputFormat {
    /// Parse format string to format enum value
    ///
    /// `BoaJS` only formats such as `Text` are rejected
    pub(crate) fn parse(value: &str) -> Result<Self, OutputError> {
        match value.to_ascii_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "" | "jsonl" => Ok(Self::Jsonl),
            "csv" => Ok(Self::Csv),
            "timeline" => Ok(Self::Timeline),
            _ => Err(OutputError::UnsupportedFormat(value.to_string())),
        }
    }

    /// Parse format string for `BoaJS` runtime output
    pub(crate) fn parse_runtime(value: &str) -> Result<Self, OutputError> {
        match value.to_ascii_lowercase().as_str() {
            "txt" | "text" => Ok(Self::Text),
            _ => Self::parse(value),
        }
    }

    /// Return format name for logging and debugging
    pub(crate) fn as_str(&self) -> &str {
        match self {
            OutputFormat::Json => "json",
            OutputFormat::Jsonl => "jsonl",
            OutputFormat::Csv => "csv",
            OutputFormat::Timeline => "timeline",
            OutputFormat::Text => "text",
        }
    }
}

impl OutputDestination {
    /// Parse output location to destination enum value. Default location is local system
    pub(crate) fn parse(value: &str) -> Result<Self, OutputError> {
        match value.to_ascii_lowercase().as_str() {
            "" | "local" => Ok(Self::Local),
            #[cfg(feature = "api")]
            "api" => Ok(Self::Api),
            #[cfg(feature = "azure")]
            "azure" => Ok(Self::Azure),
            #[cfg(feature = "aws")]
            "aws" => Ok(Self::Aws),
            #[cfg(feature = "gcp")]
            "gcp" => Ok(Self::Gcp),
            _ => Err(OutputError::UnsupportedDestination(value.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        output2::{
            config::{OutputConfig, OutputDestination, OutputFormat},
            error::OutputError,
        },
        structs::toml::Output,
    };

    #[test]
    fn test_output_config() {
        let out = Output {
            name: String::from("test"),
            endpoint_id: String::from("test"),
            collection_id: 123,
            directory: String::from("test"),
            output: String::from("local"),
            format: String::from("json"),
            log_file: String::from("test"),
            ..Default::default()
        };

        let out_ng = OutputConfig::try_from(out).unwrap();
        assert_eq!(out_ng.name, "test");
        assert_eq!(out_ng.format, OutputFormat::Json);
        assert_eq!(out_ng.destination, OutputDestination::Local);
    }

    #[test]
    #[cfg(feature = "aws")]
    fn test_output_config_jsonl() {
        let out = Output {
            name: String::from("test"),
            endpoint_id: String::from("test"),
            collection_id: 123,
            directory: String::from("test"),
            output: String::from("aws"),
            format: String::from("jsonl"),
            log_file: String::from("test"),
            ..Default::default()
        };

        let out_ng = OutputConfig::try_from(out).unwrap();
        assert_eq!(out_ng.name, "test");
        assert_eq!(out_ng.format, OutputFormat::Jsonl);
        assert_eq!(out_ng.destination, OutputDestination::Aws);
    }

    #[test]
    #[cfg(feature = "azure")]
    fn test_output_config_csv() {
        let out = Output {
            name: String::from("test"),
            endpoint_id: String::from("test"),
            collection_id: 123,
            directory: String::from("test"),
            output: String::from("azure"),
            format: String::from("csv"),
            log_file: String::from("test"),
            ..Default::default()
        };

        let out_ng = OutputConfig::try_from(out).unwrap();
        assert_eq!(out_ng.name, "test");
        assert_eq!(out_ng.format, OutputFormat::Csv);
        assert_eq!(out_ng.destination, OutputDestination::Azure);
    }

    #[test]
    #[cfg(feature = "azure")]
    fn test_output_config_bad_format() {
        let out = Output {
            name: String::from("test"),
            endpoint_id: String::from("test"),
            collection_id: 123,
            directory: String::from("test"),
            output: String::from("azure"),
            format: String::from("test"),
            log_file: String::from("test"),
            ..Default::default()
        };

        let err = OutputConfig::try_from(out).unwrap_err();
        assert!(matches!(err, OutputError::UnsupportedFormat(value) if value == "test"))
    }
}
