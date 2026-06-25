use crate::{
    artifacts::os::windows::{
        mft::reader::setup_mft_reader_windows,
        usnjrnl::{error::UsnJrnlError, journal::UsnJrnlFormat},
    },
    filesystem::{
        files::{file_extension, read_file},
        ntfs::{raw_files::read_attribute, setup::setup_ntfs_parser},
    },
    output::{manager::OutputManager, record::serialize_records_to_stream},
    structs::artifacts::os::windows::UsnJrnlOptions,
};
use common::windows::UsnJrnlEntry;
use std::{
    collections::{HashMap, HashSet},
    mem::take,
};
use tracing::error;

/// Grab `UsnJrnl` entries by reading the $J ADS attribute and parsing its data runs
pub(crate) fn parse_usnjrnl_data(
    drive: char,
    mft: &str,
    manager: &mut OutputManager,
    options: &UsnJrnlOptions,
) -> Result<(), UsnJrnlError> {
    let data = get_data(drive)?;
    let ntfs_parser_result = setup_ntfs_parser(drive);
    let mut ntfs_parser = match ntfs_parser_result {
        Ok(result) => result,
        Err(err) => {
            error!("Cannot setup NTFS parser: {err:?}");
            return Err(UsnJrnlError::Parser);
        }
    };

    let ntfs_file = match setup_mft_reader_windows(&ntfs_parser.ntfs, &mut ntfs_parser.fs, mft) {
        Ok(result) => result,
        Err(err) => {
            error!("Cannot read the MFT file: {err:?}");
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
            error!("Issue with parsing whole UsnJrnl");
            return Err(UsnJrnlError::Parser);
        }
    };

    extract_entries(
        &mut result,
        Some(manager),
        Some(options),
        &journal_cache,
        &format!("{drive}:\\$Extend\\$UsnJrnl:$J"),
        &drive.to_string(),
    )?;

    Ok(())
}

/// Parse the `UsnJrnl` file at provided path and return all entries
pub(crate) fn get_usnjrnl_path(drive: char, mft: &str) -> Result<Vec<UsnJrnlEntry>, UsnJrnlError> {
    let data = get_data(drive)?;
    let ntfs_parser_result = setup_ntfs_parser(drive);
    let mut ntfs_parser = match ntfs_parser_result {
        Ok(result) => result,
        Err(err) => {
            error!("Cannot setup NTFS parser: {err:?}");
            return Err(UsnJrnlError::Parser);
        }
    };

    let ntfs_file = match setup_mft_reader_windows(&ntfs_parser.ntfs, &mut ntfs_parser.fs, mft) {
        Ok(result) => result,
        Err(err) => {
            error!("Cannot read the MFT file: {err:?}");
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
            error!("Issue with parsing whole UsnJrnl.");
            return Err(UsnJrnlError::Parser);
        }
    };
    extract_entries(
        &mut result,
        None,
        None,
        &journal_cache,
        &format!("{drive}:\\$Extend\\$UsnJrnl:$J"),
        &drive.to_string(),
    )
}

/// Parse the `UsnJrnl` file at provided path and return results
pub(crate) fn get_usnjrnl_alt_path(
    path: &str,
    mft_path: &Option<String>,
) -> Result<Vec<UsnJrnlEntry>, UsnJrnlError> {
    let data_result = read_file(path);
    let data = match data_result {
        Ok(result) => result,
        Err(err) => {
            error!("Could not read UsnJrnl file {path}: {err:?}");
            return Err(UsnJrnlError::ReadFile);
        }
    };
    let mut journal_cache = HashMap::new();

    let entries_result =
        UsnJrnlFormat::parse_usnjrnl_no_parent(&data, mft_path, &mut journal_cache);
    let mut entries = match entries_result {
        Ok((_, results)) => results,
        Err(_err) => {
            error!("Could nt parse UsnJrnl file {path}");
            return Err(UsnJrnlError::Parser);
        }
    };
    // Drive is empty because we cannot be certain what the source drive is
    extract_entries(&mut entries, None, None, &journal_cache, path, "")
}

