use std::path::PathBuf;

/// A small amount of metadata returned by the sink when output is written to the destination
///
/// Metadata associated with the record written by an output sink
pub(crate) struct OutputHandle {
    /// Artifact name
    pub(crate) artifact_name: String,
    /// Output destination
    pub(crate) location: OutputLocation,
    /// How many records were written
    pub(crate) record_count: usize,
    /// Output file extension
    pub(crate) extension: String,
    /// Whether the output compressed
    pub(crate) compressed: bool,
    /// Type of output item
    pub(crate) output_type: OutputType,
}

/// Location where an output item was written.
pub(crate) enum OutputLocation {
    /// Output sent to local system
    Local(PathBuf),
    /// Output sent to remote system
    Remote(String),
}

/// What type of file was output
pub(crate) enum OutputType {
    /// Artifact result output
    Artifact,
    /// Report result output
    Report,
    /// Log file result output
    Log,
}

impl OutputHandle {
    /// Create `OutputHandle` for an artifact output item
    pub(crate) fn artifact(
        artifact_name: &str,
        location: OutputLocation,
        record_count: usize,
        extension: &str,
        compressed: bool,
    ) -> Self {
        Self {
            artifact_name: artifact_name.to_string(),
            location,
            record_count,
            extension: extension.to_string(),
            compressed,
            output_type: OutputType::Artifact,
        }
    }

    /// Creates an `OutputHandle` for a collection report output item
    pub(crate) fn report(location: OutputLocation) -> Self {
        Self {
            artifact_name: String::from("report"),
            location,
            record_count: 1,
            extension: String::from("json"),
            compressed: false,
            output_type: OutputType::Report,
        }
    }

    /// Create `OutputHandle` for a log output item
    pub(crate) fn log(location: OutputLocation) -> Self {
        Self {
            artifact_name: String::from("logs"),
            location,
            record_count: 1,
            extension: String::from("log"),
            compressed: false,
            output_type: OutputType::Log,
        }
    }

    /// Convert sink destination location to string
    pub(crate) fn location_string(&self) -> String {
        match &self.location {
            OutputLocation::Local(path) => path.display().to_string(),
            OutputLocation::Remote(location) => location.clone(),
        }
    }
}
