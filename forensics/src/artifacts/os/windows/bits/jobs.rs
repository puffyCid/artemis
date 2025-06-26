use super::{carve::combine_file_and_job, error::BitsError, files::get_legacy_files};
use crate::{
    artifacts::os::windows::{
        bits::carve::scan_delimiter,
        securitydescriptor::{acl::parse_acl, sid::grab_sid},
    },
    utils::{
        encoding::base64_decode_standard,
        nom_helper::{
            Endian, nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_one_byte,
            nom_unsigned_sixteen_bytes,
        },
        strings::extract_utf16_string,
        time::{filetime_to_unixepoch, unixepoch_to_iso},
        uuid::format_guid_le_bytes,
    },
};
use common::windows::{
    AccessItem, BitsInfo, JobFlags, JobInfo, JobPriority, JobState, JobType, TableDump,
};
use log::error;
use nom::bytes::complete::{take, take_until};
use std::mem::size_of;

/// Loop through table rows and parse out all of the active BITS jobs
pub(crate) fn get_jobs(column_rows: &[Vec<TableDump>]) -> Result<Vec<JobInfo>, BitsError> {
    let mut jobs: Vec<JobInfo> = Vec::new();
    for rows in column_rows {
        let mut job = JobInfo {
            job_id: String::new(),
            file_id: String::new(),
            owner_sid: String::new(),
            created: String::new(),
            modified: String::new(),
            expiration: String::new(),
            completed: String::new(),
            job_name: String::new(),
            job_description: String::new(),
            job_command: String::new(),
            job_arguments: String::new(),
            error_count: 0,
            job_type: JobType::Unknown,
            job_state: JobState::Unknown,
            priority: JobPriority::Unknown,
            flags: JobFlags::Unknown,
            http_method: String::new(),
            acls: Vec::new(),
            additional_sids: Vec::new(),
            transient_error_count: 0,
            retry_delay: 0,
            timeout: 0,
            target_path: String::new(),
        };
        // Only two (2) columns in BITS table (as of Win11)
        for column in rows {
            if column.column_name == "Id" {
                job.job_id.clone_from(&column.column_data);
            }

            if column.column_name == "Blob" {
                let decode_results = base64_decode_standard(&column.column_data);
                if let Ok(results) = decode_results {
                    let is_legacy = false;
                    let carve = false;
                    let job_results = parse_job(&results, &mut job, carve);
                    let job_data = match job_results {
                        Ok((data_results, _)) => data_results,
                        Err(_err) => {
                            error!("[bits] Could not parse BITS job details");
                            continue;
                        }
                    };
                    if job_details(job_data, &mut job, is_legacy).is_ok() {
                        continue;
                    }

                    error!("[bits] Could not parse BITS job file info");
                }
            }
        }
        jobs.push(job);
    }

    Ok(jobs)
}

/// Get BITS jobs in older format
pub(crate) fn get_legacy_jobs(data: &[u8]) -> Result<Vec<BitsInfo>, BitsError> {
    let job_results = parse_legacy_job(data);
    let jobs = if let Ok((_, results)) = job_results {
        results
    } else {
        error!("[bits] Could not parse legacy BITS format");
        return Err(BitsError::ParseLegacyBits);
    };
    Ok(jobs)
}

