use crate::{
    output2::error::OutputError,
    structs::toml::{OutputDestination, OutputFormat},
};

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
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use std::path::PathBuf;

    #[test]
    fn test_output_config() {
        let out = OutputConfig {
            name: String::from("test"),
            endpoint_id: String::from("test"),
            collection_id: 123,
            directory: PathBuf::from("test"),
            format: OutputFormat::Jsonl,
            destination: OutputDestination::Local,
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
        let out = OutputConfig {
            name: String::from("test"),
            endpoint_id: String::from("test"),
            collection_id: 123,
            directory: PathBuf::from("test"),
            format: OutputFormat::Jsonl,
            destination: OutputDestination::Aws,
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
        let out = OutputConfig {
            name: String::from("test"),
            endpoint_id: String::from("test"),
            collection_id: 123,
            directory: PathBuf::from("test"),
            format: OutputFormat::Jsonl,
            destination: OutputDestination::Azure,
            ..Default::default()
        };

        let out_ng = OutputConfig::try_from(out).unwrap();
        assert_eq!(out_ng.name, "test");
        assert_eq!(out_ng.format, OutputFormat::Csv);
        assert_eq!(out_ng.destination, OutputDestination::Azure);
    }
}
