use std::path::PathBuf;

pub(crate) struct OutputHandle {
    pub(crate) artifact_name: String,
    pub(crate) location: OutputLocation,
    pub(crate) record_count: usize,
    pub(crate) extension: String,
    pub(crate) compressed: bool,
    pub(crate) output_type: OutputType,
}

pub(crate) enum OutputLocation {
    Local(PathBuf),
    Remote(String),
}

pub(crate) enum OutputType {
    Artifact,
    Report,
    Log,
}

impl OutputHandle {
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
    pub(crate) fn location_string(&self) -> String {
        match &self.location {
            OutputLocation::Local(path) => path.display().to_string(),
            OutputLocation::Remote(location) => location.clone(),
        }
    }
}
