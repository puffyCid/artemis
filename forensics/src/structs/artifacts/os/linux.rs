use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct JournalOptions {
    pub alt_dir: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LinuxSudoOptions {
    pub alt_dir: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LogonOptions {
    pub alt_file: Option<String>,
}
