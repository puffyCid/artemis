use crate::{
    artifacts::os::windows::{
        artifacts::output_data,
        mft::reader::setup_mft_reader_windows,
        usnjrnl::{error::UsnJrnlError, journal::UsnJrnlFormat},
    },
    filesystem::{
        files::{file_extension, read_file},
        ntfs::{raw_files::read_attribute, setup::setup_ntfs_parser},
    },
    structs::toml::Output,
};
use common::windows::UsnJrnlEntry;
use log::error;
use std::collections::{HashMap, HashSet};

/// Grab `UsnJrnl` entries by reading the $J ADS attribute and parsing its data runs
pub(crate) fn parse_usnjrnl_data(
    drive: char,
    mft: &str,
    output: &mut Output,
    filter: bool,
    start_time: u64,
) -> Result<(), UsnJrnlError> {
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

    let mut journal_cache = HashMap::new();
    // UsnJrnl is composed of multiple data runs
    // Each data run we grabbed contain bytes that contain UsnJrnl entries
    // We do not need to concat the data in order to parse the UsnJrnl format, we can just loop through each data run
    let mut result = match UsnJrnlFormat::parse_usnjrnl(
        &data,
        &mut ntfs_parser.fs,
        Some(&ntfs_file),
        &mut journal_cache,
    ) {
        Ok((_, result)) => result,
        Err(_err) => {
            // We might get errors if we try to parse an entry that has not yet been fully written to the UsnJrnl
            error!("[usnjrnl] Issue with parsing whole UsnJrnl");
            return Err(UsnJrnlError::Parser);
        }
    };

    extract_entries(
        &mut result,
        Some(output),
        filter,
        start_time,
        &journal_cache,
        &format!("{drive}:\\$Extend\\$UsnJrnl:$J"),
        &drive.to_string(),
    )?;

    Ok(())
}

/// Parse the `UsnJrnl` file at provided path and return all entriess
pub(crate) fn get_usnjrnl_path(drive: char, mft: &str) -> Result<Vec<UsnJrnlEntry>, UsnJrnlError> {
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

    let mut journal_cache = HashMap::new();
    // UsnJrnl is composed of multiple data runs
    // Each data run we grabbed contain bytes that contain UsnJrnl entries
    // We do not need to concat the data in order to parse the UsnJrnl format, we can just loop through each data run
    let mut result = match UsnJrnlFormat::parse_usnjrnl(
        &data,
        &mut ntfs_parser.fs,
        Some(&ntfs_file),
        &mut journal_cache,
    ) {
        Ok((_, result)) => result,
        Err(_err) => {
            // We might get errors if we try to parse an entry that has not yet been fully written to the UsnJrnl
            error!("[usnjrnl] Issue with parsing whole UsnJrnl.");
            return Err(UsnJrnlError::Parser);
        }
    };
    extract_entries(
        &mut result,
        None,
        false,
        0,
        &journal_cache,
        &format!("{drive}:\\$Extend\\$UsnJrnl:$J"),
        &drive.to_string(),
    )
}

pub(crate) fn get_usnjrnl_alt_path(
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
    let mut entries = match entries_result {
        Ok((_, results)) => results,
        Err(_err) => {
            error!("[usnjrnl] Could nt parse UsnJrnl file {path}");
            return Err(UsnJrnlError::Parser);
        }
    };
    extract_entries(&mut entries, None, false, 0, &journal_cache, path, "")
}

/// Parse the `UsnJrnl` file at provided path and output the results
pub(crate) fn get_usnjrnl_path_stream(
    path: &str,
    mft_path: &Option<String>,
    output: &mut Output,
    filter: bool,
    start_time: u64,
) -> Result<(), UsnJrnlError> {
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
    let mut entries = match entries_result {
        Ok((_, results)) => results,
        Err(_err) => {
            error!("[usnjrnl] Could nt parse UsnJrnl file {path}");
            return Err(UsnJrnlError::Parser);
        }
    };

    extract_entries(
        &mut entries,
        Some(output),
        filter,
        start_time,
        &journal_cache,
        path,
        "",
    )?;
    Ok(())
}