/// Parse older BITS format
fn parse_legacy_job(data: &[u8]) -> nom::IResult<&[u8], Vec<BitsInfo>> {
    let (_, sig) = nom_unsigned_one_byte(data, Endian::Le)?;
    let win10 = 40;
    let win10_size = 24;
    let win7_size = 16;

    let change_size: u8 = if sig == win10 { win10_size } else { win7_size };

    let (input, _change_data) = take(change_size)(data)?;

    let (input, _header_data) = take(size_of::<u128>())(input)?;
    let (input, _header_data) = take(size_of::<u128>())(input)?;

    let (mut input, number_jobs) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let mut job_count = 0;

    let mut jobs: Vec<BitsInfo> = Vec::new();
    while job_count < number_jobs {
        let mut job = JobInfo {
            job_id: String::new(),
            file_id: String::new(),
            owner_sid: String::new(),
            created: String::new(),
            modified: String::new(),
            expiration: String::new(),
            completed: String::new(),
            job_name: String::new(),
            job_description: String::new(),
            job_command: String::new(),
            job_arguments: String::new(),
            error_count: 0,
            job_type: JobType::Unknown,
            job_state: JobState::Unknown,
            priority: JobPriority::Unknown,
            flags: JobFlags::Unknown,
            http_method: String::new(),
            acls: Vec::new(),
            additional_sids: Vec::new(),
            transient_error_count: 0,
            retry_delay: 0,
            timeout: 0,
            target_path: String::new(),
        };
        let is_legacy = true;
        let carve = false;
        let (remaining_input, _) = parse_job(input, &mut job, carve)?;
        // Now comes file info
        let (remaining_input, file) = get_legacy_files(remaining_input, is_legacy, carve)?;
        let (remaining_input, _) = job_details(remaining_input, &mut job, is_legacy)?;
        let carved = false;

        jobs.push(combine_file_and_job(&job, &file, carved));
        job_count += 1;
        if job_count == number_jobs {
            break;
        }

        let job_delimiters = vec![
            [
                147, 54, 32, 53, 160, 12, 16, 74, 132, 243, 177, 126, 123, 73, 156, 215,
            ],
            [
                16, 19, 112, 200, 54, 83, 179, 65, 131, 229, 129, 85, 127, 54, 27, 135,
            ],
            [
                140, 147, 234, 100, 3, 15, 104, 64, 180, 111, 249, 127, 229, 29, 77, 205,
            ],
            [
                179, 70, 237, 61, 59, 16, 249, 68, 188, 47, 232, 55, 139, 211, 25, 134,
            ],
            [
                161, 86, 9, 225, 67, 175, 201, 66, 146, 230, 111, 152, 86, 235, 167, 246,
            ],
            [
                159, 149, 212, 76, 100, 112, 242, 75, 132, 215, 71, 106, 126, 98, 105, 159,
            ],
            [
                241, 25, 38, 169, 50, 3, 191, 76, 148, 39, 137, 136, 24, 149, 136, 49,
            ],
            [
                193, 51, 188, 221, 251, 90, 175, 77, 184, 161, 34, 104, 179, 157, 1, 173,
            ],
            [
                208, 87, 86, 143, 44, 1, 62, 78, 173, 44, 244, 165, 215, 101, 111, 175,
            ],
            [
                80, 103, 65, 148, 87, 3, 29, 70, 164, 204, 93, 217, 153, 7, 6, 228,
            ],
        ];
        let remaining_bits_size = input.len();
        // For legacy BITS scan data for known job delimiters, footer signifies the end of the job
        for job in job_delimiters {
            if !remaining_input.is_empty() {
                let scan_results = scan_delimiter(remaining_input, &job);
                // If no hits move on to next delimiter
                let hit_data = match scan_results {
                    Ok((input, _)) => input,
                    Err(_err) => break,
                };
                // Reached job footer
                let (remaining_input, _) = nom_unsigned_sixteen_bytes(hit_data, Endian::Le)?;
                input = remaining_input;
            }
            if input.len() != remaining_bits_size {
                break;
            }
        }
        // If remaining_bits_size is same as input size we did not find any job delimiters
        if input.len() == remaining_bits_size {
            break;
        }
    }

    Ok((input, jobs))
}

