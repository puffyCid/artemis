use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct TriageOptions {
    pub triage: Vec<TriageTargets>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TriageTargets {
    pub name: String,
    pub path: String,
    pub file_mask: String,
    pub recursive: bool,
    pub recreate_directories: bool,
}
