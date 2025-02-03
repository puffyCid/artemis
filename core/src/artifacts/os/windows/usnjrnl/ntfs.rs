use crate::{
    artifacts::os::windows::{
        mft::reader::setup_mft_reader_windows,
        usnjrnl::{error::UsnJrnlError, journal::UsnJrnlFormat},
    },
    filesystem::{
        files::{file_extension, read_file},
        ntfs::{raw_files::read_attribute, setup::setup_ntfs_parser},
    },
};
use common::windows::UsnJrnlEntry;
use log::error;
use std::collections::{HashMap, HashSet};

/// Grab `UsnJrnl` entries by reading the $J ADS attribute and parsing its data runs
pub(crate) fn parse_usnjrnl_data(
    drive: &char,
    mft: &str,
) -> Result<Vec<UsnJrnlEntry>, UsnJrnlError> {
    let data = get_data(drive)?;
    let ntfs_parser_result = setup_ntfs_parser(drive);
    let mut ntfs_parser = match ntfs_parser_result {
        Ok(result) => result,
        Err(err) => {
            error!("[usnjrnl] Cannot setup NTFS parser: {err:?}");
            return Err(UsnJrnlError::Parser);
        }
    };

    let ntfs_file = match setup_mft_reader_windows(&ntfs_parser.ntfs, &mut ntfs_parser.fs, mft) {
        Ok(result) => result,
        Err(err) => {
            error!("[usnjrnl] Cannot read the MFT file: {err:?}");
            return Err(UsnJrnlError::ReadFile);
        }
    };

    let mut usnjrnl_entries = Vec::new();
    let mut journal_cache = HashMap::new();
    // UsnJrnl is composed of multiple data runs
    // Each data run we grabbed contain bytes that contain UsnJrnl entries
    // We do not need to concat the data in order to parse the UsnJrnl format, we can just loop through each data run
    let usnjrnl_result = UsnJrnlFormat::parse_usnjrnl(
        &data,
        &mut ntfs_parser.fs,
        Some(&ntfs_file),
        &mut journal_cache,
    );
    match usnjrnl_result {
        Ok((_, result)) => {
            for mut jrnl_entry in result {
                // Try the cached usnjrnl paths before we give up
                if jrnl_entry.full_path.starts_with("$OrphanFiles\\") {
                    let mut tracker = HashSet::new();
                    let path = lookup_journal_cache(&journal_cache, &jrnl_entry, &mut tracker);
                    if !path.is_empty() {
                        jrnl_entry.full_path = path;
                    }
                }
                let entry = UsnJrnlEntry {
                    mft_entry: jrnl_entry.mft_entry,
                    mft_sequence: jrnl_entry.mft_sequence,
                    parent_mft_entry: jrnl_entry.parent_mft_entry,
                    parent_mft_sequence: jrnl_entry.parent_mft_sequence,
                    update_sequence_number: jrnl_entry.update_sequence_number,
                    update_time: jrnl_entry.update_time,
                    update_reason: jrnl_entry.update_reason,
                    update_source_flags: jrnl_entry.update_source_flags,
                    security_descriptor_id: jrnl_entry.security_descriptor_id,
                    file_attributes: jrnl_entry.file_attributes,
                    extension: file_extension(&jrnl_entry.name),
                    full_path: jrnl_entry.full_path,
                    filename: jrnl_entry.name,
                    drive: drive.to_string(),
                };
                usnjrnl_entries.push(entry);
            }
        }
        Err(_err) => {
            error!("[usnjrnl] Failed to parse UsnJrnl data");
            return Err(UsnJrnlError::Parser);
        }
    };
    Ok(usnjrnl_entries)
}