/// Parse the `UsnJrnl` file at provided path and output the results
pub(crate) fn get_usnjrnl_path_stream(
    path: &str,
    mft_path: &Option<String>,
    manager: &mut OutputManager,
    options: &UsnJrnlOptions,
) -> Result<(), UsnJrnlError> {
    let data_result = read_file(path);
    let data = match data_result {
        Ok(result) => result,
        Err(err) => {
            error!("Could not read UsnJrnl file {path}: {err:?}");
            return Err(UsnJrnlError::ReadFile);
        }
    };
    let mut journal_cache = HashMap::new();

    let entries_result =
        UsnJrnlFormat::parse_usnjrnl_no_parent(&data, mft_path, &mut journal_cache);
    let mut entries = match entries_result {
        Ok((_, results)) => results,
        Err(_err) => {
            error!("Could nt parse UsnJrnl file {path}");
            return Err(UsnJrnlError::Parser);
        }
    };

    extract_entries(
        &mut entries,
        Some(manager),
        Some(options),
        &journal_cache,
        path,
        // Drive is empty because we cannot be certain what the source drive is
        "",
    )?;
    Ok(())
}

/// Loop through the parsed entries
fn extract_entries(
    data: &mut [UsnJrnlFormat],
    mut manager: Option<&mut OutputManager>,
    options: Option<&UsnJrnlOptions>,
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
        if let Some(out) = manager.as_deref_mut()
            && usnjrnl_entries.len() == limit
            && let Some(opt) = options
        {
            let _ = output_usnjnl(take(&mut usnjrnl_entries), out, opt);
        }
    }

    if let Some(out) = manager
        && !usnjrnl_entries.is_empty()
        && let Some(opt) = options
    {
        let _ = output_usnjnl(take(&mut usnjrnl_entries), out, opt);
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
            error!("Could not read UsnJrnl $J attribute: {err:?}");
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

/// Output `UsnJrnl` entries based on `Output` structure
fn output_usnjnl(
    entries: Vec<UsnJrnlEntry>,
    manager: &mut OutputManager,
    options: &UsnJrnlOptions,
) -> Result<(), UsnJrnlError> {
    if entries.is_empty() {
        return Ok(());
    }

    let mut records = match serialize_records_to_stream(entries) {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to serialize UsnJrnl entries: {err:?}");
            return Err(UsnJrnlError::Serialize);
        }
    };
    let artifact_name = "usnjrnl";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("Could not output UsnJrnl entries: {err:?}");
        return Err(UsnJrnlError::OutputData);
    }

    Ok(())
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::{get_data, parse_usnjrnl_data};
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use crate::{
        artifacts::os::windows::usnjrnl::ntfs::{get_usnjrnl_alt_path, get_usnjrnl_path_stream},
        filesystem::metadata::glob_paths,
        output::manager::OutputManager,
        structs::artifacts::os::windows::UsnJrnlOptions,
    };
    use common::windows::UsnJrnlEntry;
    use std::{
        fs::File,
        io::{BufRead, BufReader},
        path::PathBuf,
    };

    fn output_options(name: &str, directory: &str, compress: bool) -> OutputManager {
        let config = OutputConfig {
            name: name.to_string(),
            directory: PathBuf::from(directory),
            format: OutputFormat::Jsonl,
            compress,
            endpoint_id: String::from("abcd"),
            destination: OutputDestination::Local,
            ..Default::default()
        };
        OutputManager::new(config).unwrap()
    }

    #[test]
    fn test_parse_usnjrnl_data() {
        let mut output = output_options("usnjrnl_temp", "./tmp", false);
        let params = UsnJrnlOptions {
            alt_drive: None,
            alt_file: None,
            alt_mft: None,
        };
        parse_usnjrnl_data('C', "C:\\$MFT", &mut output, &params).unwrap();
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

    #[test]
    fn test_get_usnjrnl_path_stream() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\dfir\\windows\\usnjrnl\\win11\\$J");
        let mut out = output_options("usnjrnl_stream_alt", "./tmp", false);
        let params = UsnJrnlOptions {
            alt_drive: None,
            alt_file: None,
            alt_mft: None,
        };
        get_usnjrnl_path_stream(test_location.to_str().unwrap(), &None, &mut out, &params).unwrap();
        let mut output_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        output_location.push("tmp/usnjrnl_stream_alt/*");

        let results = glob_paths(output_location.to_str().unwrap()).unwrap();
        let mut count = 0;
        for result in results {
            if !result.filename.contains("usnjrnl_") {
                continue;
            }

            // Output is in JSONL based on the struct above!
            let file = File::open(&result.full_path).unwrap();
            let reader = BufReader::new(file);
            for (_, line) in reader.lines().enumerate() {
                let value = line.unwrap();

                let info: UsnJrnlEntry = serde_json::from_str(&value).unwrap();
                if info.filename.is_empty() {
                    panic!("no filename?")
                }
                assert_ne!(info.update_time, "1970-01-01T00:00:00.000Z");
                count += 1;
            }
        }

        assert_eq!(count, 133099);
    }
}