/// Parse the BITS job
pub(crate) fn parse_job<'a>(
    data: &'a [u8],
    job_info: &mut JobInfo,
    carve: bool,
) -> nom::IResult<&'a [u8], ()> {
    let (input, _header_data) = take(size_of::<u128>())(data)?;
    let (input, job_type) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, job_priority) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, job_state) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let (input, _unknown) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let (input, job_id_data) = take(size_of::<u128>())(input)?;
    if job_info.job_id.is_empty() {
        job_info.job_id = format_guid_le_bytes(job_id_data);
    }
    let (input, name_size) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let wide_char_adjust = 2;
    let (input, job_name_data) = take(name_size * wide_char_adjust)(input)?;
    let job_name = extract_utf16_string(job_name_data);

    let (input, desc_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, desc_data) = take(desc_size * wide_char_adjust)(input)?;
    let description = extract_utf16_string(desc_data);

    let (input, cmd_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, cmd_data) = take(cmd_size * wide_char_adjust)(input)?;
    let cmd = extract_utf16_string(cmd_data);

    let (input, args_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, args_data) = take(args_size * wide_char_adjust)(input)?;
    let args = extract_utf16_string(args_data);

    let (input, sid_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, sid_data) = take(sid_size * wide_char_adjust)(input)?;
    let sid = extract_utf16_string(sid_data);

    let (input, job_flag) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let token_size: u8 = 70;
    // Not sure what this is used for, is it unique per Job?
    let (input, _access_token) = take(token_size)(input)?;

    job_info.job_type = get_type(job_type);
    job_info.priority = get_priority(job_priority);
    job_info.job_state = get_state(job_state);
    job_info.job_name = job_name;
    job_info.job_description = description;
    job_info.job_command = cmd;
    job_info.job_arguments = args;
    job_info.owner_sid = sid;
    job_info.flags = get_flag(job_flag);

    let padding_size: u16 = 982;
    let (input, _) = take(padding_size)(input)?;
    // Sometimes there are no ACLs. Just padding. Seen when carving
    if input.starts_with(&[0, 0, 0, 0]) {
        return Ok((input, ()));
    }
    let (input, acls) = parse_acl(input, &AccessItem::NonFolder)?;

    job_info.acls = acls;

    // Only grab additional SIDs if we are not carving
    if !carve {
        let sid_len: u8 = 12;
        let delimiter_len: u8 = 16;
        // Grab the last two (2) SIDs (these are not part of the DACL)
        let (input, sid_data) = take(sid_len)(input)?;
        let (_, sid) = grab_sid(sid_data)?;
        job_info.additional_sids.push(sid);

        let (input, _delim_data) = take(delimiter_len)(input)?;

        let (input, sid_data) = take(sid_len)(input)?;
        let (_, sid) = grab_sid(sid_data)?;
        job_info.additional_sids.push(sid);

        let (_, _delim_data) = take(delimiter_len)(input)?;
    }

    Ok((input, ()))
}

/// Parse additional job details
pub(crate) fn job_details<'a>(
    data: &'a [u8],
    job_info: &mut JobInfo,
    is_legacy: bool,
) -> nom::IResult<&'a [u8], ()> {
    // This key means we have reached some of the job details
    let delimiter = [
        54, 218, 86, 119, 111, 81, 90, 67, 172, 172, 68, 162, 72, 255, 243, 77,
    ];
    let (input, _) = take_until(delimiter.as_slice())(data)?;
    let (mut input, _delimilter_data) = nom_unsigned_sixteen_bytes(input, Endian::Le)?;

    if !is_legacy {
        let (remaining_input, _count) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (remaining_input, file_id_data) = take(size_of::<u128>())(remaining_input)?;
        // Delimiter repeats again
        let (remaining_input, _delimiter_data) =
            nom_unsigned_sixteen_bytes(remaining_input, Endian::Le)?;
        input = remaining_input;
        job_info.file_id = format_guid_le_bytes(file_id_data);
    }

    let (input, error_count) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, transient_error_count) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, retry_delay) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, timeout) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let (input, created) = nom_unsigned_eight_bytes(input, Endian::Le)?;
    let (input, modified) = nom_unsigned_eight_bytes(input, Endian::Le)?;
    let (mut input, complete_time) = nom_unsigned_eight_bytes(input, Endian::Le)?;

    // The rest of the unknown timestamps could they be related job modifications? They are all the same as modified and complete timestamps
    // Ex: Suspend time, resume time, cancel time? Changes to the job state update the modified time
    let unknown_size: u8 = if is_legacy {
        14
    } else {
        let (remaining_input, _unknown_time2) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        input = remaining_input;
        6
    };
    let (input, _unknown) = take(unknown_size)(input)?;
    let (input, _unknown_time3) = nom_unsigned_eight_bytes(input, Endian::Le)?;
    let (input, expiration_time) = nom_unsigned_eight_bytes(input, Endian::Le)?;

    job_info.error_count = error_count;
    job_info.created = unixepoch_to_iso(filetime_to_unixepoch(created));
    job_info.modified = unixepoch_to_iso(filetime_to_unixepoch(modified));
    job_info.completed = unixepoch_to_iso(filetime_to_unixepoch(complete_time));
    job_info.expiration = unixepoch_to_iso(filetime_to_unixepoch(expiration_time));
    job_info.timeout = timeout;
    job_info.retry_delay = retry_delay;
    job_info.transient_error_count = transient_error_count;

    if is_legacy {
        return Ok((input, ()));
    }

    let proxy_data_size: u8 = 108;
    // Currently skipping proxy settings
    let (input, _proxy_data) = take(proxy_data_size)(input)?;
    let unknown_data_size: u8 = 55;
    let (input, _unknown) = take(unknown_data_size)(input)?;

    // Remaining data seems to only exist on newer versions of BITS (Win10+)
    let (input, target_path_size) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let wide_char_adjust = 2;
    // When carving return early if size is larger than remaining input
    if target_path_size as usize > input.len()
        || (target_path_size * wide_char_adjust) as usize > input.len()
    {
        return Ok((input, ()));
    }

    let (input, target_path_data) = take(target_path_size * wide_char_adjust)(input)?;
    job_info.target_path = extract_utf16_string(target_path_data);

    let unknown_size: u8 = 16;
    let (input, _unknown) = take(unknown_size)(input)?;

    let (input, method_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    // When carving return early if size is larger than remaining input
    if method_size as usize > input.len() || (method_size * wide_char_adjust) as usize > input.len()
    {
        return Ok((input, ()));
    }
    let (input, method_data) = take(method_size * wide_char_adjust)(input)?;
    job_info.http_method = extract_utf16_string(method_data);

    // Rest of data is unknown maybe custom http headers?
    // Last 16 bytes is the footer (same value as header)

    Ok((input, ()))
}

