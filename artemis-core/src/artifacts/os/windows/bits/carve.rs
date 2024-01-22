use std::collections::HashMap;

use crate::{
    artifacts::os::windows::accounts::parser::get_users,
    utils::nom_helper::{nom_unsigned_four_bytes, nom_unsigned_sixteen_bytes, Endian},
};
use common::windows::{BitsInfo, FileInfo, JobFlags, JobInfo, JobPriority, JobState, JobType};
use nom::bytes::complete::take_until;

use super::{
    files::get_legacy_files,
    jobs::{get_type, job_details, parse_job},
};

pub(crate) type WinBits = (Vec<BitsInfo>, Vec<JobInfo>, Vec<FileInfo>);

/// Attempt to carve out BITS data by looking for known job and file delimiters
pub(crate) fn carve_bits(data: &[u8], is_legacy: bool) -> nom::IResult<&[u8], WinBits> {
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

    let mut job_data = data;
    let mut bits = Vec::new();
    let mut jobs = Vec::new();
    let mut files = Vec::new();
    let carve = true;

    // Start by scanning for known job delimiters
    for job in job_delimiters {
        while !job_data.is_empty() {
            let scan_results = scan_delimter(job_data, &job);
            // If no hits move on to next delimiter
            let hit_data = match scan_results {
                Ok((input, _)) => input,
                Err(_err) => break,
            };

            let (input, _) = nom_unsigned_sixteen_bytes(hit_data, Endian::Le)?;
            let (_, job_type_value) = nom_unsigned_four_bytes(input, Endian::Le)?;
            let job_type = get_type(&job_type_value);

            if job_type == JobType::Unknown {
                job_data = input;
                continue;
            }

            let mut job = JobInfo {
                job_id: String::new(),
                file_id: String::new(),
                owner_sid: String::new(),
                created: 0,
                modified: 0,
                expiration: 0,
                completed: 0,
                job_name: String::new(),
                job_description: String::new(),
                job_command: String::new(),
                job_arguements: String::new(),
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
            let (input, _) = parse_job(hit_data, &mut job, carve)?;

            if is_legacy {
                let (remaining_input, file) = get_legacy_files(input, is_legacy, carve)?;
                let (remaining_input, _) = job_details(remaining_input, &mut job, is_legacy)?;

                job_data = remaining_input;
                let carved = true;
                let users = get_users().unwrap_or_default();
                bits.push(combine_file_and_job(&job, &file, carved, &users));
                continue;
            }
            let remaining_input_result = job_details(input, &mut job, is_legacy);
            match remaining_input_result {
                Ok((result, _)) => job_data = result,
                Err(_) => job_data = &[],
            }
            jobs.push(job);
        }
    }

    if is_legacy {
        return Ok((data, (bits, jobs, files)));
    }

    // For ESE BITS since the file data is in a separate table it may be located at a completely different offset than the job data table
    // (In legacy BITs the file data is part of the BITS job entry)
    // So we scan for file data using a separate loop
    let mut file_data = data;
    while !file_data.is_empty() {
        let scan_results = get_legacy_files(file_data, is_legacy, carve);
        let (hit_data, file) = match scan_results {
            Ok(results) => results,
            Err(_err) => {
                // Before we break because of parsing error, check one more time for the file delimter
                // If we find another delimiter, keep trying to parse the data
                let (check_data, _) = nom_unsigned_sixteen_bytes(file_data, Endian::Le)?;
                let file_delimiter = [
                    228, 207, 158, 81, 70, 217, 151, 67, 183, 62, 38, 133, 19, 5, 26, 178,
                ];
                let scan_results = scan_delimter(check_data, &file_delimiter);
                match scan_results {
                    Ok((input, _)) => file_data = input,
                    Err(_err) => break,
                };
                continue;
            }
        };
        file_data = hit_data;
        if file.full_path.is_empty() {
            continue;
        }
        files.push(file);
    }
    Ok((data, (bits, jobs, files)))
}

/// The legacy BITS format has both job and file info in same structure, we combine them both here into one strcuture
pub(crate) fn combine_file_and_job(
    job: &JobInfo,
    file: &FileInfo,
    carved: bool,
    users: &HashMap<String, String>,
) -> BitsInfo {
    BitsInfo {
        job_id: job.job_id.clone(),
        file_id: job.file_id.clone(),
        owner_sid: job.owner_sid.clone(),
        username: users
            .get(&job.owner_sid.clone())
            .unwrap_or(&String::new())
            .to_string(),
        created: job.created,
        modified: job.modified,
        completed: job.completed,
        files_total: file.files_transferred,
        bytes_downloaded: file.download_bytes_size,
        bytes_tranferred: file.trasfer_bytes_size,
        job_name: job.job_name.clone(),
        job_description: job.job_description.clone(),
        job_command: job.job_command.clone(),
        job_arguements: job.job_arguements.clone(),
        error_count: job.error_count,
        job_type: job.job_type.clone(),
        job_state: job.job_state.clone(),
        priority: job.priority.clone(),
        flags: job.flags.clone(),
        http_method: job.http_method.clone(),
        full_path: file.full_path.clone(),
        filename: file.filename.clone(),
        target_path: job.target_path.clone(),
        tmp_file: file.tmp_fullpath.clone(),
        volume: file.volume.clone(),
        url: file.url.clone(),
        carved,
        expiration: job.expiration,
        transient_error_count: job.transient_error_count,
        acls: job.acls.clone(),
        timeout: job.timeout,
        retry_delay: job.retry_delay,
        additional_sids: job.additional_sids.clone(),
    }
}

pub(crate) fn scan_delimter<'a>(data: &'a [u8], delimter: &[u8]) -> nom::IResult<&'a [u8], ()> {
    let (input, _) = take_until(delimter)(data)?;
    Ok((input, ()))
}

