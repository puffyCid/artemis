use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct PrefetchOptions {
    pub alt_drive: Option<char>,
}

#[derive(Debug, Deserialize)]
pub struct EventLogsOptions {
    pub alt_drive: Option<char>,
}

#[derive(Debug, Deserialize)]
pub struct RawFilesOptions {
    pub drive_letter: char,
    pub start_path: String,
    pub depth: u8,
    /**Extract deleted indx entries */
    pub recover_indx: bool,
    pub md5: Option<bool>,
    pub sha1: Option<bool>,
    pub sha256: Option<bool>,
    pub metadata: Option<bool>,
    pub path_regex: Option<String>,
    pub filename_regex: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ShimdbOptions {
    pub alt_drive: Option<char>,
}

#[derive(Debug, Deserialize)]
pub struct RegistryOptions {
    pub user_hives: bool,
    pub system_hives: bool,
    pub alt_drive: Option<char>,
    pub path_regex: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UserAssistOptions {
    pub alt_drive: Option<char>,
    pub resolve_descriptions: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ShimcacheOptions {
    pub alt_drive: Option<char>,
}

#[derive(Debug, Deserialize)]
pub struct ShellbagsOptions {
    pub resolve_guids: bool,
    pub alt_drive: Option<char>,
}

#[derive(Debug, Deserialize)]
pub struct AmcacheOptions {
    pub alt_drive: Option<char>,
}

#[derive(Debug, Deserialize)]
pub struct ShortcutOptions {
    /**Path to directory containing `Shortcut (lnk)` files */
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct UsnJrnlOptions {
    pub alt_drive: Option<char>,
}

#[derive(Debug, Deserialize)]
pub struct BitsOptions {
    pub alt_file: Option<String>,
    pub carve: bool,
}

#[derive(Debug, Deserialize)]
pub struct SrumOptions {
    pub alt_file: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UserOptions {
    pub alt_drive: Option<char>,
}

#[derive(Debug, Deserialize)]
pub struct SearchOptions {
    pub alt_file: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TasksOptions {
    pub alt_drive: Option<char>,
}

#[derive(Debug, Deserialize)]
pub struct ServicesOptions {
    pub alt_drive: Option<char>,
}

#[derive(Debug, Deserialize)]
pub struct JumplistsOptions {
    pub alt_drive: Option<char>,
}

#[derive(Debug, Deserialize)]
pub struct RecycleBinOptions {
    pub alt_drive: Option<char>,
}

#[derive(Debug, Deserialize)]
pub struct WmiPersistOptions {
    pub alt_drive: Option<char>,
    pub alt_dir: Option<String>,
}
