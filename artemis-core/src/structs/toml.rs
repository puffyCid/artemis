use crate::structs::artifacts::{
    os::{files::FileOptions, processes::ProcessOptions},
    runtime::script::JSScript,
};
use serde::Deserialize;

#[cfg(target_os = "windows")]
use crate::structs::artifacts::os::windows::{
    AmcacheOptions, BitsOptions, EventLogsOptions, JumplistsOptions, PrefetchOptions,
    RawFilesOptions, RecycleBinOptions, RegistryOptions, SearchOptions, ServicesOptions,
    ShellbagsOptions, ShimcacheOptions, ShimdbOptions, ShortcutOptions, SrumOptions, TasksOptions,
    UserAssistOptions, UserOptions, UsnJrnlOptions, WmiPersistOptions,
};

#[cfg(target_family = "unix")]
use super::artifacts::os::macos::{
    EmondOptions, ExecPolicyOptions, FseventsOptions, LaunchdOptions, LoginitemsOptions,
    MacosGroupsOptions, MacosSudoOptions, MacosUsersOptions, SpotlightOptions, UnifiedLogsOptions,
};

#[cfg(target_family = "unix")]
use super::artifacts::os::linux::{JournalOptions, LinuxSudoOptions, LogonOptions};

#[derive(Debug, Deserialize)]
pub struct ArtemisToml {
    pub system: String,
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
    pub filter_name: Option<String>,
    pub filter_script: Option<String>,
    pub url: Option<String>,
    pub api_key: Option<String>,
    pub logging: Option<String>,
}

#[derive(Debug, Deserialize)]
#[cfg(target_family = "unix")]
pub struct Artifacts {
    /**Based on artifact parse one of the artifact types */
    pub artifact_name: String,
    /**Specify whether to filter the parsed data */
    pub filter: Option<bool>,
    pub processes: Option<ProcessOptions>,
    pub files: Option<FileOptions>,
    pub unifiedlogs: Option<UnifiedLogsOptions>,
    pub script: Option<JSScript>,
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
}

#[derive(Debug, Deserialize)]
#[cfg(target_os = "windows")]
pub struct Artifacts {
    /**Based on artifact parse one of the artifact types */
    pub artifact_name: String,
    /**Specify whether to filter the parsed data */
    pub filter: Option<bool>,
    pub eventlogs: Option<EventLogsOptions>,
    pub prefetch: Option<PrefetchOptions>,
    pub processes: Option<ProcessOptions>,
    pub rawfiles: Option<RawFilesOptions>,
    pub files: Option<FileOptions>,
    pub shimdb: Option<ShimdbOptions>,
    pub registry: Option<RegistryOptions>,
    pub userassist: Option<UserAssistOptions>,
    pub shimcache: Option<ShimcacheOptions>,
    pub shellbags: Option<ShellbagsOptions>,
    pub amcache: Option<AmcacheOptions>,
    pub script: Option<JSScript>,
    pub shortcuts: Option<ShortcutOptions>,
    pub usnjrnl: Option<UsnJrnlOptions>,
    pub bits: Option<BitsOptions>,
    pub srum: Option<SrumOptions>,
    pub users: Option<UserOptions>,
    pub search: Option<SearchOptions>,
    pub tasks: Option<TasksOptions>,
    pub services: Option<ServicesOptions>,
    pub jumplists: Option<JumplistsOptions>,
    pub recyclebin: Option<RecycleBinOptions>,
    pub wmipersist: Option<WmiPersistOptions>,
}
