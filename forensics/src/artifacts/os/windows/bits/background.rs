use super::{
    carve::{WinBits, carve_bits},
    error::BitsError,
    files::get_files,
    jobs::{get_jobs, get_legacy_jobs},
};
use crate::{
    artifacts::os::windows::ese::{
        helper::{get_all_pages, get_catalog_info, get_page_data},
        tables::table_info,
    },
    filesystem::{files::is_file, ntfs::raw_files::raw_read_file},
};
use common::windows::{BitsInfo, FileInfo, JobInfo, JobPriority, JobState, JobType, TableDump};
use log::error;

/**
 * Parse modern version (Win10+) of BITS which is an ESE database by dumping the `Jobs` and `Files` tables and parsing their contents  
 */
pub(crate) fn parse_ese_bits(bits_path: &str, carve: bool) -> Result<Vec<BitsInfo>, BitsError> {
    // Dump the Jobs and Files tables from the BITS database
    let files = get_bits_ese(bits_path, "Files")?;
    let jobs_info = get_bits_ese(bits_path, "Jobs")?;

    let jobs = get_jobs(&jobs_info)?;

    let files_info = get_files(&files)?;
    let mut bits_info: Vec<BitsInfo> = Vec::new();

    for job in &jobs {
        for file in &files_info {
            if job.file_ids.contains(&file.file_id) {
                let bit_info = BitsInfo {
                    job_id: job.job_id.clone(),
                    file_id: file.file_id.clone(),
                    owner_sid: job.owner_sid.clone(),
                    created: job.created.clone(),
                    modified: job.modified.clone(),
                    completed: job.completed.clone(),
                    expiration: job.expiration.clone(),
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
                    tmp_file: file.tmp_fullpath.clone(),
                    volume: file.volume.clone(),
                    url: file.url.clone(),
                    timeout: job.timeout,
                    retry_delay: job.retry_delay,
                    transient_error_count: job.transient_error_count,
                    acls: job.acls.clone(),
                    additional_sids: job.additional_sids.clone(),
                    carved: false,
                    drive: file.drive.clone(),
                    tmp_fullpath: file.tmp_fullpath.clone(),
                };
                bits_info.push(bit_info);
            }
        }
    }

    // If we are carving and since this is ESE bits we currently do not combine job and file info
    if carve {
        let is_legacy = false;
        let read_result = raw_read_file(bits_path);
        if let Ok(result) = read_result {
            let (_carved_bits, carved_jobs, carved_files) = parse_carve(&result, is_legacy);
            add_carved_bits(&mut bits_info, &carved_jobs, &carved_files);
        } else {
            error!(
                "[bits] Could not read {bits_path} for carving: {:?}",
                read_result.unwrap_err()
            );
        }
    }
    Ok(bits_info)
}

/// Extract BITs info from ESE database
pub(crate) fn get_bits_ese(path: &str, table: &str) -> Result<Vec<Vec<TableDump>>, BitsError> {
    let catalog_result = get_catalog_info(path);
    let catalog = match catalog_result {
        Ok(result) => result,
        Err(err) => {
            error!("[bits] Failed to parse {path} catalog: {err:?}");
            return Err(BitsError::ParseEse);
        }
    };

    let mut info = table_info(&catalog, table);
    let pages_result = get_all_pages(path, info.table_page as u32);
    let pages = match pages_result {
        Ok(result) => result,
        Err(err) => {
            error!("[bits] Failed to get {table} pages at {path}: {err:?}");
            return Err(BitsError::ParseEse);
        }
    };

    let rows_results = get_page_data(path, &pages, &mut info, table);
    let table_rows = match rows_results {
        Ok(result) => result,
        Err(err) => {
            error!("[bits] Failed to parse {table} table at {path}: {err:?}");
            return Err(BitsError::ParseEse);
        }
    };

    Ok(table_rows.get(table).unwrap_or(&Vec::new()).clone())
}

/**
 * Parse older version (pre-Win10) of BITS which is a custom binary format
 */
pub(crate) fn parse_legacy_bits(
    systemdrive: char,
    carve: bool,
) -> Result<Vec<BitsInfo>, BitsError> {
    let mut bits_path =
        format!("{systemdrive}:\\ProgramData\\Microsoft\\Network\\Downloader\\qmgr0.dat");

    let mut bits = Vec::new();
    if is_file(&bits_path) {
        let mut results = legacy_bits(&bits_path, carve)?;
        bits.append(&mut results);
    }
    // Legacy BITS has two (2) files
    bits_path = format!("{systemdrive}:\\ProgramData\\Microsoft\\Network\\Downloader\\qmgr1.dat");
    if is_file(&bits_path) {
        let mut results = legacy_bits(&bits_path, carve)?;
        bits.append(&mut results);
    }

    Ok(bits)
}

