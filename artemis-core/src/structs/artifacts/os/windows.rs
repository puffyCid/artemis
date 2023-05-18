use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct PrefetchOptions {
    pub(crate) alt_drive: Option<char>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EventLogsOptions {
    pub(crate) alt_drive: Option<char>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RawFilesOptions {
    pub(crate) drive_letter: char,
    pub(crate) start_path: String,
    pub(crate) depth: u8,
    /**Extract deleted indx entries */
    pub(crate) recover_indx: bool,
    pub(crate) md5: Option<bool>,
    pub(crate) sha1: Option<bool>,
    pub(crate) sha256: Option<bool>,
    pub(crate) metadata: Option<bool>,
    pub(crate) path_regex: Option<String>,
    pub(crate) filename_regex: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ShimdbOptions {
    pub(crate) alt_drive: Option<char>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RegistryOptions {
    pub(crate) user_hives: bool,
    pub(crate) system_hives: bool,
    pub(crate) alt_drive: Option<char>,
    pub(crate) path_regex: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct UserAssistOptions {
    pub(crate) alt_drive: Option<char>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ShimcacheOptions {
    pub(crate) alt_drive: Option<char>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ShellbagsOptions {
    pub(crate) resolve_guids: bool,
    pub(crate) alt_drive: Option<char>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AmcacheOptions {
    pub(crate) alt_drive: Option<char>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ShortcutOptions {
    /**Path to directory containing `Shortcut (lnk)` files */
    pub(crate) path: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct UsnJrnlOptions {
    pub(crate) alt_drive: Option<char>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct BitsOptions {
    pub(crate) alt_path: Option<String>,
    pub(crate) carve: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SrumOptions {
    pub(crate) alt_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct UserOptions {
    pub(crate) alt_drive: Option<char>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SearchOptions {
    pub(crate) alt_path: Option<String>,
}
