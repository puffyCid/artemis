/// How the accessor resolves paths against a live OS
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) enum AccessMode {
    /// Use OS APIs to access the system
    #[default]
    Live,
    /// Read the raw volume to access the system
    Raw,
    /// Try live access first, then raw if `AccessorError::is_recoverable_via_raw`
    /// suggests it may help
    Auto,
}

/// Limits and behavior for an `Accessor` instance
#[derive(Debug, Clone)]
pub(crate) struct AccessorConfig {
    /// How the file or directory be accessed
    pub(crate) access_mode: AccessMode,
    /// Maximum size for `read_file` reads. Default is 2GB
    pub(crate) max_read_size: Option<u64>,
}

impl Default for AccessorConfig {
    fn default() -> Self {
        Self {
            access_mode: AccessMode::Live,
            max_read_size: Some (2 * 1024 * 1024 * 1024),
        }
    }
}
