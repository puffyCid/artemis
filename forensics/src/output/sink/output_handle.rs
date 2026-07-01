use std::path::PathBuf;

/// A small amount of metadata returned by the sink when output is written to the destination
///
/// Metadata associated with the record written by an output sink
pub(crate) struct OutputHandle {
    /// Output destination
    pub(crate) location: OutputLocation,
    /// How many records were written
    pub(crate) record_count: usize,
}

#[derive(Debug)]
/// Location where an output item was written.
pub(crate) enum OutputLocation {
    /// Output sent to local system
    Local(PathBuf),
    /// Output sent to remote system
    Remote(String),
}

impl OutputHandle {
    /// Create `OutputHandle` for an artifact output item
    pub(crate) fn artifact(location: OutputLocation, record_count: usize) -> Self {
        Self {
            location,
            record_count,
        }
    }

    /// Creates an `OutputHandle` for a collection report output item
    pub(crate) fn report(location: OutputLocation) -> Self {
        Self {
            location,
            record_count: 1,
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
