use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct JSScript {
    pub(crate) name: String,
    pub(crate) script: String, // Base64 encoded js script
}
