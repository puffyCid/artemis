use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ChromiumDownloads {
    pub downloads: Vec<ChromiumDownload>,
    pub path: String,
    pub user: String,
}

#[derive(Debug, Serialize)]
pub struct ChromiumDownload {
    pub id: i64,
    pub guid: String,
    pub current_path: String,
    pub target_path: String,
    pub start_time: i64,
    pub received_bytes: i64,
    pub total_bytes: i64,
    pub state: i64,
    pub danger_type: i64,
    pub interrupt_reason: i64,
    pub hash: Vec<u8>,
    pub end_time: i64,
    pub opened: i64,
    pub last_access_time: i64,
    pub transient: i64,
    pub referrer: String,
    pub site_url: String,
    pub tab_url: String,
    pub tab_referrer_url: String,
    pub http_method: String,
    pub by_ext_id: String,
    pub by_ext_name: String,
    pub etag: String,
    pub last_modified: String,
    pub mime_type: String,
    pub original_mime_type: String,
    pub downloads_url_chain_id: i64,
    pub chain_index: i64,
    pub url: String,
}

#[derive(Serialize)]
pub struct ChromiumHistory {
    pub history: Vec<ChromiumHistoryEntry>,
    pub path: String,
    pub user: String,
}

#[derive(Serialize)]
pub struct ChromiumHistoryEntry {
    pub id: i64,
    pub url: String,   // Can be null
    pub title: String, // Can be null
    pub visit_count: i64,
    pub typed_count: i64,
    pub last_visit_time: i64,
    pub hidden: i64,
    pub visits_id: i64,
    pub from_visit: i64, // Can be null
    pub transition: i64,
    pub segment_id: i64, // Can be null
    pub visit_duration: i64,
    pub opener_visit: i64, // Can be null
}

#[derive(Debug, Serialize)]
pub struct FirefoxDownloads {
    pub downloads: Vec<FirefoxDownload>,
    pub path: String,
    pub user: String,
}

#[derive(Debug, Serialize)]
pub struct FirefoxDownload {
    pub id: i64,
    pub place_id: i64,
    pub anno_attribute_id: i64,
    pub content: String,
    pub flags: i64,
    pub expiration: i64,
    pub download_type: i64,
    pub date_added: i64,
    pub last_modified: i64,
    pub name: String,
    pub history: FirefoxHistoryEntry,
}

#[derive(Debug, Serialize)]
pub struct FirefoxHistory {
    pub history: Vec<FirefoxHistoryEntry>,
    pub path: String,
    pub user: String,
}

#[derive(Debug, Serialize)]
pub struct FirefoxHistoryEntry {
    pub moz_places_id: i64,
    pub url: String,   // Can be null
    pub title: String, // Can be null
    pub rev_host: String,
    pub visit_count: i64,
    pub hidden: i64,
    pub typed: i64,
    pub frequency: i64,
    pub last_visit_date: i64, // Can be null
    pub guid: String,
    pub foreign_count: i64, // Can be null
    pub url_hash: i64,
    pub description: String,       // Can be null
    pub preview_image_url: String, // Can be null
    pub prefix: String,
    pub host: String,
}

#[derive(Debug, Serialize)]
pub struct SafariDownload {
    pub source_url: String,
    pub download_path: String,
    pub sandbox_id: String,
    pub download_bytes: i64,
    pub download_id: String,
    pub download_entry_date: u64,
    pub download_entry_finish: u64,
    pub path: Vec<String>,             // Path to binary to run
    pub cnid_path: Vec<i64>,           // Path represented as Catalog Node ID
    pub created: i64,                  // Created timestamp of binary target
    pub volume_path: String,           // Root
    pub volume_url: String,            // URL type
    pub volume_name: String,           // Name of Volume
    pub volume_uuid: String,           // Volume UUID string
    pub volume_size: i64,              // Size of Volume
    pub volume_created: i64,           // Created timestamp of Volume
    pub volume_flag: Vec<u64>,         // Volume Property flags
    pub volume_root: bool,             // If Volume is filesystem root
    pub localized_name: String,        // Optional localized name of target binary
    pub security_extension_rw: String, // Optional Security extension of target binary
    pub security_extension_ro: String, // Optional Security extension of target binary
    pub target_flags: Vec<u64>,        // Resource property flags
    pub username: String,              // Username related to bookmark
    pub folder_index: i64,             // Folder index number
    pub uid: i32,                      // User UID
    pub creation_options: i32,         // Bookmark creation options
    pub is_executable: bool,           // Can target be executed
    pub file_ref_flag: bool,
}

#[derive(Debug, Serialize)]
pub struct SafariDownloads {
    pub results: Vec<SafariDownload>,
    pub path: String,
    pub user: String,
}

#[derive(Debug, Serialize)]
pub struct SafariHistory {
    pub results: Vec<SafariHistoryEntry>,
    pub path: String,
    pub user: String,
}

#[derive(Debug, Serialize)]
pub struct SafariHistoryEntry {
    pub id: i64,
    pub url: String,
    pub domain_expansion: String, // Can be null
    pub visit_count: i64,
    pub daily_visit_counts: Vec<u8>,    // Can be null
    pub weekly_visit_counts: Vec<u8>,   // Can be null
    pub autocomplete_triggers: Vec<u8>, // Can be null
    pub should_recompute_derived_visit_counts: i64,
    pub visit_count_score: i64,
    pub status_code: i64,
    pub visit_time: i64,
    pub load_successful: bool,
    pub title: String, // Can be null
    pub attributes: f64,
    pub score: f64,
}
