use std::collections::HashSet;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct Command {
    /**Unique list of endpoint IDs */
    pub(crate) targets: HashSet<String>,
    /**Job to send to the targets */
    pub(crate) job: JobInfo,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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
    /**When endpoint should start job */
    pub(crate) start_time: u64,
    /**How long job should run */
    pub(crate) duration: u64,
    pub(crate) action: Action,
    pub(crate) job_type: JobType,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub(crate) enum Status {
    NotStarted,
    Started,
    Finished,
    Failed,
    Cancelled,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub(crate) enum Action {
    Start,
    Stop,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub(crate) enum JobType {
    Collection,
    Processes,
    Filelist,
    Script,
}