/// Parse the older BITS file
pub(crate) fn legacy_bits(path: &str, carve: bool) -> Result<Vec<BitsInfo>, BitsError> {
    let read_results = raw_read_file(path);
    let bits_data = match read_results {
        Ok(results) => results,
        Err(err) => {
            error!("[bits] Could not read file {path}: {err:?}");
            return Err(BitsError::ReadFile);
        }
    };
    let mut bits = get_legacy_jobs(&bits_data)?;

    if carve {
        let is_legacy = false;
        let (_carved_bits, carved_jobs, carved_files) = parse_carve(&bits_data, is_legacy);
        add_carved_bits(&mut bits, &carved_jobs, &carved_files);
    }
    Ok(bits)
}

/**
 * When BITS entries are deleted the data is not actually removed from the file  
 * This makes it possible to carve out older entries, however some parts of the old entries may be overwritten by new data
 * Carving is **best effort**  
 * For BITS in ESE format (Win10+) BITS jobs and files are separate tables but since we are scanning the whole ESE db  
 * we do not merge the jobs and file info since we cannot determine what links the tables
 */
fn parse_carve(data: &[u8], is_legacy: bool) -> WinBits {
    let results = carve_bits(data, is_legacy);
    match results {
        Ok((_, bits)) => bits,
        Err(_err) => {
            error!("[bits] Could not carve BITS data");
            (Vec::new(), Vec::new(), Vec::new())
        }
    }
}

fn add_carved_bits(bits: &mut Vec<BitsInfo>, jobs: &Vec<JobInfo>, files: &Vec<FileInfo>) {
    for job in jobs {
        let bit = BitsInfo {
            job_id: job.job_id.clone(),
            file_id: String::new(),
            owner_sid: job.owner_sid.clone(),
            created: job.created.clone(),
            modified: job.modified.clone(),
            completed: job.completed.clone(),
            expiration: job.expiration.clone(),
            bytes_downloaded: 0,
            bytes_transferred: 0,
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
            full_path: String::new(),
            filename: String::new(),
            target_path: job.target_path.clone(),
            tmp_file: String::new(),
            volume: String::new(),
            url: String::new(),
            timeout: job.timeout,
            retry_delay: job.retry_delay,
            transient_error_count: job.transient_error_count,
            acls: job.acls.clone(),
            additional_sids: job.additional_sids.clone(),
            carved: true,
            drive: String::new(),
            tmp_fullpath: String::new(),
        };
        bits.push(bit);
    }

    for file in files {
        let bit = BitsInfo {
            job_id: String::new(),
            file_id: file.file_id.clone(),
            owner_sid: String::new(),
            created: String::new(),
            modified: String::new(),
            completed: String::new(),
            expiration: String::new(),
            bytes_downloaded: file.download_bytes_size,
            bytes_transferred: file.transfer_bytes_size,
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
            full_path: file.full_path.clone(),
            filename: file.filename.clone(),
            target_path: String::new(),
            tmp_file: file.tmp_fullpath.clone(),
            volume: file.volume.clone(),
            url: file.url.clone(),
            timeout: 0,
            retry_delay: 0,
            transient_error_count: 0,
            acls: Vec::new(),
            additional_sids: Vec::new(),
            carved: true,
            drive: String::new(),
            tmp_fullpath: String::new(),
        };
        bits.push(bit);
    }
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::parse_ese_bits;
    use crate::{
        artifacts::os::windows::bits::background::{
            get_bits_ese, legacy_bits, parse_carve, parse_legacy_bits,
        },
        filesystem::files::read_file,
    };
    use std::path::PathBuf;

    #[test]
    fn test_parse_ese_bits() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\ese\\win10\\qmgr.db");
        let results = parse_ese_bits(test_location.to_str().unwrap(), false).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_get_bits_ese() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\ese\\win10\\qmgr.db");
        let results = get_bits_ese(test_location.to_str().unwrap(), "Files").unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_parse_legacy_bits() {
        let results = parse_legacy_bits('C', false).unwrap();
        assert_eq!(results.is_empty(), true);
    }

    #[test]
    fn test_legacy_bits() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\bits\\win81\\qmgr0.dat");
        let results = legacy_bits(&test_location.to_str().unwrap(), false).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_parse_carve() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/ese/win10/qmgr.db");
        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let (_, jobs, files) = parse_carve(&data, false);
        assert_eq!(jobs.len(), 106);
        assert_eq!(files.len(), 41);
    }
}
