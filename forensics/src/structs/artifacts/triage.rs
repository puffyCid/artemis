use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct TriageOptions {
    pub triage: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct ArtemisTriage {
    pub(crate) description: String,
    pub(crate) author: String,
    pub(crate) version: f32,
    pub(crate) recreate_directories: bool,
    pub(crate) targets: Vec<Targets>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Targets {
    pub(crate) name: String,
    pub(crate) category: String,
    pub(crate) recursive: bool,
    pub(crate) file_mask: String,
    pub(crate) path: String,
}
