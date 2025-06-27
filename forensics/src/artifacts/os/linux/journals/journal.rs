use super::{
    error::JournalError,
    header::{IncompatFlags, JournalHeader},
    objects::{
        array::EntryArray,
        header::{ObjectHeader, ObjectType},
    },
};
use crate::{filesystem::files::file_reader, structs::toml::Output};
use common::linux::Journal;
use log::error;
use std::{collections::HashSet, fs::File, io::Read};

/// Parse provided `Journal` file path. Will output results when finished. Use `parse_journal_file` if you want the results
pub(crate) async fn parse_journal(
    path: &str,
    output: &mut Output,
    filter: bool,
    start_time: u64,
) -> Result<(), JournalError> {
    let reader_result = file_reader(path);
    let mut reader = match reader_result {
        Ok(result) => result,
        Err(err) => {
            error!("[journal] Could not create reader for file {path}: {err:?}");
            return Err(JournalError::ReaderError);
        }
    };

    // We technically only need first 232 bytes but version 252 is 264 bytes in size
    let mut header_buff = [0; 264];
    if reader.read(&mut header_buff).is_err() {
        error!("[journal] Could not read file header {path}");
        return Err(JournalError::ReadError);
    }

    let header_result = JournalHeader::parse_header(&header_buff);
    let journal_header = match header_result {
        Ok((_, result)) => result,
        Err(_err) => {
            error!("[journal] Could not parser file header {path}");
            return Err(JournalError::JournalHeader);
        }
    };

    let is_compact = journal_header
        .incompatible_flags
        .contains(&IncompatFlags::Compact);

    get_entries(
        &mut reader,
        journal_header.entry_array_offset,
        is_compact,
        output,
        filter,
        start_time,
    )
    .await?;

    Ok(())
}

/// Parse provided `Journal` file path. Returns parsed entries.
pub(crate) fn parse_journal_file(path: &str) -> Result<Vec<Journal>, JournalError> {
    let reader_result = file_reader(path);
    let mut reader = match reader_result {
        Ok(result) => result,
        Err(err) => {
            error!("[journal] Could not create reader for file {path}: {err:?}");
            return Err(JournalError::ReaderError);
        }
    };

    // We technically only need first 232 bytes but version 252 is 264 bytes in size
    let mut header_buff = [0; 264];
    if reader.read(&mut header_buff).is_err() {
        error!("[journal] Could not read file header {path}");
        return Err(JournalError::ReadError);
    }

    let header_result = JournalHeader::parse_header(&header_buff);
    let journal_header = match header_result {
        Ok((_, result)) => result,
        Err(_err) => {
            error!("[journal] Could not parser file header {path}");
            return Err(JournalError::JournalHeader);
        }
    };

    let is_compact = journal_header
        .incompatible_flags
        .contains(&IncompatFlags::Compact);

    let mut offset = journal_header.entry_array_offset;
    // Track offsets to make sure we do not encounter infinite loops
    let mut offset_tracker: HashSet<u64> = HashSet::new();
    offset_tracker.insert(offset);

    let last_entry = 0;

    let mut entries = EntryArray {
        entries: Vec::new(),
        next_entry_array_offset: 0,
    };

    while offset != last_entry {
        let object_header = ObjectHeader::parse_header(&mut reader, offset)?;
        if object_header.obj_type != ObjectType::EntryArray {
            error!(
                "[journal] Did not get Entry Array type at entry_array_offset. Got: {:?}. Exiting early",
                object_header.obj_type
            );
            break;
        }

        let entry_result =
            EntryArray::walk_all_entries(&mut reader, &object_header.payload, is_compact);
        let mut entry_array = match entry_result {
            Ok((_, result)) => result,
            Err(_err) => {
                error!("[journal] Could not walk journal entries. Exiting early");
                break;
            }
        };

        entries.entries.append(&mut entry_array.entries);
        offset = entry_array.next_entry_array_offset;

        if offset_tracker.contains(&offset) {
            error!("[journal] Found recursive offset. Exiting now");
            break;
        }
    }

    let messages = EntryArray::parse_messages(&entries.entries);

    Ok(messages)
}

/// Loop through the `Journal` entries and get the data
async fn get_entries(
    reader: &mut File,
    array_offset: u64,
    is_compact: bool,
    output: &mut Output,
    filter: bool,
    start_time: u64,
) -> Result<(), JournalError> {
    let mut offset = array_offset;
    let last_entry = 0;

    // Track offsets to make sure we do not encounter infinite loops
    let mut offset_tracker: HashSet<u64> = HashSet::new();
    offset_tracker.insert(offset);

    while offset != last_entry {
        let object_header = ObjectHeader::parse_header(reader, offset)?;
        if object_header.obj_type != ObjectType::EntryArray {
            error!(
                "[journal] Did not get Entry Array type at entry_array_offset. Got: {:?}. Exiting early",
                object_header.obj_type
            );
            break;
        }

        let entry_result = EntryArray::walk_entries(
            reader,
            &object_header.payload,
            is_compact,
            output,
            filter,
            start_time,
        )
        .await;
        let next_offset = match entry_result {
            Ok((_, result)) => result,
            Err(_err) => {
                error!("[journal] Could not walk journal entries. Exiting early");
                break;
            }
        };
        offset = next_offset;
        if offset_tracker.contains(&offset) {
            error!("[journal] Found recursive offset. Exiting now");
            break;
        }

        offset_tracker.insert(offset);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{get_entries, parse_journal};
    use crate::{
        artifacts::os::linux::journals::journal::parse_journal_file,
        filesystem::files::file_reader, structs::toml::Output,
    };
    use std::path::PathBuf;

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("json"),
            compress,
            timeline: false,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        }
    }

    #[test]
    fn test_parse_journal() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux/journal/user-1000@e755452aab34485787b6d73f3035fb8c-000000000000068d-0005ff8ae923c73b.journal");
        let mut output = output_options("journal_test", "local", "./tmp", false);

        parse_journal(&test_location.display().to_string(), &mut output, false, 0).unwrap();
    }

    #[test]
    fn test_get_entries() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux/journal/user-1000@e755452aab34485787b6d73f3035fb8c-000000000000068d-0005ff8ae923c73b.journal");

        let mut reader = file_reader(&test_location.display().to_string()).unwrap();
        let mut output = output_options("journal_test", "local", "./tmp", false);
        get_entries(&mut reader, 3738992, true, &mut output, false, 0).unwrap();
    }

    #[test]
    fn test_parse_journal_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux/journal/user-1000@e755452aab34485787b6d73f3035fb8c-000000000000068d-0005ff8ae923c73b.journal");

        let result = parse_journal_file(&test_location.display().to_string()).unwrap();
        assert_eq!(result.len(), 410);
    }

    #[test]
    fn test_parse_journal_bad_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux/journal/bad_recursive.journal");

        let result = parse_journal_file(&test_location.display().to_string()).unwrap();
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_parse_journal_bad() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux/journal/bad_recursive.journal");
        let mut output = output_options("journal_test", "local", "./tmp", false);

        parse_journal(&test_location.display().to_string(), &mut output, false, 0).unwrap();
    }
}
