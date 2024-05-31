use std::collections::HashMap;

use plist::Dictionary;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize)]
pub struct OpendirectoryUsers {
    pub uid: Vec<String>,
    pub gid: Vec<String>,
    pub name: Vec<String>,
    pub real_name: Vec<String>,
    pub account_photo: Vec<String>,
    pub account_created: f64,
    pub password_last_set: f64,
    pub shell: Vec<String>,
    pub unlock_options: Vec<String>,
    pub home_path: Vec<String>,
    pub uuid: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct OpendirectoryGroups {
    pub gid: Vec<String>,
    pub name: Vec<String>,
    pub real_name: Vec<String>,
    pub users: Vec<String>,
    pub groupmembers: Vec<String>,
    pub uuid: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct BookmarkData {
    /**Path to file to run */
    pub path: Vec<String>,
    /**Path represented as Catalog Node ID */
    pub cnid_path: Vec<i64>,
    /**Created timestamp of target file in UNIXEPOCH seconds */
    pub created: i64,
    /**Path to the volume of target file */
    pub volume_path: String,
    /**Target file URL type */
    pub volume_url: String,
    /**Name of volume target file is on */
    pub volume_name: String,
    /**Volume UUID */
    pub volume_uuid: String,
    /**Size of target volume in bytes */
    pub volume_size: i64,
    /**Created timestamp of volume in UNIXEPOCH seconds */
    pub volume_created: i64,
    /**Volume Property flags */
    pub volume_flag: Vec<u64>,
    /**Flag if volume if the root filesystem */
    pub volume_root: bool,
    /**Localized name of target file */
    pub localized_name: String,
    /**Read-Write security extension of target file */
    pub security_extension_rw: String,
    /**Read-Only security extension of target file */
    pub security_extension_ro: String,
    /**File property flags */
    pub target_flags: Vec<u64>,
    /**Username associated with `Bookmark` */
    pub username: String,
    /**Folder index number associated with target file */
    pub folder_index: i64,
    /**UID associated with `Bookmark` */
    pub uid: i32,
    /**`Bookmark` creation flags */
    pub creation_options: i32,
    /**Is target file executable */
    pub is_executable: bool,
    /**Does target file have file reference flag */
    pub file_ref_flag: bool,
}

#[derive(Debug, Serialize)]
pub struct EmondData {
    pub name: String,
    pub enabled: bool,
    pub event_types: Vec<String>,
    pub start_time: String,
    pub allow_partial_criterion_match: bool,
    pub command_actions: Vec<Command>,
    pub log_actions: Vec<Log>,
    pub send_email_actions: Vec<SendEmail>,
    pub send_sms_actions: Vec<SendEmail>, // Same format as SendEmail
    pub send_notification_actions: Vec<SendNotification>,
    pub criterion: Vec<Dictionary>,
    pub variables: Vec<Dictionary>,
    pub emond_clients_enabled: bool,
}

#[derive(Debug)]
pub struct Actions {
    pub command_actions: Vec<Command>,
    pub log_actions: Vec<Log>,
    pub send_email_actions: Vec<SendEmail>,
    pub send_sms_action: Vec<SendEmail>,
    pub send_notification: Vec<SendNotification>,
}

#[derive(Debug, Serialize)]
pub struct Command {
    pub command: String,
    pub user: String,
    pub group: String,
    pub arguments: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Log {
    pub message: String,
    pub facility: String,
    pub log_level: String,
    pub log_type: String,
    pub parameters: Dictionary,
}

#[derive(Debug, Serialize)]
pub struct SendEmail {
    pub message: String,
    pub subject: String,
    pub localization_bundle_path: String,
    pub relay_host: String,
    pub admin_email: String,
    pub recipient_addresses: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SendNotification {
    pub name: String,
    pub message: String,
    pub details: Dictionary,
}

#[derive(Debug, Serialize)]
pub struct ExecPolicy {
    pub is_signed: i64,
    pub file_identifier: String,
    pub bundle_identifier: String,
    pub bundle_version: String,
    pub team_identifier: String,
    pub signing_identifier: String,
    pub cdhash: String,
    pub main_executable_hash: String,
    pub executable_timestamp: i64,
    pub file_size: i64,
    pub is_library: i64,
    pub is_used: i64,
    pub responsible_file_identifier: String,
    pub is_valid: i64,
    pub is_quarantined: i64,
    pub executable_measurements_v2_timestamp: i64,
    pub reported_timstamp: i64,
    pub pk: i64,
    pub volume_uuid: String,
    pub object_id: i64,
    pub fs_type_name: String,
    pub bundle_id: String,
    pub policy_match: i64,
    pub malware_result: i64,
    pub flags: i64,
    pub mod_time: i64,
    pub policy_scan_cache_timestamp: i64,
    pub revocation_check_time: i64,
    pub scan_version: i64,
    pub top_policy_match: i64,
}

#[derive(Debug, Serialize)]
pub struct FsEvents {
    /**Flags associated with `FsEvent` record */
    pub flags: Vec<String>,
    /**Full path for `FsEvent` record */
    pub path: String,
    /**Node ID for the `FsEvent` record */
    pub node: u64,
    /**Event ID for the `FsEvent` record */
    pub event_id: u64,
}

#[derive(Debug, Serialize)]
pub struct LaunchdPlist {
    pub launchd_data: Dictionary,
    pub plist_path: String,
}

#[derive(Debug, Serialize)]
pub struct LoginItemsData {
    /**Path to file to run */
    pub path: Vec<String>,
    /**Path represented as Catalog Node ID */
    pub cnid_path: Vec<i64>,
    /**Created timestamp of target file in UNIXEPOCH seconds */
    pub created: i64,
    /**Path to the volume of target file */
    pub volume_path: String,
    /**Target file URL type */
    pub volume_url: String,
    /**Name of volume target file is on */
    pub volume_name: String,
    /**Volume UUID */
    pub volume_uuid: String,
    /**Size of target volume in bytes */
    pub volume_size: i64,
    /**Created timestamp of volume in UNIXEPOCH seconds */
    pub volume_created: i64,
    /**Volume Property flags */
    pub volume_flag: Vec<u64>,
    /**Flag if volume if the root filesystem */
    pub volume_root: bool,
    /**Localized name of target file */
    pub localized_name: String,
    /**Read-Write security extension of target file */
    pub security_extension_rw: String,
    /**Read-Only security extension of target file */
    pub security_extension_ro: String,
    /**File property flags */
    pub target_flags: Vec<u64>,
    /**Username associated with `Bookmark` */
    pub username: String,
    /**Folder index number associated with target file */
    pub folder_index: i64,
    /**UID associated with `LoginItem` */
    pub uid: i32,
    /**`LoginItem` creation flags */
    pub creation_options: i32,
    /**Is `LoginItem` bundled in app */
    pub is_bundled: bool,
    /**App ID associated with `LoginItem` */
    pub app_id: String,
    /**App binary name */
    pub app_binary: String,
    /**Is target file executable */
    pub is_executable: bool,
    /**Does target file have file reference flag */
    pub file_ref_flag: bool,
    /**Path to `LoginItem` source */
    pub source_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct MachoInfo {
    pub cpu_type: String,
    pub cpu_subtype: String,
    pub filetype: String,
    pub segments: Vec<Segment64>,
    pub dylib_command: Vec<DylibCommand>,
    pub id: String,
    pub team_id: String,
    pub entitlements: Dictionary,
    pub certs: String,
    pub minos: String,
    pub sdk: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Segment64 {
    pub name: String,
    pub vmaddr: u64,
    pub vmsize: u64,
    pub file_offset: u64,
    pub file_size: u64,
    pub max_prot: u32,
    pub init_prot: u32,
    pub nsects: u32,
    pub flags: u32,
    pub sections: Vec<Section>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct DylibCommand {
    pub name: String,
    pub timestamp: u32,
    pub current_version: String,
    pub compatibility_version: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Section {
    pub section_name: String,
    pub segment_name: String,
    pub addr: u64,
    pub size: u64,
    pub offset: u32,
    pub align: u32,
    pub relocation_offset: u32,
    pub number_relocation_entries: u32,
    pub flags: u32,
    pub reserved: u32,
    pub reserved2: u32,
    pub reserved3: u32,
}

#[derive(Debug, Serialize)]
pub struct SpotlightEntries {
    pub inode: usize,
    pub parent_inode: usize,
    pub flags: u8,
    pub store_id: usize,
    pub last_updated: usize,
    pub values: HashMap<String, SpotlightValue>,
    pub directory: String,
}

#[derive(Debug, Serialize)]
pub struct SpotlightValue {
    pub attribute: DataAttribute,
    pub value: Value,
}

#[derive(Debug, PartialEq, Serialize, Clone, Deserialize)]
pub enum DataAttribute {
    AttrBool,
    AttrUnknown,
    AttrVariableSizeInt,
    AttrUnknown2,
    AttrUnknown3,
    AttrUnknown4,
    AttrVariableSizeInt2,
    AttrVariableSizeIntMultiValue,
    AttrByte,
    AttrFloat32,
    AttrFloat64,
    AttrString,
    AttrDate,
    AttrBinary,
    AttrList,
    Unknown,
}
