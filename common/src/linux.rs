use std::collections::HashMap;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ElfInfo {
    pub symbols: Vec<String>,
    pub sections: Vec<String>,
    pub machine_type: String,
}

#[derive(Debug, Serialize)]
pub struct Journal {
    pub uid: u32,
    pub gid: u32,
    pub pid: usize,
    pub comm: String,
    pub priority: Priority,
    pub syslog_facility: Facility,
    pub thread_id: usize,
    pub syslog_identifier: String,
    pub executable: String,
    pub cmdline: String,
    pub cap_effective: String,
    pub audit_session: usize,
    pub audit_loginuid: u32,
    pub systemd_cgroup: String,
    pub systemd_owner_uid: usize,
    pub systemd_unit: String,
    pub systemd_user_unit: String,
    pub systemd_slice: String,
    pub systemd_user_slice: String,
    pub systemd_invocation_id: String,
    pub boot_id: String,
    pub machine_id: String,
    pub hostname: String,
    pub runtime_scope: String,
    pub source_realtime: u64,
    pub realtime: u64,
    pub transport: String,
    pub message: String,
    pub message_id: String,
    pub unit_result: String,
    pub code_line: usize,
    pub code_function: String,
    pub code_file: String,
    pub user_invocation_id: String,
    pub user_unit: String,
    pub custom: HashMap<String, String>,
    pub seqnum: u64,
}

// https://wiki.archlinux.org/title/Systemd/Journal
#[derive(Debug, Serialize, PartialEq)]
pub enum Priority {
    Emergency,
    Alert,
    Critical,
    Error,
    Warning,
    Notice,
    Informational,
    Debug,
    None,
}

// https://wiki.archlinux.org/title/Systemd/Journal
#[derive(Debug, Serialize, PartialEq)]
pub enum Facility {
    Kernel,
    User,
    Mail,
    Daemon,
    Authentication,
    Syslog,
    LinePrinter,
    News,
    Uucp,
    Clock,
    AuthenticationPriv,
    Ftp,
    Ntp,
    LogAudit,
    LogAlert,
    Cron,
    Local0,
    Local1,
    Local2,
    Local3,
    Local4,
    Local5,
    Local6,
    Local7,
    None,
}
