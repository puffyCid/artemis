use super::{
    error::TaskError,
    sections::{
        fixed::{parse_fixed, Flags, Priority, Status},
        variable::{parse_variable, Triggers},
    },
};
use crate::filesystem::files::read_file;
use log::error;
use serde::Serialize;

/**
 * The old Windows Task format. Disabled on Windows 8 and higher. But can be enabled via Registry
 * Format at: [libyal](https://github.com/libyal/dtformats/blob/main/documentation/Job%20file%20format.asciidoc)
 */
#[derive(Serialize)]
pub(crate) struct TaskJob {
    job_id: String,
    error_retry_count: u16,
    error_retry_interval: u16,
    idle_deadline: u16,
    idle_wait: u16,
    priority: Priority,
    max_run_time: u32,
    exit_code: u32,
    status: Status,
    flags: Vec<Flags>,
    system_time: String,
    running_instance_count: u16,
    application_name: String,
    parameters: String,
    working_directory: String,
    author: String,
    comments: String,
    user_data: String,
    start_error: u32,
    triggers: Vec<Triggers>,
}

impl TaskJob {
    /// Parse the older Task format
    pub(crate) fn parse_job(path: &str) -> Result<TaskJob, TaskError> {
        TaskJob::read_job(path)
    }

    /// Read and parse the binary `Job` format
    fn read_job(path: &str) -> Result<TaskJob, TaskError> {
        let bytes_result = read_file(path);
        let bytes = match bytes_result {
            Ok(result) => result,
            Err(err) => {
                error!("[tasks] Could not read Task Job file at {path}: {err:?}");
                return Err(TaskError::ReadJob);
            }
        };

        let fixed_result = parse_fixed(&bytes);
        let (var_data, fixed_value) = match fixed_result {
            Ok(result) => result,
            Err(_err) => {
                error!("[tasks] Could not parse Fixed section of Job file {path}");
                return Err(TaskError::FixedSection);
            }
        };

        let var_result = parse_variable(var_data);
        let (_, variable_value) = match var_result {
            Ok(result) => result,
            Err(_err) => {
                error!("[tasks] Could not parse Variable section of Job file {path}");
                return Err(TaskError::VariableSection);
            }
        };

        let job = TaskJob {
            job_id: fixed_value.job_id,
            error_retry_count: fixed_value.error_retry_count,
            error_retry_interval: fixed_value.error_retry_interval,
            idle_deadline: fixed_value.idle_deadline,
            idle_wait: fixed_value.idle_wait,
            priority: fixed_value.priority,
            max_run_time: fixed_value.max_run_time,
            exit_code: fixed_value.exit_code,
            status: fixed_value.status,
            flags: fixed_value.flags,
            system_time: fixed_value.system_time,
            running_instance_count: variable_value.running_instance_count,
            application_name: variable_value.app_name,
            parameters: variable_value.parameters,
            working_directory: variable_value.working_directory,
            author: variable_value.author,
            comments: variable_value.comment,
            user_data: variable_value.user_data,
            start_error: variable_value.start_error,
            triggers: variable_value.triggers,
        };

        Ok(job)
    }
}

#[cfg(test)]
mod tests {
    use super::TaskJob;
    use std::path::PathBuf;

    #[test]
    fn test_parse_job() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/tasks/win10/At1.job");

        let result = TaskJob::parse_job(&test_location.display().to_string()).unwrap();

        assert_eq!(result.job_id, "01402ff8-7371-4bba-a728-a7d4f012d5c6");
        assert_eq!(result.author, "WORKGROUP\\DESKTOP-EIS938N$");
    }

    #[test]
    fn test_read_job() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/tasks/win10/At1.job");

        let result = TaskJob::read_job(&test_location.display().to_string()).unwrap();

        assert_eq!(result.application_name, "cmd.exe");
        assert_eq!(result.comments, "Created by NetScheduleJobAdd.");
    }
}
