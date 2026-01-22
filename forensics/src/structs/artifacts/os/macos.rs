use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct UnifiedLogsOptions {
    pub sources: Vec<String>,
    pub logarchive_path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MacosSudoOptions {
    pub logarchive_path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MacosUsersOptions {
    pub alt_path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MacosGroupsOptions {
    pub alt_path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EmondOptions {
    pub alt_path: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ExecPolicyOptions {
    pub alt_file: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LaunchdOptions {
    pub alt_file: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FseventsOptions {
    pub alt_file: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LoginitemsOptions {
    pub alt_file: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SpotlightOptions {
    pub alt_path: Option<String>,
    pub include_additional: Option<bool>,
}
