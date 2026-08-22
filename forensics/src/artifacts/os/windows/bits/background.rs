use super::{
    carve::{WinBits, carve_bits},
    error::BitsError,
    files::get_files,
    jobs::{get_jobs, get_legacy_jobs},
};
use crate::{
    accessor::{access::Accessor, entry::handle::FileHandle},
    artifacts::os::windows::ese::{
        helper::{get_all_pages, get_catalog_info, get_page_data},
        tables::table_info,
    },
};
use common::windows::{BitsInfo, FileInfo, JobInfo, TableDump};
use tracing::error;

/**
 * Parse modern version (Win10+) of BITS which is an ESE database by dumping the `Jobs` and `Files` tables and parsing their contents  
 */
pub(crate) fn parse_ese_bits(handle: &FileHandle, carve: bool) -> Result<Vec<BitsInfo>, BitsError> {
    // Dump the Jobs and Files tables from the BITS database
    let files = get_bits_ese(handle, "Files")?;
    let jobs_info = get_bits_ese(handle, "Jobs")?;

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
                    evidence: handle.display_path(),
                };
                bits_info.push(bit_info);
            }
        }
    }

    // If we are carving and since this is ESE bits we currently do not combine job and file info
    if carve {
        let mut accessor = Accessor::with_defaults();
        let is_legacy = false;
        let read_result = accessor.read_file_handle(handle);
        if let Ok(result) = read_result {
            let (_carved_bits, carved_jobs, carved_files) =
                parse_carve(&result, is_legacy, &handle.display_path());
            add_carved_bits(
                &mut bits_info,
                carved_jobs,
                carved_files,
                &handle.display_path(),
            );
        } else {
            error!(
                "Could not read {} for carving: {:?}",
                handle.display_path(),
                read_result.unwrap_err()
            );
        }
    }
    Ok(bits_info)
}

/// Extract BITs info from ESE database
pub(crate) fn get_bits_ese(
    handle: &FileHandle,
    table: &str,
) -> Result<Vec<Vec<TableDump>>, BitsError> {
    let catalog_result = get_catalog_info(handle);
    let catalog = match catalog_result {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to parse {} catalog: {err:?}", handle.display_path());
            return Err(BitsError::ParseEse);
        }
    };

    let mut info = table_info(&catalog, table);
    let pages_result = get_all_pages(handle, info.table_page as u32);
    let pages = match pages_result {
        Ok(result) => result,
        Err(err) => {
            error!(
                "Failed to get {table} pages at {}: {err:?}",
                handle.display_path()
            );
            return Err(BitsError::ParseEse);
        }
    };

    let rows_results = get_page_data(handle, &pages, &mut info, table);
    let table_rows = match rows_results {
        Ok(result) => result,
        Err(err) => {
            error!(
                "Failed to parse {table} table at {}: {err:?}",
                handle.display_path()
            );
            return Err(BitsError::ParseEse);
        }
    };

    Ok(table_rows.get(table).unwrap_or(&Vec::new()).clone())
}

/// Parse the older BITS file
pub(crate) fn legacy_bits(handle: &FileHandle, carve: bool) -> Result<Vec<BitsInfo>, BitsError> {
    let read_results = Accessor::with_defaults().read_file_handle(handle);
    let bits_data = match read_results {
        Ok(results) => results,
        Err(err) => {
            error!("Could not read file {}: {err:?}", handle.display_path());
            return Err(BitsError::ReadFile);
        }
    };
    let mut bits = get_legacy_jobs(&bits_data, &handle.display_path())?;

    if carve {
        let is_legacy = false;
        let (_carved_bits, carved_jobs, carved_files) =
            parse_carve(&bits_data, is_legacy, &handle.display_path());
        add_carved_bits(&mut bits, carved_jobs, carved_files, &handle.display_path());
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
fn parse_carve(data: &[u8], is_legacy: bool, evidence: &str) -> WinBits {
    let results = carve_bits(data, is_legacy, evidence);
    match results {
        Ok((_, bits)) => bits,
        Err(_err) => {
            error!("Could not carve BITS data");
            (Vec::new(), Vec::new(), Vec::new())
        }
    }
}

/// Add the carved Jobs and Files to our parsed bits array
/// We cannot combine them
fn add_carved_bits(
    bits: &mut Vec<BitsInfo>,
    jobs: Vec<JobInfo>,
    files: Vec<FileInfo>,
    evidence: &str,
) {
    for job in jobs {
        let bit = BitsInfo {
            job_id: job.job_id,
            owner_sid: job.owner_sid,
            created: job.created,
            modified: job.modified,
            completed: job.completed,
            expiration: job.expiration,
            job_name: job.job_name,
            job_description: job.job_description,
            job_command: job.job_command,
            job_arguments: job.job_arguments,
            error_count: job.error_count,
            job_type: job.job_type,
            job_state: job.job_state,
            priority: job.priority,
            flags: job.flags,
            http_method: job.http_method,
            target_path: job.target_path,
            timeout: job.timeout,
            retry_delay: job.retry_delay,
            transient_error_count: job.transient_error_count,
            acls: job.acls,
            additional_sids: job.additional_sids,
            carved: true,
            evidence: evidence.to_string(),
            ..Default::default()
        };
        bits.push(bit);
    }

    for file in files {
        let bit = BitsInfo {
            file_id: file.file_id,
            bytes_downloaded: file.download_bytes_size,
            bytes_transferred: file.transfer_bytes_size,
            full_path: file.full_path,
            filename: file.filename,
            volume: file.volume,
            url: file.url,
            carved: true,
            evidence: evidence.to_string(),
            ..Default::default()
        };
        bits.push(bit);
    }
}

#[cfg(test)]
mod tests {
    use super::parse_ese_bits;
    use crate::{
        accessor::{access::Accessor, entry::handle::FileHandle},
        artifacts::os::windows::bits::background::{get_bits_ese, legacy_bits, parse_carve},
    };
    use std::path::PathBuf;

    #[test]
    fn test_parse_ese_bits() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\ese\\win10\\qmgr.db");
        let handle = FileHandle::host(test_location);
        let results = parse_ese_bits(&handle, false).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_get_bits_ese() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\ese\\win10\\qmgr.db");
        let handle = FileHandle::host(test_location);

        let results = get_bits_ese(&handle, "Files").unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_legacy_bits() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\bits\\win81\\qmgr0.dat");
        let handle = FileHandle::host(test_location);

        let results = legacy_bits(&handle, false).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_parse_carve() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/ese/win10/qmgr.db");
        let data = Accessor::with_defaults()
            .read_file(&test_location.to_str().unwrap())
            .unwrap();
        let (_, jobs, files) = parse_carve(&data, false, &test_location.to_str().unwrap());
        assert_eq!(jobs.len(), 106);
        assert_eq!(files.len(), 41);
    }
}
