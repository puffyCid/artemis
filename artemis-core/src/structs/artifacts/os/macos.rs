use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct UnifiedLogsOptions {
    pub sources: Vec<String>,
    pub logarchive_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SudoOptions {
    pub logarchive_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UsersOptions {
    pub alt_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GroupsOptions {
    pub alt_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct EmondOptions {
    pub alt_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ExecPolicyOptions {
    pub alt_file: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LaunchdOptions {
    pub alt_file: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FseventsOptions {
    pub alt_file: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginitemsOptions {
    pub alt_file: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SpotlightOptions {
    pub alt_path: Option<String>,
    pub include_additional: Option<bool>,
}
