use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct JournalOptions {
    pub alt_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LinuxSudoOptions {
    pub alt_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LogonOptions {
    pub alt_file: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Ext4Options {
    pub start_path: String,
    pub depth: u8,
    pub device: Option<String>,
    pub md5: Option<bool>,
    pub sha1: Option<bool>,
    pub sha256: Option<bool>,
    pub path_regex: Option<String>,
    pub filename_regex: Option<String>,
}
