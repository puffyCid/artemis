use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::outlook::PropertyName;

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub last_logon: String,
    pub password_last_set: String,
    pub account_expires: String,
    pub last_password_failure: String,
    pub relative_id: u32,
    pub primary_group_id: u32,
    pub user_account_control_flags: Vec<UacFlags>,
    pub country_code: u16,
    pub code_page: u16,
    pub number_password_failures: u16,
    pub number_logons: u16,
    pub username: String,
    pub sid: String,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum UacFlags {
    AccountDisabled,
    HomeDirectoryRequired,
    PasswordNotRequired,
    TempDuplicateAccount,
    NormalAccount,
    MNSLogonAccount,
    InterdomainTrustAccount,
    WorkstationTrustAccount,
    ServerTrustAccount,
    DontExpirePassword,
    AccountAutoLocked,
    EncryptedTextPasswordAllowed,
    SmartcardRequired,
    TrustedForDelegation,
    NotDelegated,
    UseDESKeyOnly,
    DontRequirePreauth,
    PasswordExpired,
    TrustedToAuthenticateForDelegation,
    NoAuthDataRequired,
    PartialSecretsAccount,
    UseAESKeys,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct PeInfo {
    pub imports: Vec<String>,
    pub sections: Vec<String>,
    pub cert: String,
    pub pdb: String,
    pub product_version: String,
    pub file_version: String,
    pub product_name: String,
    pub company_name: String,
    pub file_description: String,
    pub internal_name: String,
    pub legal_copyright: String,
    pub original_filename: String,
    pub manifest: String,
    pub icons: Vec<String>,
}

/**
 * `Amcache` is just a Registry file with plaintext entries. No additional parsing is needed
 * Each entry contains PE metadata such as size, version, original filename, SHA1 (First ~31MB), publisher
 */
#[derive(Debug, Serialize)]
pub struct Amcache {
    pub last_modified: String,
    pub path: String,
    pub name: String,
    pub original_name: String,
    pub version: String,
    pub binary_type: String,
    pub product_version: String,
    pub product_name: String,
    pub language: String,
    pub file_id: String,
    pub link_date: String,
    pub path_hash: String,
    pub program_id: String,
    pub size: String,
    pub publisher: String,
    pub usn: String,
    pub sha1: String, // Only first ~31MBs
    pub reg_path: String,
    pub source_path: String,
}

#[derive(Debug, Serialize)]
pub struct WindowsBits {
    pub bits: Vec<BitsInfo>,
    pub carved_jobs: Vec<JobInfo>,
    pub carved_files: Vec<FileInfo>,
}

#[derive(Debug, Serialize)]
pub struct BitsInfo {
    pub job_id: String,
    pub file_id: String,
    pub owner_sid: String,
    pub created: String,
    pub modified: String,
    pub completed: String,
    pub expiration: String,
    pub files_total: u32,
    pub bytes_downloaded: u64,
    pub bytes_transferred: u64,
    pub job_name: String,
    pub job_description: String,
    pub job_command: String,
    pub job_arguments: String,
    pub error_count: u32,
    pub job_type: JobType,
    pub job_state: JobState,
    pub priority: JobPriority,
    pub flags: JobFlags,
    pub http_method: String,
    pub full_path: String,
    pub filename: String,
    pub target_path: String,
    pub tmp_file: String,
    pub volume: String,
    pub url: String,
    pub carved: bool,
    pub transient_error_count: u32,
    pub acls: Vec<AccessControlEntry>,
    pub timeout: u32,
    pub retry_delay: u32,
    pub additional_sids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct FileInfo {
    pub file_id: String,
    pub filename: String,
    pub full_path: String,
    pub tmp_fullpath: String,
    pub drive: String,
    pub volume: String,
    pub url: String,
    pub download_bytes_size: u64,
    pub transfer_bytes_size: u64,
    pub files_transferred: u32,
}

#[derive(Debug, Serialize)]
pub struct JobInfo {
    pub job_id: String,
    pub file_id: String,
    pub owner_sid: String,
    pub created: String,
    pub modified: String,
    pub completed: String,
    pub expiration: String,
    pub job_name: String,
    pub job_description: String,
    pub job_command: String,
    pub job_arguments: String,
    pub error_count: u32,
    pub transient_error_count: u32,
    pub job_type: JobType,
    pub job_state: JobState,
    pub priority: JobPriority,
    pub flags: JobFlags,
    pub http_method: String,
    pub acls: Vec<AccessControlEntry>,
    pub additional_sids: Vec<String>,
    pub timeout: u32,
    pub retry_delay: u32,
    pub target_path: String,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub enum JobState {
    Queued,
    Connecting,
    Transferring,
    Suspended,
    Error,
    TransientError,
    Transferred,
    Acknowledged,
    Cancelled,
    Unknown,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub enum JobPriority {
    Foreground,
    High,
    Normal,
    Low,
    Unknown,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub enum JobType {
    Download,
    Upload,
    UploadReply,
    Unknown,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub enum JobFlags {
    Transferred,
    Error,
    TransferredBackgroundError,
    Disable,
    TransferredBackgroundDisable,
    ErrorBackgroundDisable,
    TransferredBackgroundErrorDisable,
    Modification,
    FileTransferred,
    Unknown,
}

#[derive(Debug, PartialEq, Serialize, Clone)]
pub struct AccessControlEntry {
    pub ace_type: AceTypes,
    pub flags: Vec<AceFlags>,
    pub access_rights: Vec<AccessMask>,
    pub sid: String,
    pub account: String,
    /**Only if Object data_type and ACE_OBJECT_TYPE_PRESENT object flag */
    pub object_flags: ObjectFlag,
    /**Only if Object data_type and ACE_INHERITED_OBJECT_TYPE_PRESENT object flag */
    pub object_type_guid: String,
    pub inherited_object_type_guid: String,
}

#[derive(Debug, PartialEq, Serialize, Clone)]
pub enum AceTypes {
    AccessAllowedAceType,
    AccessDeniedAceType,
    SystemAuditAceType,
    SystemAlarmAceType,
    Reserved,
    AccessAllowedObjectType,
    AccessDeniedObjectType,
    SystemAuditObjectType,
    SystemAlarmObjectType,
    AccessAllowedAceTypeCallback,
    AccessDeniedAceTypeCallback,
    SystemAuditAceTypeCallback,
    SystemAlarmAceTypeCallback,
    AccessAllowedObjectTypeCallback,
    AccessDeniedObjectTypeCallback,
    SystemAuditObjectTypeCallback,
    SystemAlarmObjectTypeCallback,
    SystemMandatoryLabel,
    Unknown,
    Ace,
    Object,
}

#[derive(Debug, PartialEq, Serialize, Clone)]
pub enum ObjectFlag {
    ObjectType,
    InheritedObjectType,
    None,
}

#[derive(Debug, PartialEq, Serialize, Clone)]
pub enum AceFlags {
    ObjectInherit,
    ContainerInherit,
    NoPropagateInherit,
    InheritOnly,
    SuccessfulAccess,
    FailedAccess,
}

#[derive(Debug, PartialEq, Serialize, Clone)]
pub enum AccessItem {
    Folder,
    NonFolder,
    Mandatory,
    Registry,
}

#[derive(Debug, PartialEq, Serialize, Clone)]
pub enum AccessMask {
    Delete,
    ReadControl,
    WriteDac,
    WriteOwner,
    Synchronize,
    _AccessSystemSecurity,
    _MaximumAllowed,
    GenericAll,
    GenericExecute,
    GenericWrite,
    GenericRead,
    FileReadData,
    FileWriteData,
    FileReadEa,
    FileWriteEa,
    FileExecute,
    FileReadAttributes,
    FileWriteAttributes,
    AppendMsg,
    FileListDirectory,
    FileAddFile,
    FileAddSubdirectory,
    MandatoryNoWriteUp,
    MandatoryNoReadUp,
    MandatoryNoExecuteUp,
    // Registry related. https://learn.microsoft.com/en-us/windows/win32/sysinfo/registry-key-security-and-access-rights
    AllAccess,
    CreateLink,
    CreateSubKey,
    EnumerateSubKeys,
    Execute,
    Notify,
    QueryValue,
    Read,
    SetValue,
    Wow64Key32,
    Wow64Key64,
    Write,
}

/**
 * A simple abstracted table dump from the ESE database  
 * Will auto parse non-binary column types
 */
#[derive(Debug, Clone, Serialize)]
pub struct TableDump {
    /**The column type. Ex: GUID, binary, text, bit, long, etc */
    pub column_type: ColumnType,
    /**Name of the column */
    pub column_name: String,
    /**Column data as a string. Empty columns have empty strings. Binary data is base64 encoded */
    pub column_data: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ColumnType {
    Nil,
    Bit,
    UnsignedByte,
    Short,
    Long,
    Currency,
    Float32,
    Float64,
    DateTime,
    Binary,
    /**Can be ASCII OR Unicode */
    Text,
    LongBinary,
    /**Can be ASCII or Unicode */
    LongText,
    /**No longer used */
    SuperLong,
    UnsignedLong,
    LongLong,
    Guid,
    UnsignedShort,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EventLogRecord {
    pub event_record_id: u64,
    pub timestamp: String,
    pub data: Value,
}

#[derive(Debug, Serialize)]
pub struct JumplistEntry {
    pub lnk_info: ShortcutInfo,
    pub path: String,
    pub jumplist_type: ListType,
    pub app_id: String,
    /**Only applicable for Automatic Jumplists */
    pub jumplist_metadata: DestEntries,
}

#[derive(Debug, Clone, Serialize)]
pub struct DestEntries {
    pub droid_volume_id: String,
    pub droid_file_id: String,
    pub birth_droid_volume_id: String,
    pub birth_droid_file_id: String,
    pub hostname: String,
    pub entry: u32,
    pub modified: String,
    pub pin_status: PinStatus,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
pub enum PinStatus {
    Pinned,
    NotPinned,
    None,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum ListType {
    Automatic,
    Custom,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct ShortcutInfo {
    pub source_path: String,
    pub data_flags: Vec<DataFlags>,
    pub attribute_flags: Vec<AttributeFlags>,
    pub created: String,
    pub modified: String,
    pub accessed: String,
    pub file_size: u32,
    pub location_flags: LocationFlag,
    pub path: String,
    pub drive_serial: String,
    pub drive_type: DriveType,
    pub volume_label: String,
    pub network_provider: NetworkProviderType,
    pub network_share_name: String,
    pub network_device_name: String,
    pub description: String,
    pub relative_path: String,
    pub working_directory: String,
    pub command_line_args: String,
    pub icon_location: String,
    pub hostname: String,
    pub droid_volume_id: String,
    pub droid_file_id: String,
    pub birth_droid_volume_id: String,
    pub birth_droid_file_id: String,
    pub shellitems: Vec<ShellItem>,
    pub properties: Vec<HashMap<String, Value>>,
    pub environment_variable: String,
    pub console: Vec<Console>,
    pub codepage: u32,
    pub special_folder_id: u32,
    pub darwin_id: String,
    pub shim_layer: String,
    pub known_folder: String,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum NetworkProviderType {
    WnncNetAvid,
    WnncNetDocuspace,
    WnncNetMangsoft,
    WnncNetSernet,
    WnncNetRiverFront1,
    WnncNetRiverFront2,
    WnncNetDecorb,
    WnncNetProtstor,
    WnncNetFjRedir,
    WnncNetDistinct,
    WnncNetTwins,
    WnncNetRdr2Sample,
    WnncNetCsc,
    WnncNet3In1,
    WnncNetExtendNet,
    WnncNetStac,
    WnncNetFoxbat,
    WnncNetYahoo,
    WnncNetExifs,
    WnncNetDav,
    WnncNetKnoware,
    WnncNetObjectDire,
    WnncNetMasfax,
    WnncNetHobNfs,
    WnncNetShiva,
    WnncNetIbmal,
    WnncNetLock,
    WnncNetTermsrv,
    WnncNetSrt,
    WnncNetQuincy,
    WnncNetOpenafs,
    WnncNetAvid1,
    WnncNetDfs,
    WnncNetKwnp,
    WnncNetZenworks,
    WnncNetDriveOnWeb,
    WnncNetVmware,
    WnncNetRsfx,
    WnncNetMfiles,
    WnncNetMsNfs,
    WnncNetGoogle,
    Unknown,
    None,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum LocationFlag {
    VolumeIDAndLocalBasePath,
    CommonNetworkRelativeLinkAndPathSuffix,
    None,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum DriveType {
    DriveUnknown,
    DriveNotRootDir,
    DriveRemovable,
    DriveFixed,
    DriveRemote,
    DriveCdrom,
    DriveRamdisk,
    None,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Console {
    pub color_flags: Vec<ColorFlags>,
    pub popup_fill_attributes: Vec<ColorFlags>,
    pub screen_width_buffer_size: u16,
    pub screen_height_buffer_size: u16,
    pub window_width: u16,
    pub window_height: u16,
    pub window_x_coordinate: u16,
    pub window_y_coordinate: u16,
    pub font_size: u16,
    pub font_family: FontFamily,
    pub font_weight: FontWeight,
    pub face_name: String,
    pub cursor_size: CursorSize,
    pub full_screen: u32,
    pub insert_mode: u32,
    pub automatic_position: u32,
    pub history_buffer_size: u32,
    pub number_history_buffers: u32,
    pub duplicates_allowed_history: u32,
    pub color_table: String,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum ColorFlags {
    ForegroundBlue,
    ForegroundGreen,
    ForegroundRed,
    ForegroundIntensity,
    BackgroundBlue,
    BackgroundGreen,
    BackgroundRed,
    BackgroundIntensity,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum FontFamily {
    DontCare,
    Roman,
    Swiss,
    Modern,
    Script,
    Decorative,
    Unknown,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum FontWeight {
    Regular,
    Bold,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum CursorSize {
    Small,
    Normal,
    Large,
    Unknown,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum DataFlags {
    HasTargetIdList,
    HasLinkInfo,
    HasName,
    HasRelativePath,
    HasWorkingDirectory,
    HasArguements,
    HasIconLocation,
    IsUnicode,
    ForceNoLinkInfo,
    HasExpString,
    RunInSeparateProcess,
    HasDarwinId,
    RunAsUser,
    HasExpIcon,
    NoPidAlias,
    RunWithShimLayer,
    ForceNoLinkTrack,
    EnableTargetMetadata,
    DisableLinkPathTracking,
    DisableKnownFolderTracking,
    DisableKnownFolderAlias,
    AllowLinkToLink,
    UnaliasOnSave,
    PreferEnvironmentPath,
    KeepLocalDListForUncTarget,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum AttributeFlags {
    ReadOnly,
    Hidden,
    System,
    Directory,
    Archive,
    Device,
    Normal,
    Temporary,
    SparseFile,
    ReparsePoint,
    Compressed,
    Offline,
    NotConentIndexed,
    Encrypted,
    Virtual,
}

/**  Return a `ShellItem` structure containing
 * value: Generic value of the `ShellItem`, can be a directory, file, URI, or GUID
 * created: FAT timestamp, only found on directory, file `ShellItems`
 * accessed: FAT timestamp, only found on directory, file `ShellItems`
 * modified: FAT timestamp, only found on directory, file `ShellItems`
 * `mft_entry`: The MFT entry for a file or directory `ShellItem`
 * `mft_sequence`: The MFT sequence for a file or directory `ShellItem`
*/
#[derive(Debug, PartialEq, Serialize)]
pub struct ShellItem {
    pub value: String,
    pub shell_type: ShellType,
    /**FAT time */
    pub created: String,
    /**FAT time */
    pub modified: String,
    /**FAT time */
    pub accessed: String,
    pub mft_entry: u64,
    pub mft_sequence: u16,
    pub stores: Vec<HashMap<String, Value>>,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
pub enum ShellType {
    Directory, // After applying bitwise AND 0x70
    Network,   // After applying bitwise AND 0x70
    Volume,    // After apply bitwise AND 0x70
    RootFolder,
    ControlPanel,
    ControlPanelEntry,
    UserPropertyView, // Can have the same id as RootFolder, but its much larger
    Delegate,         // Similar to File type
    Uri,
    Variable,
    Mtp,
    Unknown,
    History,
    GameFolder,
    _Optical, // No optical drives available to test on.
}

#[derive(Debug, Serialize, Clone)]
pub struct RawFilelist {
    pub full_path: String,
    pub directory: String,
    pub filename: String,
    pub extension: String,
    pub created: String,
    pub modified: String,
    pub changed: String,
    pub accessed: String,
    pub filename_created: String,
    pub filename_modified: String,
    pub filename_changed: String,
    pub filename_accessed: String,
    pub size: u64,
    pub compressed_size: u64,
    pub compression_type: CompressionType,
    pub inode: u64,
    pub sequence_number: u16,
    pub parent_mft_reference: u64,
    pub owner: u32,
    pub attributes: Vec<String>,
    pub md5: String,
    pub sha1: String,
    pub sha256: String,
    pub is_file: bool,
    pub is_directory: bool,
    pub is_indx: bool,
    pub depth: usize,
    pub usn: u64,
    pub sid: u32,
    pub user_sid: String,
    pub group_sid: String,
    pub drive: String,
    pub ads_info: Vec<ADSInfo>,
    pub pe_info: Vec<PeInfo>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ADSInfo {
    pub name: String,
    pub size: u64,
}

#[derive(Debug, Serialize, Clone)]
pub enum CompressionType {
    NTFSCompressed,
    WofCompressed,
    None,
}

#[derive(Debug, Serialize)]
pub struct Prefetch {
    pub path: String,
    pub filename: String,
    pub hash: String,
    pub last_run_time: String,
    pub all_run_times: Vec<String>,
    pub run_count: u32,
    pub size: u32,
    pub volume_serial: Vec<String>,
    pub volume_creation: Vec<String>,
    pub volume_path: Vec<String>,
    pub accessed_files_count: u32,
    pub accessed_directories_count: u32,
    pub accessed_files: Vec<String>,
    pub accessed_directories: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct RecycleBin {
    pub size: u64,
    pub deleted: String,
    pub filename: String,
    pub full_path: String,
    pub directory: String,
    pub sid: String,
    pub recycle_path: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct RegistryData {
    pub path: String, // ROOT\...\CurrentVersion\Run
    pub key: String,  // ROOT\...\CurrentVersion
    pub name: String, // Run key
    pub values: Vec<KeyValue>,
    pub last_modified: String,
    pub depth: usize,
    pub security_offset: i32,
    pub registry_path: String,
    pub registry_file: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct KeyValue {
    pub value: String,     // Run key => Value: Vmware
    pub data: String,      // C:\vmware.exe
    pub data_type: String, // REG_WORD, REG_DWORD
}

#[derive(Debug, Serialize)]
pub struct ServicesData {
    pub state: ServiceState,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub start_mode: StartMode,
    pub path: String,
    pub service_type: Vec<ServiceType>,
    pub account: String,
    pub modified: String,
    pub service_dll: String,
    pub failure_command: String,
    pub reset_period: u32,
    pub failure_actions: Vec<FailureActions>,
    pub required_privileges: Vec<String>,
    pub error_control: ServiceError,
    pub reg_path: String,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum StartMode {
    Automatic,
    Boot,
    Disabled,
    Manual,
    System,
    Unknown,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum ServiceState {
    Stopped,
    StartPending,
    StopPending,
    Running,
    ContinuePending,
    PausePending,
    Paused,
    Unknown,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum ServiceError {
    Ignore,
    Normal,
    Severe,
    Critical,
    Unknown,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum ServiceType {
    Adapter,
    FileSystemDriver,
    InteractiveProcess,
    KernelDriver,
    RecognizeDriver,
    Win32OwnProcess,
    Win32SharedProcess,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct FailureActions {
    pub action: Action,
    pub delay: u32,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum Action {
    None,
    Reboot,
    Restart,
    RunCommand,
    Unknown,
}

#[derive(Debug, Serialize)]
pub struct ShimcacheEntry {
    pub entry: u32,
    pub path: String,
    pub last_modified: String,
    pub key_path: String,
    pub source_path: String,
}

#[derive(Debug, Serialize)]
pub struct ShimData {
    pub indexes: Vec<TagData>,
    pub db_data: DatabaseData,
    pub sdb_path: String,
}

#[derive(Debug, Serialize)]
pub struct DatabaseData {
    pub sdb_version: String,
    pub compile_time: String,
    pub compiler_version: String,
    pub name: String,
    pub platform: u32,
    pub database_id: String,
    pub additional_metadata: HashMap<String, String>,
    pub list_data: Vec<TagData>,
}

#[derive(Debug, Serialize)]
pub struct TagData {
    pub data: HashMap<String, String>, //key: TAG_SHIM_TAGID, value: "0x11", binary: base64, string
    pub list_data: Vec<HashMap<String, String>>,
}

#[derive(Debug, Serialize)]
pub struct ApplicationInfo {
    pub auto_inc_id: i32,
    pub timestamp: String,
    pub app_id: String,
    pub user_id: String,
    pub foreground_cycle_time: i64,
    pub background_cycle_time: i64,
    pub facetime: i64,
    pub foreground_context_switches: i32,
    pub background_context_switches: i32,
    pub foreground_bytes_read: i64,
    pub foreground_bytes_written: i64,
    pub foreground_num_read_operations: i32,
    pub foreground_num_write_options: i32,
    pub foreground_number_of_flushes: i32,
    pub background_bytes_read: i64,
    pub background_bytes_written: i64,
    pub background_num_read_operations: i32,
    pub background_num_write_operations: i32,
    pub background_number_of_flushes: i32,
}

#[derive(Debug, Serialize)]
pub struct AppTimelineInfo {
    pub auto_inc_id: i32,
    pub timestamp: String,
    pub app_id: String,
    pub user_id: String,
    pub flags: i32,
    pub end_time: String,
    pub duration_ms: i32,
    pub span_ms: i32,
    pub timeline_end: i32,
    pub in_focus_timeline: i64,
    pub user_input_timeline: i64,
    pub comp_rendered_timeline: i64,
    pub comp_dirtied_timeline: i64,
    pub comp_propagated_timeline: i64,
    pub audio_in_timeline: i64,
    pub audio_out_timeline: i64,
    pub cpu_timeline: i64,
    pub disk_timeline: i64,
    pub network_timeline: i64,
    pub mbb_timeline: i64,
    pub in_focus_s: i32,
    pub psm_foreground_s: i32,
    pub user_input_s: i32,
    pub comp_rendered_s: i32,
    pub comp_dirtied_s: i32,
    pub comp_propagated_s: i32,
    pub audio_in_s: i32,
    pub audio_out_s: i32,
    pub cycles: i64,
    pub cycles_breakdown: i64,
    pub cycles_attr: i64,
    pub cycles_attr_breakdown: i64,
    pub cycles_wob: i64,
    pub cycles_wob_breakdown: i64,
    pub disk_raw: i64,
    pub network_tail_raw: i64,
    pub network_bytes_raw: i64,
    pub mbb_tail_raw: i64,
    pub mbb_bytes_raw: i64,
    pub display_required_s: i64,
    pub display_required_timeline: i64,
    pub keyboard_input_timeline: i64,
    pub keyboard_input_s: i32,
    pub mouse_input_s: i32,
}

#[derive(Debug, Serialize)]
pub struct AppVfu {
    pub auto_inc_id: i32,
    pub timestamp: String,
    pub app_id: String,
    pub user_id: String,
    pub flags: i32,
    pub start_time: String,
    pub end_time: String,
    pub usage: String,
}

#[derive(Debug, Serialize)]
pub struct EnergyInfo {
    pub auto_inc_id: i32,
    pub timestamp: String,
    pub app_id: String,
    pub user_id: String,
    pub binary_data: String,
}

#[derive(Debug, Serialize)]
pub struct EnergyUsage {
    pub auto_inc_id: i32,
    pub timestamp: String,
    pub app_id: String,
    pub user_id: String,
    pub event_timestamp: String,
    pub state_transition: i32,
    pub full_charged_capacity: i32,
    pub designed_capacity: i32,
    pub charge_level: i32,
    pub cycle_count: i32,
    pub configuration_hash: i64,
}

#[derive(Debug, Serialize)]
pub struct NetworkInfo {
    pub auto_inc_id: i32,
    pub timestamp: String,
    pub app_id: String,
    pub user_id: String,
    pub interface_luid: i64,
    pub l2_profile_id: i64,
    pub l2_profile_flags: i32,
    pub bytes_sent: i64,
    pub bytes_recvd: i64,
}

#[derive(Debug, Serialize)]
pub struct NetworkConnectivityInfo {
    pub auto_inc_id: i32,
    pub timestamp: String,
    pub app_id: String,
    pub user_id: String,
    pub interface_luid: i64,
    pub l2_profile_id: i64,
    pub connected_time: i32,
    pub connect_start_time: String,
    pub l2_profile_flags: i32,
}

#[derive(Debug, Serialize)]
pub struct NotificationInfo {
    pub auto_inc_id: i32,
    pub timestamp: String,
    pub app_id: String,
    pub user_id: String,
    pub notification_type: i32,
    pub payload_size: i32,
    pub network_type: i32,
}

#[derive(Serialize)]
pub struct TaskData {
    pub tasks: Vec<TaskXml>,
    pub jobs: Vec<TaskJob>,
}
/**
 * Structure of a XML format Schedule Task
 * Schema at: [Task XML](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-tsch/0d6383e4-de92-43e7-b0bb-a60cfa36379f)
 */
#[derive(Debug, Serialize)]
pub struct TaskXml {
    pub registration_info: Option<RegistrationInfo>,
    pub triggers: Option<Triggers>,
    pub settings: Option<Settings>,
    /**Arbitrary data, we base64 encode the data */
    pub data: Option<String>,
    pub principals: Option<Vec<Principals>>,
    pub actions: Actions,
    pub path: String,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Actions {
    pub exec: Vec<ExecType>,
    pub com_handler: Vec<ComHandlerType>,
    pub send_email: Vec<SendEmail>,
    pub show_message: Vec<Message>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct ExecType {
    pub command: String,
    pub arguments: Option<String>,
    pub working_directory: Option<String>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct ComHandlerType {
    pub class_id: String,
    pub data: Option<String>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct SendEmail {
    pub server: Option<String>,
    pub subject: Option<String>,
    pub to: Option<String>,
    pub cc: Option<String>,
    pub bcc: Option<String>,
    pub reply_to: Option<String>,
    pub from: String,
    pub header_fields: Option<HashMap<String, String>>,
    pub body: Option<String>,
    pub attachment: Option<Vec<String>>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct Message {
    pub title: Option<String>,
    pub body: String,
}

#[derive(Debug, PartialEq, Serialize, Clone)]
pub struct Principals {
    pub user_id: Option<String>,
    pub logon_type: Option<String>,
    pub group_id: Option<String>,
    pub display_name: Option<String>,
    pub run_level: Option<String>,
    pub process_token_sid_type: Option<String>,
    pub required_privileges: Option<Vec<String>>,
    pub id_attribute: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RegistrationInfo {
    pub uri: Option<String>,
    pub sid: Option<String>,
    pub source: Option<String>,
    pub date: Option<String>,
    pub author: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub documentation: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Settings {
    pub allow_start_on_demand: Option<bool>,
    pub restart_on_failure: Option<RestartType>,
    pub multiple_instances_policy: Option<String>,
    pub disallow_start_if_on_batteries: Option<bool>,
    pub stop_if_going_on_batteries: Option<bool>,
    pub allow_hard_terminate: Option<bool>,
    pub start_when_available: Option<bool>,
    pub network_profile_name: Option<String>,
    pub run_only_if_network_available: Option<bool>,
    pub wake_to_run: Option<bool>,
    pub enabled: Option<bool>,
    pub hidden: Option<bool>,
    pub delete_expired_tasks_after: Option<String>,
    pub idle_settings: Option<IdleSettings>,
    pub network_settings: Option<NetworkSettings>,
    pub execution_time_limit: Option<String>,
    pub priority: Option<u8>,
    pub run_only_if_idle: Option<bool>,
    pub use_unified_scheduling_engine: Option<bool>,
    pub disallow_start_on_remote_app_session: Option<bool>,
    pub maintenance_settings: Option<MaintenanceSettings>,
    pub volatile: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct RestartType {
    pub interval: String,
    pub count: u16,
}

#[derive(Debug, Serialize)]
pub struct IdleSettings {
    pub duration: Option<String>,
    pub wait_timeout: Option<String>,
    pub stop_on_idle_end: Option<bool>,
    pub restart_on_idle: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct NetworkSettings {
    pub name: Option<String>,
    pub id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MaintenanceSettings {
    pub period: String,
    pub deadline: Option<String>,
    pub exclusive: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct Triggers {
    pub boot: Vec<BootTrigger>,
    pub registration: Vec<BootTrigger>,
    pub idle: Vec<IdleTrigger>,
    pub time: Vec<TimeTrigger>,
    pub event: Vec<EventTrigger>,
    pub logon: Vec<LogonTrigger>,
    pub session: Vec<SessionTrigger>,
    pub calendar: Vec<CalendarTrigger>,
    pub wnf: Vec<WnfTrigger>,
}

#[derive(Debug, Serialize)]
pub struct BaseTriggers {
    pub id: Option<String>,
    pub start_boundary: Option<String>,
    pub end_boundary: Option<String>,
    pub enabled: Option<bool>,
    pub execution_time_limit: Option<String>,
    pub repetition: Option<Repetition>,
}

#[derive(Debug, Serialize)]
pub struct Repetition {
    pub interval: String,
    pub duration: Option<String>,
    pub stop_at_duration_end: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct BootTrigger {
    pub common: Option<BaseTriggers>,
    pub delay: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct IdleTrigger {
    pub common: Option<BaseTriggers>,
}

#[derive(Debug, Serialize)]
pub struct TimeTrigger {
    pub common: Option<BaseTriggers>,
    pub random_delay: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EventTrigger {
    pub common: Option<BaseTriggers>,
    pub subscription: Vec<String>,
    pub delay: Option<String>,
    pub number_of_occurrences: Option<u8>,
    pub period_of_occurrence: Option<String>,
    pub matching_element: Option<String>,
    pub value_queries: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct LogonTrigger {
    pub common: Option<BaseTriggers>,
    pub user_id: Option<String>,
    pub delay: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SessionTrigger {
    pub common: Option<BaseTriggers>,
    pub user_id: Option<String>,
    pub delay: Option<String>,
    pub state_change: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WnfTrigger {
    pub common: Option<BaseTriggers>,
    pub state_name: String,
    pub delay: Option<String>,
    pub data: Option<String>,
    pub data_offset: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CalendarTrigger {
    pub common: Option<BaseTriggers>,
    pub random_delay: Option<String>,
    pub schedule_by_day: Option<ByDay>,
    pub schedule_by_week: Option<ByWeek>,
    pub schedule_by_month: Option<ByMonth>,
    pub schedule_by_month_day_of_week: Option<ByMonthDayWeek>,
}

#[derive(Debug, Serialize)]
pub struct ByDay {
    pub days_interval: Option<u16>,
}

#[derive(Debug, Serialize)]
pub struct ByWeek {
    pub weeks_interval: Option<u8>,
    pub days_of_week: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct ByMonth {
    pub days_of_month: Option<Vec<String>>,
    pub months: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct ByMonthDayWeek {
    pub weeks: Option<Vec<String>>,
    pub days_of_week: Option<Vec<String>>,
    pub months: Option<Vec<String>>,
}

/**
 * The old Windows Task format. Disabled on Windows 8 and higher. But can be enabled via Registry
 * Format at: [libyal](https://github.com/libyal/dtformats/blob/main/documentation/Job%20file%20format.asciidoc)
 */
#[derive(Serialize)]
pub struct TaskJob {
    pub job_id: String,
    pub error_retry_count: u16,
    pub error_retry_interval: u16,
    pub idle_deadline: u16,
    pub idle_wait: u16,
    pub priority: Priority,
    pub max_run_time: u32,
    pub exit_code: u32,
    pub status: Status,
    pub flags: Vec<Flags>,
    pub system_time: String,
    pub running_instance_count: u16,
    pub application_name: String,
    pub parameters: String,
    pub working_directory: String,
    pub author: String,
    pub comments: String,
    pub user_data: String,
    pub start_error: u32,
    pub triggers: Vec<VarTriggers>,
    pub path: String,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum Priority {
    Normal,
    High,
    Idle,
    Realtime,
    Unknown,
}

/// Additional status codes at [Microsoft](https://learn.microsoft.com/en-us/windows/win32/taskschd/task-scheduler-error-and-success-constants)
#[derive(Debug, PartialEq, Serialize)]
pub enum Status {
    Ready,
    Running,
    Disabled,
    HasNotRun,
    NoMoreRuns,
    NotScheduled,
    Terminated,
    NoValidTriggers,
    SomeTriggersFailed,
    BatchLogonProblem,
    Queued,
    Unknown,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum Flags {
    Interactive,
    DeleteWhenDone,
    Disabled,
    KillOnIdleEnd,
    StartOnlyIfIdle,
    DontStartIfOnBatteries,
    KillIfGoingOnBatteries,
    RunOnlyIfDocked,
    Hidden,
    RunIfConnectedToInternet,
    RestartOnIdleResume,
    SystemRequired,
    RunOnlyIfLoggedOn,
    ApplicationName,
}

#[derive(Debug, Serialize)]
pub struct VarTriggers {
    pub start_date: String,
    pub end_date: String,
    pub start_time: String,
    pub duration: u32,
    pub interval_mins: u32,
    pub flags: Vec<TriggerFlags>,
    pub types: Vec<TriggerTypes>,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum TriggerFlags {
    HasEndDate,
    KillAtDurationEnd,
    Disabled,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum TriggerTypes {
    Once,
    Daily,
    Weekly,
    MonthlyDate,
    MonthlyDow,
    EventOnIdle,
    EventAtSystemstart,
    EventAtLogon,
}

#[derive(Debug, Serialize)]
pub struct UserAssistEntry {
    pub path: String,
    pub last_execution: String,
    pub count: u32,
    pub reg_path: String,
    pub rot_path: String,
    pub folder_path: String,
}

#[derive(Serialize)]
pub struct UsnJrnlEntry {
    pub mft_entry: u64,
    pub mft_sequence: u16,
    pub parent_mft_entry: u64,
    pub parent_mft_sequence: u16,
    pub update_sequence_number: u64,
    pub update_time: String,
    pub update_reason: Vec<Reason>,
    pub update_source_flags: Source,
    pub security_descriptor_id: u32,
    pub file_attributes: Vec<AttributeFlags>,
    pub filename: String,
    pub extension: String,
    pub full_path: String,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum Reason {
    Overwrite,
    Extend,
    Truncation,
    NamedOverwrite,
    NamedExtend,
    NamedTruncation,
    FileCreate,
    FileDelete,
    EAChange,
    SecurityChange,
    RenameOldName,
    RenameNewName,
    IndexableChange,
    BasicInfoChange,
    HardLinkChange,
    CompressionChange,
    EncryptionChange,
    ObjectIDChange,
    ReparsePointChange,
    StreamChange,
    TransactedChange,
    Close,
}

#[derive(Debug, PartialEq, Serialize)]
pub enum Source {
    DataManagement,
    AuxiliaryData,
    ReplicationManagement,
    None,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct WmiPersist {
    pub class: String,
    pub values: BTreeMap<String, Value>,
    pub query: String,
    pub sid: String,
    pub filter: String,
    pub consumer: String,
    pub consumer_name: String,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct OutlookMessage {
    pub body: String,
    pub subject: String,
    pub from: String,
    pub recipient: String,
    pub delivered: String,
    pub recipients: HashSet<String>,
    pub attachments: Vec<OutlookAttachment>,
    pub properties: Vec<PropertyContext>,
    pub folder_path: String,
    pub source_file: String,
    pub yara_hits: Vec<String>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutlookAttachment {
    pub name: String,
    pub size: u64,
    pub method: String,
    pub mime: String,
    pub extension: String,
    pub data: String,
    pub properties: Vec<PropertyContext>,
}

/// Property Context Table (also called 0xbc table)
#[derive(Debug, PartialEq, Eq, Serialize, Clone, Deserialize)]
pub struct PropertyContext {
    pub name: Vec<PropertyName>,
    pub property_type: PropertyType,
    pub prop_id: u16,
    pub property_number: u16,
    pub reference: u32,
    pub value: Value,
}

#[derive(Debug, PartialEq, Eq, Serialize, Clone, Deserialize)]
pub enum PropertyType {
    Int16,
    Int32,
    Float32,
    Float64,
    Currency,
    FloatTime,
    ErrorCode,
    Bool,
    Int64,
    String,
    String8,
    Time,
    Guid,
    ServerId,
    Restriction,
    Binary,
    MultiInt16,
    MultiInt32,
    MultiFloat32,
    MultiFloat64,
    MultiCurrency,
    MultiFloatTime,
    MultiInt64,
    MultiString,
    MultiString8,
    MultiTime,
    MultiGuid,
    MultiBinary,
    Unspecified,
    Null,
    Object,
    RuleAction,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EventMessage {
    pub message: String,
    pub template_message: String,
    pub raw_event_data: Value,
    pub event_id: u64,
    pub qualifier: u64,
    pub version: u64,
    pub guid: String,
    pub provider: String,
    pub source_name: String,
    pub record_id: u64,
    pub task: u64,
    pub level: EventLevel,
    pub opcode: u64,
    pub keywords: String,
    pub generated: String,
    pub system_time: String,
    pub activity_id: String,
    pub process_id: u64,
    pub thread_id: u64,
    pub sid: String,
    pub channel: String,
    pub computer: String,
    pub source_file: String,
    pub message_file: String,
    pub parameter_file: String,
    pub registry_file: String,
    pub registry_path: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum EventLevel {
    Information,
    Warning,
    Critical,
    Verbose,
    Error,
    Unknown,
}
