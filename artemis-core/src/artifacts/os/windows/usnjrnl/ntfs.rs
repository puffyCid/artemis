use crate::{
    artifacts::os::windows::usnjrnl::{error::UsnJrnlError, journal::UsnJrnlFormat},
    filesystem::{
        files::file_extension,
        ntfs::{raw_files::read_attribute, setup::setup_ntfs_parser},
    },
};
use common::windows::UsnJrnlEntry;
use log::error;

/// Grab `UsnJrnl` entries by reading the $J ADS attribute and parsing its data runs
pub(crate) fn parse_usnjrnl_data(drive: &char) -> Result<Vec<UsnJrnlEntry>, UsnJrnlError> {
    let data = get_data(drive)?;
    let ntfs_parser_result = setup_ntfs_parser(drive);
    let mut ntfs_parser = match ntfs_parser_result {
        Ok(result) => result,
        Err(err) => {
            error!("[usnjrnl] Cannot setup NTFS parser: {err:?}");
            return Err(UsnJrnlError::Parser);
        }
    };

    let mut usnjrnl_entries = Vec::new();
    // UsnJrnl is composed of multiple data runs
    // Each data run we grabbed contain bytes that contain UsnJrnl entries
    // We do not need to concat the data in order to parse the UsnJrnl format, we can just loop through each data run
    let usnjrnl_result =
        UsnJrnlFormat::parse_usnjrnl(&data, &ntfs_parser.ntfs, &mut ntfs_parser.fs);
    match usnjrnl_result {
        Ok((_, result)) => {
            for jrnl_entry in result {
                let path = if jrnl_entry.full_path.is_empty() {
                    String::new()
                } else {
                    format!("\\{}", jrnl_entry.full_path)
                };
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
                    full_path: format!("{drive}:{}\\{}", path, jrnl_entry.name),
                    filename: jrnl_entry.name,
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

#[cfg(test)]
mod tests {
    use super::{get_data, parse_usnjrnl_data};

    #[test]
    #[ignore = "Parses the whole USNJrnl data"]
    fn test_parse_usnjrnl_data() {
        let result = parse_usnjrnl_data(&'C').unwrap();
        assert!(result.len() > 20)
    }

    #[test]
    #[ignore = "Takes a long time"]
    fn test_get_data() {
        let result = get_data(&'C').unwrap();
        assert!(result.len() > 20)
    }
}
