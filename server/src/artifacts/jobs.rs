use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct JobInfo {
    pub(crate) id: u64,
    pub(crate) name: String,
    /**When Job is created */
    pub(crate) created: u64,
    /**When endpoint executes the Job */
    pub(crate) started: u64,
    /**When endpoint finishes the Job */
    pub(crate) finished: u64,
    pub(crate) status: Status,
    /**Base64 encoded TOML collection */
    pub(crate) collection: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub(crate) enum Status {
    NotStarted,
    Started,
    Finished,
    Failed,
}