/// Parse the `UsnJrnl` file at provided path
pub(crate) fn get_usnjrnl_path(
    path: &str,
    mft_path: &Option<String>,
) -> Result<Vec<UsnJrnlEntry>, UsnJrnlError> {
    let data_result = read_file(path);
    let data = match data_result {
        Ok(result) => result,
        Err(err) => {
            error!("[usnjrnl] Could not read UsnJrnl file {path}: {err:?}");
            return Err(UsnJrnlError::ReadFile);
        }
    };
    let mut journal_cache = HashMap::new();

    let entries_result =
        UsnJrnlFormat::parse_usnjrnl_no_parent(&data, mft_path, &mut journal_cache);
    let entries = match entries_result {
        Ok((_, results)) => results,
        Err(_err) => {
            error!("[usnjrnl] Could nt parse UsnJrnl file {path}");
            return Err(UsnJrnlError::Parser);
        }
    };
    let mut usnjrnl_entries = Vec::new();
    for mut jrnl_entry in entries {
        // Try the cached usnjrnl paths before we give up
        if jrnl_entry.full_path.starts_with("$OrphanFiles\\") {
            let mut tracker = HashSet::new();
            let path = lookup_journal_cache(&journal_cache, &jrnl_entry, &mut tracker);
            if !path.is_empty() {
                jrnl_entry.full_path = path;
            }
        }
        let entry = UsnJrnlEntry {
            mft_entry: jrnl_entry.mft_entry,
            mft_sequence: jrnl_entry.mft_sequence,
            parent_mft_entry: jrnl_entry.parent_mft_entry,
            parent_mft_sequence: jrnl_entry.parent_mft_sequence,
            update_sequence_number: jrnl_entry.update_sequence_number,
            update_time: jrnl_entry.update_time,
            update_reason: jrnl_entry.update_reason,
            update_source_flags: jrnl_entry.update_source_flags,
            security_descriptor_id: jrnl_entry.security_descriptor_id,
            file_attributes: jrnl_entry.file_attributes,
            extension: file_extension(&jrnl_entry.name),
            full_path: jrnl_entry.full_path,
            filename: jrnl_entry.name,
            drive: String::new(),
        };
        usnjrnl_entries.push(entry);
    }
    Ok(usnjrnl_entries)
}

/// `UsnJrnl` data is in an alternative data stream (ADS) at \<drive\>\\$Extend\\$UsnJrnl:$J (where $J is the ADS name)
fn get_data(drive: &char) -> Result<Vec<u8>, UsnJrnlError> {
    let usn_path = format!("{drive}:\\$Extend\\$UsnJrnl");
    let attribute = "$J";
    // Read the $J attribute and get all the data runs
    let data_result = read_attribute(&usn_path, attribute);
    let data = match data_result {
        Ok(result) => result,
        Err(err) => {
            error!("[usnjrnl] Could not read UsnJrnl $J attribute: {err:?}");
            return Err(UsnJrnlError::Attribute);
        }
    };

    Ok(data)
}

/// Lookup cached `UsnJrnl` paths
fn lookup_journal_cache(
    cache: &HashMap<String, UsnJrnlFormat>,
    entry: &UsnJrnlFormat,
    tracker: &mut HashSet<String>,
) -> String {
    let mut path = String::new();
    let cache_path = format!("{}_{}", entry.parent_mft_entry, entry.parent_mft_sequence);

    if let Some(cache_hit) = cache.get(&cache_path) {
        if cache_hit.full_path.contains("$OrphanFiles\\") {
            tracker.insert(format!(
                "{}_{}",
                entry.parent_mft_entry, entry.parent_mft_sequence
            ));
            let parent = lookup_journal_cache(cache, cache_hit, tracker);
            path = format!("{parent}\\{}", entry.name);
        } else {
            path = format!("{}\\{}", cache_hit.full_path, entry.name);
        }
    }

    path
}
#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::{get_data, get_usnjrnl_path, parse_usnjrnl_data};
    use std::path::PathBuf;

    #[test]
    fn test_parse_usnjrnl_data() {
        let result = parse_usnjrnl_data(&'C', "C:\\$MFT").unwrap();
        assert!(result.len() > 20)
    }

    #[test]
    fn test_get_data() {
        let result = get_data(&'C').unwrap();
        assert!(result.len() > 20)
    }

    #[test]
    fn test_get_usnjrnl_path() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\usnjrnl\\win11\\usnjrnl.raw");

        let results = get_usnjrnl_path(test_location.to_str().unwrap(), &None).unwrap();
        assert_eq!(results.len(), 1);
    }
}
