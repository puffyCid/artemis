use crate::macos::{CreationFlags, TargetFlags, VolumeFlags};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SafariDownload {
    pub source_url: String,
    pub download_path: String,
    pub sandbox_id: String,
    pub download_bytes: i64,
    pub download_id: String,
    pub download_entry_date: String,
    pub download_entry_finish: String,
    pub path: String,                         // Path to binary to run
    pub cnid_path: String,                    // Path represented as Catalog Node ID
    pub created: String,                      // Created timestamp of binary target
    pub volume_path: String,                  // Root
    pub volume_url: String,                   // URL type
    pub volume_name: String,                  // Name of Volume
    pub volume_uuid: String,                  // Volume UUID string
    pub volume_size: i64,                     // Size of Volume
    pub volume_created: String,               // Created timestamp of Volume
    pub volume_flags: Vec<VolumeFlags>,       // Volume Property flags
    pub volume_root: bool,                    // If Volume is filesystem root
    pub localized_name: String,               // Optional localized name of target binary
    pub security_extension_rw: String,        // Optional Security extension of target binary
    pub security_extension_ro: String,        // Optional Security extension of target binary
    pub target_flags: Vec<TargetFlags>,       // Resource property flags
    pub username: String,                     // Username related to bookmark
    pub folder_index: i64,                    // Folder index number
    pub uid: i32,                             // User UID
    pub creation_options: Vec<CreationFlags>, // Bookmark creation options
    pub is_executable: bool,                  // Can target be executed
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
    pub visit_time: String,
    pub load_successful: bool,
    pub title: String, // Can be null
    pub attributes: f64,
    pub score: f64,
}