#[cfg(test)]
mod tests {
    use super::{carve_bits, combine_file_and_job, scan_delimter};
    use crate::filesystem::files::read_file;
    use common::windows::{FileInfo, JobFlags, JobInfo, JobPriority, JobState, JobType};
    use std::{collections::HashMap, path::PathBuf};

    #[test]
    fn test_carve_bits() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/bits/win81/qmgr0.dat");
        let data = read_file(test_location.to_str().unwrap()).unwrap();

        let (_, (results, _, _)) = carve_bits(&data, true).unwrap();
        assert_eq!(results.len(), 20);
        assert_eq!(results[1].job_name, "WU Client Download");
        assert_eq!(results[3].bytes_downloaded, 0);
        assert_eq!(results[18].job_id, "38efd4fb-5c6a-4c7c-b58e-4ef7fa7a349b");
    }

    #[test]
    fn test_carve_bits_ese() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/ese/win10/qmgr.db");
        let data = read_file(test_location.to_str().unwrap()).unwrap();

        let (_, (_, jobs, files)) = carve_bits(&data, false).unwrap();
        assert_eq!(jobs.len(), 86);
        assert_eq!(files.len(), 41);

        assert_eq!(jobs[1].job_name, "PreSignInSettingsConfigJSON");
        assert_eq!(jobs[3].created, 1574638203);
        assert_eq!(jobs[18].job_id, "2d101a37-d827-41c8-828c-664a276b096d");

        assert_eq!(
            files[1].full_path,
            "C:\\Users\\bob\\AppData\\Local\\{CD2A12A4-B1E8-495D-A6C7-5C5C90E27F7A}"
        );
        assert_eq!(
            files[4].tmp_fullpath,
            "C:\\Users\\bob\\AppData\\Local\\Temp\\BITC907.tmp"
        );
        assert_eq!(files[8].url,"https://download.visualstudio.microsoft.com/download/pr/40040b24-2de2-4177-8715-900ac0996174/ab3c263d5fb2e088ddc38701c467e832bf65cca25f68958b03daad9950f8647b/Xamarin.Android.Sdk-11.4.99.70.vsix");
    }

    #[test]
    fn test_combine_file_and_job() {
        let job = JobInfo {
            job_id: String::new(),
            file_id: String::new(),
            owner_sid: String::new(),
            created: 0,
            modified: 0,
            expiration: 0,
            completed: 0,
            job_name: String::new(),
            job_description: String::new(),
            job_command: String::new(),
            job_arguements: String::new(),
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

        let file = FileInfo {
            file_id: String::new(),
            filename: String::new(),
            full_path: String::new(),
            tmp_fullpath: String::new(),
            drive: String::new(),
            volume: String::new(),
            url: String::new(),
            download_bytes_size: 0,
            trasfer_bytes_size: 0,
            files_transferred: 0,
        };

        let bit_info = combine_file_and_job(&job, &file, true, &HashMap::new());
        assert_eq!(bit_info.carved, true);
    }

    #[test]
    fn test_scan_delimter() {
        let file_delimiter = [
            228, 207, 158, 81, 70, 217, 151, 67, 183, 62, 38, 133, 19, 5, 26, 178,
        ];
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/ese/win10/qmgr.db");
        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let (scan_results, _) = scan_delimter(&data, &file_delimiter).unwrap();

        assert_eq!(scan_results.len(), 769801);
    }
}