/// Determine the job type
pub(crate) fn get_type(job_type: u32) -> JobType {
    match job_type {
        0 => JobType::Download,
        1 => JobType::Upload,
        2 => JobType::UploadReply,
        _ => JobType::Unknown,
    }
}

/// Determien the job priority
pub(crate) fn get_priority(job_priority: u32) -> JobPriority {
    match job_priority {
        0 => JobPriority::Foreground,
        1 => JobPriority::High,
        2 => JobPriority::Normal,
        3 => JobPriority::Low,
        _ => JobPriority::Unknown,
    }
}

/// Determine the job state
pub(crate) fn get_state(job_state: u32) -> JobState {
    match job_state {
        0 => JobState::Queued,
        1 => JobState::Connecting,
        2 => JobState::Transferring,
        3 => JobState::Suspended,
        4 => JobState::Error,
        5 => JobState::TransientError,
        6 => JobState::Transferred,
        7 => JobState::Acknowledged,
        8 => JobState::Cancelled,
        _ => JobState::Unknown,
    }
}

/// Determine flag associated with job
pub(crate) fn get_flag(job_flag: u32) -> JobFlags {
    match job_flag {
        1 => JobFlags::Transferred,
        2 => JobFlags::Error,
        3 => JobFlags::TransferredBackgroundError,
        4 => JobFlags::Disable,
        5 => JobFlags::TransferredBackgroundDisable,
        6 => JobFlags::ErrorBackgroundDisable,
        7 => JobFlags::TransferredBackgroundErrorDisable,
        8 => JobFlags::Modification,
        16 => JobFlags::FileTransferred,
        _ => JobFlags::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::bits::jobs::{
            get_flag, get_legacy_jobs, get_priority, get_state, get_type, job_details, parse_job,
            parse_legacy_job,
        },
        filesystem::files::read_file,
    };
    use common::windows::{JobFlags, JobInfo, JobPriority, JobState, JobType};
    use std::path::PathBuf;

    #[test]
    fn test_get_flag() {
        let test = 1;
        let results = get_flag(test);
        assert_eq!(results, JobFlags::Transferred);
    }

    #[test]
    fn test_get_state() {
        let test = 0;
        let results = get_state(test);
        assert_eq!(results, JobState::Queued);
    }

    #[test]
    fn test_get_priority() {
        let test = 0;
        let results = get_priority(test);
        assert_eq!(results, JobPriority::Foreground);
    }

    #[test]
    fn test_get_type() {
        let test = 0;
        let results = get_type(test);
        assert_eq!(results, JobType::Download);
    }

    #[test]
    fn test_parse_job() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/bits/win10/job.raw");
        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let mut job = JobInfo {
            job_id: String::new(),
            file_id: String::new(),
            owner_sid: String::new(),
            created: String::new(),
            modified: String::new(),
            expiration: String::new(),
            completed: String::new(),
            job_name: String::new(),
            job_description: String::new(),
            job_command: String::new(),
            job_arguments: String::new(),
            error_count: 0,
            job_type: JobType::Unknown,
            job_state: JobState::Unknown,
            priority: JobPriority::Unknown,
            flags: JobFlags::Unknown,
            http_method: String::new(),
            acls: Vec::new(),
            additional_sids: Vec::new(),
            transient_error_count: 0,
            retry_delay: 0,
            timeout: 0,
            target_path: String::new(),
        };

        let _ = parse_job(&data, &mut job, false).unwrap();
        assert_eq!(
            job.owner_sid,
            "S-1-5-21-1079689790-2336414676-942872339-1001"
        );
        assert_eq!(job.job_name, "Chrome Component Updater");

        assert_eq!(
            job.job_description,
            "lmelglejhemejginpboagddgdfbepgmp_372_all_ZZ_djv5ss66g7sivnpz6ljtwr2zji.crx3"
        );
    }

    #[test]
    fn test_parse_job_details() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/bits/win10/job.raw");
        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let mut job = JobInfo {
            job_id: String::new(),
            file_id: String::new(),
            owner_sid: String::new(),
            created: String::new(),
            modified: String::new(),
            expiration: String::new(),
            completed: String::new(),
            job_name: String::new(),
            job_description: String::new(),
            job_command: String::new(),
            job_arguments: String::new(),
            error_count: 0,
            job_type: JobType::Unknown,
            job_state: JobState::Unknown,
            priority: JobPriority::Unknown,
            flags: JobFlags::Unknown,
            http_method: String::new(),
            acls: Vec::new(),
            additional_sids: Vec::new(),
            transient_error_count: 0,
            retry_delay: 0,
            timeout: 0,
            target_path: String::new(),
        };

        let (input, _) = parse_job(&data, &mut job, false).unwrap();
        assert_eq!(
            job.owner_sid,
            "S-1-5-21-1079689790-2336414676-942872339-1001"
        );
        assert_eq!(job.job_name, "Chrome Component Updater");

        assert_eq!(
            job.job_description,
            "lmelglejhemejginpboagddgdfbepgmp_372_all_ZZ_djv5ss66g7sivnpz6ljtwr2zji.crx3"
        );

        let _ = job_details(&input, &mut job, false).unwrap();
        assert_eq!(job.file_id, "95d6889c-b2d3-4748-8eb1-9da0650cb892");

        assert_eq!(job.http_method, "GET");
        assert_eq!(job.timeout, 86400);
        assert_eq!(job.created, "2022-12-21T02:18:03.000Z");
        assert_eq!(job.retry_delay, 60);
        assert_eq!(
            job.target_path,
            "C:\\Program Files\\Chromium\\Application\\chrome.exe"
        );
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_get_jobs() {
        use crate::artifacts::os::windows::bits::{background::get_bits_ese, jobs::get_jobs};

        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\ese\\win10\\qmgr.db");

        let jobs = get_bits_ese(test_location.to_str().unwrap(), "Jobs").unwrap();

        let jobs_info = get_jobs(&jobs).unwrap();
        assert_eq!(jobs_info.len(), 1);
    }

    #[test]
    fn test_get_legacy_jobs() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/bits/win81/qmgr0.dat");
        let data = read_file(test_location.to_str().unwrap()).unwrap();

        let results = get_legacy_jobs(&data).unwrap();
        assert_eq!(results[0].job_id, "5422299c-cd21-4c51-bad5-9da178edc742");
        assert_eq!(results[0].created, "2023-03-14T06:39:48.000Z");
        assert_eq!(results[0].job_type, JobType::Download);
        assert_eq!(results[0].job_state, JobState::Queued);
    }

    #[test]
    fn test_parse_legacy_job() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/bits/win81/qmgr0.dat");
        let data = read_file(test_location.to_str().unwrap()).unwrap();

        let (_, results) = parse_legacy_job(&data).unwrap();
        assert_eq!(results[0].job_id, "5422299c-cd21-4c51-bad5-9da178edc742");
        assert_eq!(results[0].created, "2023-03-14T06:39:48.000Z");
        assert_eq!(results[0].job_type, JobType::Download);
        assert_eq!(results[0].job_state, JobState::Queued);
    }
}
