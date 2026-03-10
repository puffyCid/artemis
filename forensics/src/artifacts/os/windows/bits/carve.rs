use crate::utils::nom_helper::{Endian, nom_unsigned_four_bytes, nom_unsigned_sixteen_bytes};
use common::windows::{BitsInfo, FileInfo, JobInfo, JobPriority, JobState, JobType};
use log::warn;
use nom::bytes::complete::take_until;

use super::{
    JOB_DELIMITERS,
    files::get_legacy_files,
    jobs::{get_type, job_details, parse_job},
};

pub(crate) type WinBits = (Vec<BitsInfo>, Vec<JobInfo>, Vec<FileInfo>);

/// Attempt to carve out BITS data by looking for known job and file delimiters
pub(crate) fn carve_bits<'a>(
    data: &'a [u8],
    is_legacy: bool,
    evidence: &str,
) -> nom::IResult<&'a [u8], WinBits> {
    let mut job_data = data;
    let mut bits = Vec::new();
    let mut jobs = Vec::new();
    let mut files = Vec::new();
    let carve = true;

    // Start by scanning for known job delimiters
    for job in JOB_DELIMITERS {
        while !job_data.is_empty() {
            let scan_results = scan_delimiter(job_data, &job);
            // If no hits move on to next delimiter
            let hit_data = match scan_results {
                Ok((input, _)) => input,
                Err(_err) => break,
            };

            let (input, _) = nom_unsigned_sixteen_bytes(hit_data, Endian::Le)?;
            let (_, job_type_value) = nom_unsigned_four_bytes(input, Endian::Le)?;
            let job_type = get_type(job_type_value);

            if job_type == JobType::Unknown {
                job_data = input;
                continue;
            }

            let mut job = JobInfo {
                job_id: String::new(),
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
                flags: Vec::new(),
                http_method: String::new(),
                acls: Vec::new(),
                additional_sids: Vec::new(),
                transient_error_count: 0,
                retry_delay: 0,
                timeout: 0,
                target_path: String::new(),
                file_ids: Vec::new(),
            };
            let input = match parse_job(hit_data, &mut job, carve) {
                Ok((result, _)) => result,
                Err(_err) => {
                    warn_carve_skip("job header", data, hit_data, evidence);
                    job_data = input;
                    continue;
                }
            };

            if is_legacy {
                let (remaining_input, file) = match get_legacy_files(input, is_legacy, carve) {
                    Ok(results) => results,
                    Err(_err) => {
                        warn_carve_skip("legacy file data", data, input, evidence);
                        job_data = input;
                        continue;
                    }
                };
                let (remaining_input, _) = match job_details(remaining_input, &mut job, is_legacy) {
                    Ok(results) => results,
                    Err(_err) => {
                        warn_carve_skip("legacy job details", data, remaining_input, evidence);
                        job_data = remaining_input;
                        continue;
                    }
                };

                job_data = remaining_input;
                let carved = true;
                bits.push(combine_file_and_job(&job, &file, carved, evidence));
                continue;
            }
            let remaining_input_result = job_details(input, &mut job, is_legacy);
            match remaining_input_result {
                Ok((result, _)) => job_data = result,
                Err(_err) => {
                    warn_carve_skip("job details", data, input, evidence);
                    job_data = input;
                    continue;
                }
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
                // Before we break because of parsing error, check one more time for the file delimiter
                // If we find another delimiter, keep trying to parse the data
                let (check_data, _) = nom_unsigned_sixteen_bytes(file_data, Endian::Le)?;
                let file_delimiter = [
                    228, 207, 158, 81, 70, 217, 151, 67, 183, 62, 38, 133, 19, 5, 26, 178,
                ];
                let scan_results = scan_delimiter(check_data, &file_delimiter);
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

fn carve_offset(data: &[u8], hit_data: &[u8]) -> usize {
    data.len().saturating_sub(hit_data.len())
}

fn warn_carve_skip(stage: &str, data: &[u8], hit_data: &[u8], evidence: &str) {
    warn!(
        "[bits] Best-effort carving skipped malformed {stage} at offset {} in {evidence}",
        carve_offset(data, hit_data)
    );
}

/// The legacy BITS format has both job and file info in same structure, we combine them both here into one structure
pub(crate) fn combine_file_and_job(
    job: &JobInfo,
    file: &FileInfo,
    carved: bool,
    evidence: &str,
) -> BitsInfo {
    BitsInfo {
        job_id: job.job_id.clone(),
        file_id: file.file_id.clone(),
        owner_sid: job.owner_sid.clone(),
        created: job.created.clone(),
        modified: job.modified.clone(),
        completed: job.completed.clone(),
        bytes_downloaded: file.download_bytes_size,
        bytes_transferred: file.transfer_bytes_size,
        job_name: job.job_name.clone(),
        job_description: job.job_description.clone(),
        job_command: job.job_command.clone(),
        job_arguments: job.job_arguments.clone(),
        error_count: job.error_count,
        job_type: job.job_type.clone(),
        job_state: job.job_state.clone(),
        priority: job.priority.clone(),
        flags: job.flags.clone(),
        http_method: job.http_method.clone(),
        full_path: file.full_path.clone(),
        filename: file.filename.clone(),
        target_path: job.target_path.clone(),
        volume: file.volume.clone(),
        url: file.url.clone(),
        carved,
        expiration: job.expiration.clone(),
        transient_error_count: job.transient_error_count,
        acls: job.acls.clone(),
        timeout: job.timeout,
        retry_delay: job.retry_delay,
        additional_sids: job.additional_sids.clone(),
        drive: file.drive.clone(),
        tmp_fullpath: file.tmp_fullpath.clone(),
        evidence: evidence.to_string(),
    }
}

/// Scan for a specific delimiter when carving BITS
pub(crate) fn scan_delimiter<'a>(data: &'a [u8], delimiter: &[u8]) -> nom::IResult<&'a [u8], ()> {
    let (input, _) = take_until(delimiter)(data)?;
    Ok((input, ()))
}

#[cfg(test)]
mod tests {
    use super::{JOB_DELIMITERS, carve_bits, combine_file_and_job, scan_delimiter};
    use crate::filesystem::files::read_file;
    use common::windows::{FileInfo, JobInfo, JobPriority, JobState, JobType};
    use std::path::PathBuf;

    #[test]
    fn test_carve_bits() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/bits/win81/qmgr0.dat");
        let data = read_file(test_location.to_str().unwrap()).unwrap();

        let (_, (results, _, _)) =
            carve_bits(&data, true, test_location.to_str().unwrap()).unwrap();
        assert_eq!(results.len(), 20);
        assert_eq!(results[1].job_name, "WU Client Download");
        assert_eq!(results[3].bytes_downloaded, 0);
        assert_eq!(results[18].job_id, "38efd4fb-5c6a-4c7c-b58e-4ef7fa7a349b");
        assert!(results[3].evidence.ends_with("qmgr0.dat"))
    }

    #[test]
    fn test_carve_bits_ese() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/ese/win10/qmgr.db");
        let data = read_file(test_location.to_str().unwrap()).unwrap();

        let (_, (_, jobs, files)) =
            carve_bits(&data, false, test_location.to_str().unwrap()).unwrap();
        assert_eq!(jobs.len(), 106);
        assert_eq!(files.len(), 41);

        assert_eq!(jobs[1].job_name, "PreSignInSettingsConfigJSON");
        assert_eq!(jobs[3].created, "2019-11-24T23:30:03.000Z");
        assert_eq!(jobs[18].job_id, "2d101a37-d827-41c8-828c-664a276b096d");

        assert_eq!(
            files[1].full_path,
            "C:\\Users\\bob\\AppData\\Local\\{CD2A12A4-B1E8-495D-A6C7-5C5C90E27F7A}"
        );
        assert_eq!(
            files[4].tmp_fullpath,
            "C:\\Users\\bob\\AppData\\Local\\Temp\\BITC907.tmp"
        );
        assert_eq!(
            files[8].url,
            "https://download.visualstudio.microsoft.com/download/pr/40040b24-2de2-4177-8715-900ac0996174/ab3c263d5fb2e088ddc38701c467e832bf65cca25f68958b03daad9950f8647b/Xamarin.Android.Sdk-11.4.99.70.vsix"
        );
    }

    #[test]
    fn test_carve_bits_ese_skips_malformed_prefix_jobs() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/ese/win10/qmgr.db");
        let mut data = Vec::new();
        for delimiter in JOB_DELIMITERS {
            data.extend_from_slice(&delimiter);
            data.extend_from_slice(&0u32.to_le_bytes());
        }
        data.extend_from_slice(&read_file(test_location.to_str().unwrap()).unwrap());

        let (_, (_, jobs, files)) =
            carve_bits(&data, false, test_location.to_str().unwrap()).unwrap();
        assert_eq!(jobs.len(), 106);
        assert_eq!(files.len(), 41);
    }

    #[test]
    fn test_combine_file_and_job() {
        let job = JobInfo {
            job_id: String::new(),
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
            flags: Vec::new(),
            http_method: String::new(),
            acls: Vec::new(),
            additional_sids: Vec::new(),
            transient_error_count: 0,
            retry_delay: 0,
            timeout: 0,
            target_path: String::new(),
            file_ids: Vec::new(),
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
            transfer_bytes_size: 0,
            files_transferred: 0,
        };

        let bit_info = combine_file_and_job(&job, &file, true, "test");
        assert_eq!(bit_info.carved, true);
        assert_eq!(bit_info.evidence, "test");
    }

    #[test]
    fn test_scan_delimiter() {
        let file_delimiter = [
            228, 207, 158, 81, 70, 217, 151, 67, 183, 62, 38, 133, 19, 5, 26, 178,
        ];
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/ese/win10/qmgr.db");
        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let (scan_results, _) = scan_delimiter(&data, &file_delimiter).unwrap();

        assert_eq!(scan_results.len(), 769801);
    }
}
