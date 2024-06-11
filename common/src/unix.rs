use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct CronFile {
    pub cron_data: Vec<Cron>,
    pub path: String,
    pub contents: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Cron {
    pub hour: String,
    pub min: String,
    pub day: String,
    pub month: String,
    pub weekday: String,
    pub command: String,
}

#[derive(Debug, Serialize)]
pub struct BashHistory {
    pub history: Vec<BashHistoryData>,
    pub path: String,
    pub user: String,
}

#[derive(Debug, Serialize)]
pub struct BashHistoryData {
    pub history: String,
    pub timestamp: String,
    pub line: usize,
}

#[derive(Debug, Serialize)]
pub struct PythonHistory {
    pub history: Vec<PythonHistoryData>,
    pub path: String,
    pub user: String,
}

#[derive(Debug, Serialize)]
pub struct PythonHistoryData {
    pub history: String,
    pub line: usize,
}

#[derive(Debug, Serialize)]
pub struct ZshHistory {
    pub history: Vec<ZshHistoryData>,
    pub path: String,
    pub user: String,
}

#[derive(Debug, Serialize)]
pub struct ZshHistoryData {
    pub history: String,
    pub timestamp: String,
    pub line: usize,
    pub duration: u64,
}
