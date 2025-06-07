use crate::utils::{
    encoding::base64_encode_standard,
    nom_helper::{Endian, nom_unsigned_four_bytes, nom_unsigned_two_bytes},
    strings::extract_utf16_string,
};
use common::windows::{TriggerFlags, TriggerTypes, VarTriggers};
use nom::bytes::complete::take;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct Variable {
    pub(crate) running_instance_count: u16,
    pub(crate) app_name: String,
    pub(crate) parameters: String,
    pub(crate) working_directory: String,
    pub(crate) author: String,
    pub(crate) comment: String,
    pub(crate) user_data: String,
    pub(crate) start_error: u32,
    /**Unused */
    task_flags: u32,
    pub(crate) triggers: Vec<VarTriggers>,
}

/// Parse the Variable section of the `Job` file
pub(crate) fn parse_variable(data: &[u8]) -> nom::IResult<&[u8], Variable> {
    let (input, running_instance_count) = nom_unsigned_two_bytes(data, Endian::Le)?;
    let (input, app_name) = get_string(input)?;
    let (input, parameters) = get_string(input)?;
    let (input, working_directory) = get_string(input)?;
    let (input, author) = get_string(input)?;
    let (input, comment) = get_string(input)?;

    let (input, user_data) = user_data(input)?;
    let (input, (start_error, task_flags)) = reserved_data(input)?;

    let (_, triggers) = triggers(input)?;

    let variable = Variable {
        running_instance_count,
        app_name,
        parameters,
        working_directory,
        author,
        comment,
        user_data,
        start_error,
        task_flags,
        triggers,
    };

    Ok((input, variable))
}

/// Extract strings in the Variable section
fn get_string(data: &[u8]) -> nom::IResult<&[u8], String> {
    let (input, size) = nom_unsigned_two_bytes(data, Endian::Le)?;

    // Size is in UTF16 characters
    let adjust_size = 2;
    let (input, string_data) = take(size * adjust_size)(input)?;
    let value = extract_utf16_string(string_data);

    Ok((input, value))
}

/// Get User Data in the Variable section
fn user_data(data: &[u8]) -> nom::IResult<&[u8], String> {
    let (input, size) = nom_unsigned_two_bytes(data, Endian::Le)?;

    let (input, user_data) = take(size)(input)?;
    let value = base64_encode_standard(user_data);

    Ok((input, value))
}

/// Get Reserved Data in the Variable section
fn reserved_data(data: &[u8]) -> nom::IResult<&[u8], (u32, u32)> {
    let (input, size) = nom_unsigned_two_bytes(data, Endian::Le)?;

    let none = 0;
    if size == none {
        return Ok((input, (0, 0)));
    }

    let (input, start_error) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, task_flags) = nom_unsigned_four_bytes(input, Endian::Le)?;

    Ok((input, (start_error, task_flags)))
}

/// Get `Job` triggers
fn triggers(data: &[u8]) -> nom::IResult<&[u8], Vec<VarTriggers>> {
    let (mut trigger_data, trigger_count) = nom_unsigned_two_bytes(data, Endian::Le)?;
    let mut count = 0;

    let mut trigger_vec = Vec::new();
    while count < trigger_count {
        let (input, _size) = nom_unsigned_two_bytes(trigger_data, Endian::Le)?;
        let (input, _unknown) = nom_unsigned_two_bytes(input, Endian::Le)?;

        let (input, start_year) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, start_month) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, start_day) = nom_unsigned_two_bytes(input, Endian::Le)?;

        let (input, end_year) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, end_month) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, end_day) = nom_unsigned_two_bytes(input, Endian::Le)?;

        let (input, start_hours) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, start_mins) = nom_unsigned_two_bytes(input, Endian::Le)?;

        let (input, duration) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, interval) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let (input, flag_data) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, type_data) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let start_date = format!("{start_year}-{start_month}-{start_day}");
        let end_date = format!("{end_year}-{end_month}-{end_day}");
        let start_time = format!("{start_hours}:{start_mins}");

        let trigger = VarTriggers {
            start_date,
            end_date,
            start_time,
            duration,
            interval_mins: interval,
            flags: trigger_flags(&flag_data),
            types: trigger_types(&type_data),
        };
        trigger_vec.push(trigger);
        trigger_data = input;

        count += 1;
    }

    Ok((trigger_data, trigger_vec))
}

