use crate::utils::{
    nom_helper::{nom_unsigned_four_bytes, nom_unsigned_two_bytes, Endian},
    uuid::format_guid_le_bytes,
};
use nom::bytes::complete::take;
use std::mem::size_of;

#[derive(Debug)]
pub(crate) struct Fixed {
    product_version: String,
    format_version: u16,
    job_id: String,
    app_offset: u16,
    triggers_offset: u16,
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
}

#[derive(Debug, PartialEq)]
enum Priority {
    Normal,
    High,
    Idle,
    Realtime,
    Unknown,
}

/// Additional status codes at [Microsoft](https://learn.microsoft.com/en-us/windows/win32/taskschd/task-scheduler-error-and-success-constants)
#[derive(Debug, PartialEq)]
enum Status {
    Ready,
    Running,
    Disabled,
    HasNotRun,
    NoMoreRuns,
    NotScheduled,
    Terminated,
    NoValidTriggers,
    SomeTriggersFailed,
    BatchLogonProblem,
    Queued,
    Unknown,
}

#[derive(Debug, PartialEq)]
enum Flags {
    Interactive,
    DeleteWhenDone,
    Disabled,
    KillOnIdleEnd,
    StartOnlyIfIdle,
    DontStartIfOnBatteries,
    KillIfGoingOnBatteries,
    RunOnlyIfDocked,
    Hidden,
    RunIfConnectedToInternet,
    RestartOnIdleResume,
    SystemRequired,
    RunOnlyIfLoggedOn,
    ApplicationName,
}

pub(crate) fn parse_fixed(data: &[u8]) -> nom::IResult<&[u8], Fixed> {
    let (input, product_version_data) = nom_unsigned_two_bytes(data, Endian::Le)?;
    let (input, format_version) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, uuid_data) = take(size_of::<u128>())(input)?;
    let job_id = format_guid_le_bytes(uuid_data);

    let (input, app_offset) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, triggers_offset) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, error_retry_count) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, error_retry_interval) = nom_unsigned_two_bytes(input, Endian::Le)?;

    let (input, idle_deadline) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, idle_wait) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, priority_data) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let (input, max_run_time) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, exit_code) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, status_data) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, flag_data) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let (input, system_time_data) = take(size_of::<u128>())(input)?;

    let fixed = Fixed {
        product_version: product_version(&product_version_data),
        format_version,
        job_id,
        app_offset,
        triggers_offset,
        error_retry_count,
        error_retry_interval,
        idle_deadline,
        idle_wait,
        priority: priority(&priority_data),
        max_run_time,
        exit_code,
        status: status(&status_data),
        flags: flags(&flag_data),
        system_time: String::new(),
    };

    Ok((input, fixed))
}

fn product_version(version: &u16) -> String {
    match version {
        0x400 => String::from("Windows NT 4.0"),
        0x500 => String::from("Windows 2000"),
        0x501 => String::from("Windows XP"),
        0x600 => String::from("Windows Vista"),
        0x601 => String::from("Windows 7"),
        0x602 => String::from("Windows 8"),
        0x603 => String::from("Windows 8.1"),
        0xa00 => String::from("Windows 10"),
        _ => String::from("Unknown"),
    }
}

fn priority(priority: &u32) -> Priority {
    match priority {
        0x20 => Priority::Normal,
        0x40 => Priority::High,
        0x80 => Priority::Idle,
        0x100 => Priority::Realtime,
        _ => Priority::Unknown,
    }
}

fn status(status: &u32) -> Status {
    match status {
        0x41300 => Status::Ready,
        0x41301 => Status::Running,
        0x41302 => Status::Disabled,
        0x41303 => Status::HasNotRun,
        0x41304 => Status::NoMoreRuns,
        0x41305 => Status::NotScheduled,
        0x41306 => Status::Terminated,
        0x41307 => Status::NoValidTriggers,
        0x4131b => Status::SomeTriggersFailed,
        0x4311c => Status::BatchLogonProblem,
        0x43125 => Status::Queued,
        _ => Status::Unknown,
    }
}

fn flags(flags: &u32) -> Vec<Flags> {
    let interactive = 0x1;
    let delete_done = 0x2;
    let disabled = 0x4;
    let start_idle = 0x10;
    let kill_idle = 0x20;
    let dont_batteries = 0x40;
    let kill_batteries = 0x80;
    let docked = 0x100;
    let hidden = 0x200;
    let internet = 0x400;
    let idle_resume = 0x800;
    let system = 0x1000;
    let logged = 0x2000;
    let app_name = 0x01000000;

    let mut flag_vec = Vec::new();

    if (flags & interactive) == interactive {
        flag_vec.push(Flags::Interactive);
    }
    if (flags & delete_done) == delete_done {
        flag_vec.push(Flags::DeleteWhenDone);
    }
    if (flags & disabled) == disabled {
        flag_vec.push(Flags::Disabled);
    }
    if (flags & start_idle) == start_idle {
        flag_vec.push(Flags::StartOnlyIfIdle);
    }
    if (flags & kill_idle) == kill_idle {
        flag_vec.push(Flags::KillOnIdleEnd);
    }
    if (flags & dont_batteries) == dont_batteries {
        flag_vec.push(Flags::DontStartIfOnBatteries);
    }
    if (flags & kill_batteries) == kill_batteries {
        flag_vec.push(Flags::KillIfGoingOnBatteries);
    }
    if (flags & docked) == docked {
        flag_vec.push(Flags::RunOnlyIfDocked);
    }
    if (flags & hidden) == hidden {
        flag_vec.push(Flags::Hidden);
    }
    if (flags & internet) == internet {
        flag_vec.push(Flags::RunIfConnectedToInternet);
    }
    if (flags & idle_resume) == idle_resume {
        flag_vec.push(Flags::RestartOnIdleResume);
    }
    if (flags & system) == system {
        flag_vec.push(Flags::SystemRequired);
    }
    if (flags & logged) == logged {
        flag_vec.push(Flags::RunOnlyIfLoggedOn);
    }
    if (flags & app_name) == app_name {
        flag_vec.push(Flags::ApplicationName);
    }

    flag_vec
}

fn system_time(data: &[u8]) {}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::tasks::sections::fixed::{Flags, Priority, Status},
        filesystem::files::read_file,
    };

    use super::parse_fixed;
    use std::path::PathBuf;

    #[test]
    fn test_parse_fixed() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/tasks/win10/At1.job");

        let data = read_file(&test_location.display().to_string()).unwrap();
        let (_, result) = parse_fixed(&data).unwrap();

        assert_eq!(result.product_version, "Windows 10");
        assert_eq!(result.format_version, 1);
        assert_eq!(result.job_id, "01402ff8-7371-4bba-a728-a7d4f012d5c6");
        assert_eq!(result.app_offset, 70);
        assert_eq!(result.triggers_offset, 222);
        assert_eq!(result.error_retry_count, 0);
        assert_eq!(result.error_retry_interval, 0);
        assert_eq!(result.idle_deadline, 60);
        assert_eq!(result.idle_wait, 10);
        assert_eq!(result.priority, Priority::Normal);
        assert_eq!(result.max_run_time, 259200000);
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.status, Status::HasNotRun);
        assert_eq!(
            result.flags,
            vec![Flags::DeleteWhenDone, Flags::ApplicationName]
        );
        assert_eq!(result.system_time, "");
    }
}