/// Loop through the parsed entries
fn extract_entries(
    data: &mut [UsnJrnlFormat],
    mut output: Option<&mut Output>,
    filter: bool,
    start_time: u64,
    journal_cache: &HashMap<String, UsnJrnlFormat>,
    path: &str,
    drive: &str,
) -> Result<Vec<UsnJrnlEntry>, UsnJrnlError> {
    let mut usnjrnl_entries = Vec::new();
    for jrnl_entry in data {
        // Try the cached usnjrnl paths before we give up
        if jrnl_entry.full_path.starts_with("$OrphanFiles\\") {
            let mut tracker = HashSet::new();
            let path = lookup_journal_cache(journal_cache, jrnl_entry, &mut tracker);
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
            update_time: jrnl_entry.update_time.clone(),
            update_reason: jrnl_entry.update_reason.clone(),
            update_source_flags: jrnl_entry.update_source_flags.clone(),
            security_descriptor_id: jrnl_entry.security_descriptor_id,
            file_attributes: jrnl_entry.file_attributes.clone(),
            extension: file_extension(&jrnl_entry.name),
            full_path: jrnl_entry.full_path.clone(),
            filename: jrnl_entry.name.clone(),
            drive: drive.to_string(),
            evidence: path.to_string(),
        };
        usnjrnl_entries.push(entry);
        let limit = 1000;
        // If we are give an output structure we will dump the results
        if let Some(out) = output.as_deref_mut()
            && usnjrnl_entries.len() == limit
        {
            let _ = output_usnjnl(&usnjrnl_entries, out, filter, start_time);
            usnjrnl_entries = Vec::new();
        }
    }

    if let Some(out) = output
        && !usnjrnl_entries.is_empty()
    {
        let _ = output_usnjnl(&usnjrnl_entries, out, filter, start_time);
    }

    // If no output structure was provided. Return all parsed entries
    Ok(usnjrnl_entries)
}

/// `UsnJrnl` data is in an alternative data stream (ADS) at \<drive\>\\$Extend\\$UsnJrnl:$J (where $J is the ADS name)
fn get_data(drive: char) -> Result<Vec<u8>, UsnJrnlError> {
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

fn output_usnjnl(
    entries: &[UsnJrnlEntry],
    output: &mut Output,
    filter: bool,
    start_time: u64,
) -> Result<(), UsnJrnlError> {
    if entries.is_empty() {
        return Ok(());
    }

    let serde_data_result = serde_json::to_value(entries);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[usnjrnl] Failed to serialize UsnJrnl entries: {err:?}");
            return Err(UsnJrnlError::Serialize);
        }
    };
    if let Err(err) = output_data(&mut serde_data, "usnjrnl", output, start_time, filter) {
        error!("[usnjrnl] Could not output UsnJrnl entries: {err:?}");
        return Err(UsnJrnlError::OutputData);
    }

    Ok(())
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::{get_data, parse_usnjrnl_data};
    use crate::{
        artifacts::os::windows::usnjrnl::ntfs::get_usnjrnl_alt_path, structs::toml::Output,
    };
    use std::path::PathBuf;

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
            compress,
            endpoint_id: String::from("abcd"),
            output: output.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_parse_usnjrnl_data() {
        let mut output = output_options("usnjrnl_temp", "local", "./tmp", false);

        parse_usnjrnl_data('C', "C:\\$MFT", &mut output, false, 0).unwrap();
    }

    #[test]
    fn test_get_data() {
        let result = get_data('C').unwrap();
        assert!(result.len() > 20)
    }

    #[test]
    fn test_get_usnjrnl_alt_path() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\usnjrnl\\win11\\usnjrnl.raw");

        let results = get_usnjrnl_alt_path(test_location.to_str().unwrap(), &None).unwrap();
        assert_eq!(results.len(), 1);
    }
}