/// Get Trigger Flags
fn trigger_flags(data: &u32) -> Vec<TriggerFlags> {
    let end_data = 0x1;
    let duration_end = 0x2;
    let disabled = 0x4;

    let mut flag_vec = Vec::new();

    if (data & end_data) == end_data {
        flag_vec.push(TriggerFlags::HasEndDate);
    }
    if (data & duration_end) == duration_end {
        flag_vec.push(TriggerFlags::KillAtDurationEnd);
    }
    if (data & disabled) == disabled {
        flag_vec.push(TriggerFlags::Disabled);
    }

    flag_vec
}

/// Get Trigger Types
fn trigger_types(data: &u32) -> Vec<TriggerTypes> {
    let once = 0x0;
    let daily = 0x1;
    let weekly = 0x2;
    let monthly_date = 0x3;
    let monthly_dow = 0x4;
    let idle = 0x5;
    let start = 0x6;
    let logon = 0x7;

    let mut types_vec = Vec::new();

    if (data & once) == once {
        types_vec.push(TriggerTypes::Once);
    }
    if (data & daily) == daily {
        types_vec.push(TriggerTypes::Daily);
    }
    if (data & weekly) == weekly {
        types_vec.push(TriggerTypes::Weekly);
    }
    if (data & monthly_date) == monthly_date {
        types_vec.push(TriggerTypes::MonthlyDate);
    }
    if (data & monthly_dow) == monthly_dow {
        types_vec.push(TriggerTypes::MonthlyDow);
    }
    if (data & idle) == idle {
        types_vec.push(TriggerTypes::EventOnIdle);
    }
    if (data & start) == start {
        types_vec.push(TriggerTypes::EventAtSystemstart);
    }
    if (data & logon) == logon {
        types_vec.push(TriggerTypes::EventAtLogon);
    }

    types_vec
}

#[cfg(test)]
mod tests {
    use super::{get_string, parse_variable};
    use crate::{
        artifacts::os::windows::tasks::sections::{
            fixed::parse_fixed,
            variable::{
                TriggerFlags, TriggerTypes, reserved_data, trigger_flags, trigger_types, triggers,
                user_data,
            },
        },
        filesystem::files::read_file,
    };
    use std::path::PathBuf;

    #[test]
    fn test_parse_variable() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/tasks/win10/At1.job");

        let data = read_file(&test_location.display().to_string()).unwrap();
        let (input, _) = parse_fixed(&data).unwrap();

        let (_, result) = parse_variable(input).unwrap();

        assert_eq!(result.app_name, "cmd.exe");
        assert_eq!(result.running_instance_count, 0);
        assert_eq!(result.parameters, "");
        assert_eq!(result.author, "WORKGROUP\\DESKTOP-EIS938N$");
        assert_eq!(result.comment, "Created by NetScheduleJobAdd.");
        assert_eq!(result.start_error, 267011);
        assert_eq!(result.triggers[0].types, vec![TriggerTypes::Once]);
    }

    #[test]
    fn test_get_string() {
        let test = [
            8, 0, 99, 0, 109, 0, 100, 0, 46, 0, 101, 0, 120, 0, 101, 0, 0, 0, 0,
        ];
        let (_, result) = get_string(&test).unwrap();
        assert_eq!(result, "cmd.exe");
    }

    #[test]
    fn test_user_data() {
        let test = [
            8, 0, 99, 0, 109, 0, 100, 0, 46, 0, 101, 0, 120, 0, 101, 0, 0, 0, 0,
        ];
        let (_, result) = user_data(&test).unwrap();
        assert_eq!(result, "YwBtAGQALgA=");
    }

    #[test]
    fn test_reserved_data() {
        let test = [8, 0, 3, 19, 4, 0, 0, 0, 0, 0];
        let (_, (error, trig_type)) = reserved_data(&test).unwrap();
        assert_eq!(error, 267011);
        assert_eq!(trig_type, 0);
    }

    #[test]
    fn test_triggers() {
        let test = [
            1, 0, 48, 0, 0, 0, 231, 7, 8, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 216, 0, 0, 0, 216, 32, 162, 183, 0, 0, 0, 0,
        ];

        let (_, results) = triggers(&test).unwrap();
        assert_eq!(results[0].start_date, "2023-8-1")
    }

    #[test]
    fn test_trigger_flags() {
        let test = 0x1;
        let result = trigger_flags(&test);
        assert_eq!(result, vec![TriggerFlags::HasEndDate]);
    }

    #[test]
    fn test_trigger_types() {
        let test = 0x1;
        let result = trigger_types(&test);
        assert_eq!(result, vec![TriggerTypes::Once, TriggerTypes::Daily]);
    }
}
