use super::{
    error::TaskError,
    sections::{fixed::parse_fixed, variable::parse_variable},
};
use crate::accessor::{access::Accessor, entry::handle::FileHandle};
use common::windows::TaskJob;
use tracing::error;

/// Read and parse the binary `Job` format
pub(crate) fn read_job(handle: &FileHandle) -> Result<TaskJob, TaskError> {
    let mut accessor = Accessor::with_defaults();
    let bytes = match accessor.read_file_handle(handle) {
        Ok(result) => result,
        Err(err) => {
            error!(
                "Could not read Task Job file at {}: {err:?}",
                handle.display_path()
            );
            return Err(TaskError::ReadJob);
        }
    };

    let fixed_result = parse_fixed(&bytes);
    let (var_data, fixed_value) = match fixed_result {
        Ok(result) => result,
        Err(_err) => {
            error!(
                "Could not parse Fixed section of Job file {}",
                handle.display_path()
            );
            return Err(TaskError::FixedSection);
        }
    };

    let var_result = parse_variable(var_data);
    let (_, variable_value) = match var_result {
        Ok(result) => result,
        Err(_err) => {
            error!(
                "Could not parse Variable section of Job file {}",
                handle.display_path()
            );
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
        evidence: handle.display_path(),
    };

    Ok(job)
}

#[cfg(test)]
mod tests {
    use crate::{
        accessor::entry::handle::FileHandle, artifacts::os::windows::tasks::job::read_job,
    };
    use std::path::PathBuf;

    #[test]
    fn test_read_job() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/tasks/win10/At1.job");

        let result = read_job(&FileHandle::host(test_location)).unwrap();

        assert_eq!(result.application_name, "cmd.exe");
        assert_eq!(result.comments, "Created by NetScheduleJobAdd.");
    }
}
