use super::artifacts::os::linux::{JournalOptions, LinuxSudoOptions, LogonOptions};
use super::artifacts::os::macos::{
    EmondOptions, ExecPolicyOptions, FseventsOptions, LaunchdOptions, LoginitemsOptions,
    MacosGroupsOptions, MacosSudoOptions, MacosUsersOptions, SpotlightOptions, UnifiedLogsOptions,
};
use super::artifacts::os::windows::{MftOptions, OutlookOptions};
use crate::structs::artifacts::os::linux::Ext4Options;
use crate::structs::artifacts::os::windows::{
    AmcacheOptions, BitsOptions, EventLogsOptions, JumplistsOptions, PrefetchOptions,
    RawFilesOptions, RecycleBinOptions, RegistryOptions, SearchOptions, ServicesOptions,
    ShellbagsOptions, ShimcacheOptions, ShimdbOptions, ShortcutOptions, SrumOptions, TasksOptions,
    UserAssistOptions, UsnJrnlOptions, WindowsUserOptions, WmiPersistOptions,
};
use crate::structs::artifacts::{
    os::{files::FileOptions, processes::ProcessOptions},
    runtime::script::JSScript,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ArtemisToml {
    pub output: Output,
    pub artifacts: Vec<Artifacts>,
}

#[derive(Debug, Deserialize)]
pub struct Output {
    pub name: String,
    pub endpoint_id: String,
    pub collection_id: u64,
    pub directory: String,
    pub output: String,
    pub format: String,
    pub compress: bool,
    pub timeline: bool,
    pub filter_name: Option<String>,
    pub filter_script: Option<String>,
    pub url: Option<String>,
    pub api_key: Option<String>,
    pub logging: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Artifacts {
    /**Based on artifact parse one of the artifact types */
    pub artifact_name: String,
    /**Specify whether to filter the parsed data */
    pub filter: Option<bool>,
    pub processes: Option<ProcessOptions>,
    pub files: Option<FileOptions>,
    pub unifiedlogs: Option<UnifiedLogsOptions>,
    pub users_macos: Option<MacosUsersOptions>,
    pub groups_macos: Option<MacosGroupsOptions>,
    pub emond: Option<EmondOptions>,
    pub execpolicy: Option<ExecPolicyOptions>,
    pub launchd: Option<LaunchdOptions>,
    pub loginitems: Option<LoginitemsOptions>,
    pub fseventsd: Option<FseventsOptions>,
    pub sudologs_macos: Option<MacosSudoOptions>,
    pub spotlight: Option<SpotlightOptions>,
    pub journals: Option<JournalOptions>,
    pub sudologs_linux: Option<LinuxSudoOptions>,
    pub logons: Option<LogonOptions>,
    pub rawfiles_ext4: Option<Ext4Options>,
    pub eventlogs: Option<EventLogsOptions>,
    pub prefetch: Option<PrefetchOptions>,
    pub rawfiles: Option<RawFilesOptions>,
    pub shimdb: Option<ShimdbOptions>,
    pub registry: Option<RegistryOptions>,
    pub userassist: Option<UserAssistOptions>,
    pub shimcache: Option<ShimcacheOptions>,
    pub shellbags: Option<ShellbagsOptions>,
    pub amcache: Option<AmcacheOptions>,
    pub shortcuts: Option<ShortcutOptions>,
    pub usnjrnl: Option<UsnJrnlOptions>,
    pub bits: Option<BitsOptions>,
    pub srum: Option<SrumOptions>,
    pub users_windows: Option<WindowsUserOptions>,
    pub search: Option<SearchOptions>,
    pub tasks: Option<TasksOptions>,
    pub services: Option<ServicesOptions>,
    pub jumplists: Option<JumplistsOptions>,
    pub recyclebin: Option<RecycleBinOptions>,
    pub wmipersist: Option<WmiPersistOptions>,
    pub outlook: Option<OutlookOptions>,
    pub mft: Option<MftOptions>,
    pub connections: Option<()>,

    // Scripts to run in BoaJS
    pub script: Option<JSScript>,
}

#[derive(Debug, Deserialize)]
pub struct Marker {
    /**Path to save marker file in */
    pub path: String,
    /**Name of the marker file */
    pub name: String,
    /**Age in minutes */
    pub age: u64,
}
