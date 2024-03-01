use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct JournalOptions {
    pub alt_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SudoOptions {
    pub alt_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LogonOptions {
    pub alt_file: Option<String>,
}
