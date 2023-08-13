use crate::utils::{
    nom_helper::{
        nom_signed_four_bytes, nom_unsigned_eight_bytes, nom_unsigned_four_bytes,
        nom_unsigned_two_bytes, Endian,
    },
    strings::{extract_utf16_string, extract_utf8_string},
    time::filetime_to_unixepoch,
    uuid::format_guid_le_bytes,
};
use nom::bytes::complete::take;
use serde::Serialize;
use std::mem::size_of;

#[derive(Debug)]
pub(crate) struct DestList {
    pub(crate) version: DestVersion,
    pub(crate) number_entries: u32,
    pub(crate) number_pinned_entries: u32,
    pub(crate) last_entry: u32,
    pub(crate) last_revision: u32,
    pub(crate) entries: Vec<DestEntries>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DestEntries {
    pub(crate) droid_volume_id: String,
    pub(crate) droid_file_id: String,
    pub(crate) birth_droid_volume_id: String,
    pub(crate) birth_droid_file_id: String,
    pub(crate) hostname: String,
    pub(crate) entry: u32,
    pub(crate) modified: i64,
    pub(crate) pin_status: PinStatus,
    pub(crate) path: String,
}

#[derive(Debug, PartialEq)]
pub(crate) enum DestVersion {
    Win7,
    Win10,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) enum PinStatus {
    Pinned,
    NotPinned,
    None,
}

/// Parse the DestList OLE Directory. Contains metadata about LNK data (JumpList entries)
pub(crate) fn parse_destlist(data: &[u8]) -> nom::IResult<&[u8], DestList> {
    let (input, version_data) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (input, number_entries) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, number_pinned_entries) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let (input, _unknown) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, last_entry) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, _unknown) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, last_revision) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (mut input, _unknown) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let win7 = 1;
    let win10 = 3;
    let version = if version_data == win7 {
        DestVersion::Win7
    } else if version_data >= win10 {
        DestVersion::Win10
    } else {
        DestVersion::Unknown
    };

    let mut dest_data = DestList {
        version,
        number_entries,
        number_pinned_entries,
        last_entry,
        last_revision,
        entries: Vec::new(),
    };

    // Get all the metadata associated with Jumplist entries
    while !input.is_empty() {
        let (remaining_data, _unknown) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (remaining_data, droid_volume) = take(size_of::<u128>())(remaining_data)?;
        let (remaining_data, droid_file) = take(size_of::<u128>())(remaining_data)?;
        let (remaining_data, birth_volume) = take(size_of::<u128>())(remaining_data)?;
        let (remaining_data, birth_file) = take(size_of::<u128>())(remaining_data)?;

        let (remaining_data, hostname_data) = take(size_of::<u128>())(remaining_data)?;
        let (remaining_data, entry) = nom_unsigned_four_bytes(remaining_data, Endian::Le)?;
        let (remaining_data, _unknown) = nom_unsigned_four_bytes(remaining_data, Endian::Le)?;
        let (remaining_data, _unknown) = nom_unsigned_four_bytes(remaining_data, Endian::Le)?;

        let (remaining_data, modified) = nom_unsigned_eight_bytes(remaining_data, Endian::Le)?;
        let (mut remaining_data, pin_data) = nom_signed_four_bytes(remaining_data, Endian::Le)?;

        let not_pin = -1;
        // Anything that is not -1 is pinned
        let pin_status = if pin_data == not_pin {
            PinStatus::NotPinned
        } else {
            PinStatus::Pinned
        };

        // Windows 10 introduced three (3) additional data types
        if dest_data.version == DestVersion::Win10 {
            let (remaining, _unknown) = nom_unsigned_four_bytes(remaining_data, Endian::Le)?;
            // Unsure if this is really access count
            let (remaining, _access_count) = nom_unsigned_four_bytes(remaining, Endian::Le)?;
            let (remaining, _unknown) = nom_unsigned_eight_bytes(remaining, Endian::Le)?;
            remaining_data = remaining;
        }

        let (remaining_data, path_size) = nom_unsigned_two_bytes(remaining_data, Endian::Le)?;
        let utf_adjust: u32 = 2;
        let (remaining_data, path_data) = take(path_size as u32 * utf_adjust)(remaining_data)?;

        // Check for end of string character. Sometimes the path has it
        if !remaining_data.is_empty() {
            let (next_data, end_of_string) = nom_unsigned_four_bytes(remaining_data, Endian::Le)?;
            if end_of_string == 0 {
                input = next_data;
            } else {
                input = remaining_data;
            }
        } else {
            input = remaining_data;
        }

        let entry = DestEntries {
            droid_volume_id: format_guid_le_bytes(droid_volume),
            droid_file_id: format_guid_le_bytes(droid_file),
            birth_droid_volume_id: format_guid_le_bytes(birth_volume),
            birth_droid_file_id: format_guid_le_bytes(birth_file),
            hostname: extract_utf8_string(hostname_data),
            entry,
            modified: filetime_to_unixepoch(&modified),
            pin_status,
            path: extract_utf16_string(path_data),
        };

        dest_data.entries.push(entry);
    }

    Ok((data, dest_data))
}

#[cfg(test)]
mod tests {
    use super::parse_destlist;
    use crate::{artifacts::os::windows::ole::olecf::OleData, filesystem::files::read_file};
    use std::path::PathBuf;

    #[test]
    fn test_parse_destlist() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push(
            "tests/test_data/dfir/windows/jumplists/win7/1b4dd67f29cb1962.automaticDestinations-ms",
        );
        let data = read_file(&test_location.display().to_string()).unwrap();

        let (_, jump_ole) = OleData::parse_ole(&data).unwrap();

        for entry in jump_ole {
            if entry.name != "DestList" {
                continue;
            }

            let (_, result) = parse_destlist(&entry.data).unwrap();
            assert_eq!(result.number_entries, 4);
            assert_eq!(result.last_entry, 4);
            assert_eq!(result.last_revision, 4);
            assert_eq!(result.entries.len(), 4);
            assert_eq!(result.entries[0].hostname, "win7x64");
            assert_eq!(result.entries[0].entry, 4);
            assert_eq!(
                result.entries[0].droid_file_id,
                "ba034b9e-bc96-11e5-b231-00155d016d0b"
            );
        }
    }
}
