use crate::{
    accessor::{
        filesystem::{helper::attributes::windows_attributes, ntfs::attributes::read_value_bytes},
        io::reader::{directory_from_display, extension_from_filename},
    },
    artifacts::os::windows::mft::attributes::filename::Filename,
    utils::time::filetime_to_iso,
};
use common::{
    files::{Attributes, EntryKind, FileNtfsInfo},
    windows::Namespace,
};
use nom::{
    bytes::complete::{take, take_until},
    number::complete::{le_u32, le_u64},
};
use ntfs::{NtfsAttributeType, NtfsFile};
use std::io::{Read, Seek};
use tracing::warn;

pub(super) fn recover_indx_slack<R: Read + Seek>(
    reader: &mut R,
    dir: &NtfsFile<'_>,
    parent: &str,
    depth: usize,
    drive: char,
    evidence: &str,
) -> Vec<FileNtfsInfo> {
    let mut attrs = dir.attributes();
    let mut recovered = Vec::new();

    while let Some(attr) = attrs.next(reader) {
        let item = match attr {
            Ok(result) => result,
            Err(err) => {
                warn!(
                    "Could not read INDX attribute for {}: {err:?}",
                    dir.file_record_number()
                );
                continue;
            }
        };

        let value = match item.to_attribute() {
            Ok(result) => result,
            Err(err) => {
                warn!(
                    "Could not parse INDX attribute for {}: {err:?}",
                    dir.file_record_number()
                );
                continue;
            }
        };

        let name = match value.name() {
            Ok(result) => result.to_string_lossy(),
            Err(err) => {
                warn!(
                    "Could not read INDX attribute name for {}: {err:?}",
                    dir.file_record_number()
                );
                continue;
            }
        };

        if name != "$I30" {
            continue;
        }

        let attr_type = match value.ty() {
            Ok(result) => result,
            Err(err) => {
                warn!(
                    "Could not read INDX attribute type for {}: {err:?}",
                    dir.file_record_number()
                );
                continue;
            }
        };

        if attr_type != NtfsAttributeType::IndexAllocation {
            continue;
        }

        let mut attr_value = match value.value(reader) {
            Ok(result) => result,
            Err(err) => {
                warn!("Could not open $I30 IndexAllocation: {err:?}");
                continue;
            }
        };

        let bytes = match read_value_bytes(&mut attr_value, reader, value.value_length()) {
            Ok(result) => result,
            Err(err) => {
                warn!(
                    "Could not read $I30 allocation {}: {err:?}",
                    dir.file_record_number()
                );
                continue;
            }
        };

        if let Ok((_, entries)) = parse_indx_slack(&bytes) {
            recovered.extend(
                entries
                    .into_iter()
                    .map(|entry| slack_to_file_info(entry, parent, depth, drive, evidence)),
            );
        }
    }

    recovered
}

#[derive(Debug, PartialEq)]
struct IndxSlackEntry {
    filename: String,
    created: u64,
    modified: u64,
    changed: u64,
    accessed: u64,
    size: u64,
    flags: u32,
    inode: u64,
    sequence_number: u16,
    parent_mft_reference: u32,
    parent_sequence_number: u16,
    namespace: Namespace,
}

fn parse_indx_slack(data: &[u8]) -> nom::IResult<&[u8], Vec<IndxSlackEntry>> {
    let mut indx_data = data;
    let mut slack = Vec::new();

    let min_parent_size = 64;
    while indx_data.len() >= min_parent_size {
        let (_, (mft_parent_reference, record_size, allocated_size)) =
            get_mft_parent_reference(indx_data)?;

        let (indx_slack, _) = take(record_size)(indx_data)?;

        if allocated_size < record_size {
            break;
        }

        let (_, mut indx_slack_data) = take(allocated_size - record_size)(indx_slack)?;

        while !indx_slack_data.is_empty() {
            let (slack_entry, prefix) = match search_slack(indx_slack_data, mft_parent_reference) {
                Ok(result) => result,
                Err(_) => break,
            };

            let (inode, sequence_number) = child_mft_reference(prefix);
            match Filename::parse_filename(slack_entry) {
                Ok((remaining, filename)) => {
                    indx_slack_data = remaining;
                    if filename.name.is_empty() {
                        break;
                    }

                    slack.push(IndxSlackEntry {
                        filename: filename.name,
                        created: filename.created,
                        modified: filename.modified,
                        changed: filename.changed,
                        accessed: filename.accessed,
                        size: filename.size,
                        flags: filename.file_attributes_data,
                        inode,
                        sequence_number,
                        parent_mft_reference: filename.parent_mft,
                        parent_sequence_number: filename.parent_sequence,
                        namespace: filename.namespace,
                    });
                }
                Err(_) => {
                    let (remaining, _) = take(size_of::<u64>())(slack_entry)?;
                    indx_slack_data = remaining;
                }
            }
        }

        // Header is not included in the `allocated_size`
        let indx_header_size = 24;
        let (next_indx, _) = take(allocated_size + indx_header_size)(indx_data)?;
        indx_data = next_indx;
    }

    Ok((indx_data, slack))
}

/// Search slack for the directory parent MFT reference
fn search_slack<'a>(
    indx_slack_data: &'a [u8],
    mft_parent: &'a [u8],
) -> nom::IResult<&'a [u8], &'a [u8]> {
    take_until(mft_parent)(indx_slack_data)
}

/// All INDX entries have the same parent MFT reference (the parent directory). Even entries in slack space
fn get_mft_parent_reference(indx_data: &[u8]) -> nom::IResult<&[u8], (&[u8], u32, u32)> {
    let indx_header_size: u32 = 24;
    let (data, _) = take(indx_header_size)(indx_data)?;

    let (data, offset_size) = le_u32(data)?;
    let (data, record_size) = le_u32(data)?;
    let (_, allocated_size) = le_u32(data)?;

    let (indx_entry, _) = take(offset_size + indx_header_size)(indx_data)?;
    let mft_parent_offset: u8 = 16;
    let (parent_offset, _) = take(mft_parent_offset)(indx_entry)?;
    let (_, mft_parent_reference) = take(size_of::<u64>())(parent_offset)?;

    Ok((
        parent_offset,
        (mft_parent_reference, record_size, allocated_size),
    ))
}

/// Get the MFT reference
fn child_mft_reference(prefix: &[u8]) -> (u64, u16) {
    let mft_entry_start = 16;
    let mft_entry_size = 8;

    if prefix.len() <= mft_entry_start {
        return (0, 0);
    }

    let mft_entry = &prefix[prefix.len() - mft_entry_start..prefix.len() - mft_entry_size];
    match le_u64::<_, nom::error::Error<_>>(mft_entry) {
        Ok((_, value)) => split_mft_reference(value),
        Err(_) => (0, 0),
    }
}

/// NTFS file references are a 48-bit record number plus a 16-bit sequence
fn split_mft_reference(value: u64) -> (u64, u16) {
    (value & 0x0000_FFFF_FFFF_FFFF, (value >> 48) as u16)
}

/// Map a carved slack entry onto the accessor filelisting record
fn slack_to_file_info(
    entry: IndxSlackEntry,
    parent_display: &str,
    depth: usize,
    drive: char,
    evidence: &str,
) -> FileNtfsInfo {
    let display_path = if parent_display.is_empty() {
        format!("{drive}:\\{}", entry.filename)
    } else if parent_display.ends_with('\\') {
        format!("{}{}", parent_display, entry.filename)
    } else {
        format!("{}\\{}", parent_display, entry.filename)
    };

    let scheme_path = format!("ntfs:{display_path}");
    let attributes = windows_attributes(entry.flags);

    let kind = if attributes.contains(&Attributes::Directory) {
        EntryKind::Directory
    } else {
        EntryKind::File
    };

    FileNtfsInfo {
        full_path: display_path,
        directory: directory_from_display(&scheme_path),
        filename: entry.filename.clone(),
        extension: extension_from_filename(&entry.filename),
        created: filetime_to_iso(entry.created),
        modified: filetime_to_iso(entry.modified),
        changed: filetime_to_iso(entry.changed),
        accessed: filetime_to_iso(entry.accessed),
        attributes,
        size: entry.size,
        kind,
        depth,
        display_path: scheme_path,
        inode: entry.inode,
        sequence_number: entry.sequence_number,
        parent_sequence_number: entry.parent_sequence_number,
        parent_mft_reference: entry.parent_mft_reference,
        namespace: entry.namespace,
        drive: format!("{drive}:"),
        evidence: evidence.to_string(),
        is_indx: true,
        filename_created: String::from("1970-01-01T00:00:00.000Z"),
        filename_changed: String::from("1970-01-01T00:00:00.000Z"),
        filename_accessed: String::from("1970-01-01T00:00:00.000Z"),
        filename_modified: String::from("1970-01-01T00:00:00.000Z"),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        IndxSlackEntry, get_mft_parent_reference, parse_indx_slack, search_slack,
        slack_to_file_info,
    };
    use common::{files::EntryKind, windows::Namespace};
    use std::{fs, path::PathBuf};

    #[test]
    fn test_search_slack() {
        let test_data = [1, 0, 11, 11, 100];
        let search_data = [11, 11];
        let (search_hit, nomed_data) = search_slack(&test_data, &search_data).unwrap();

        assert_eq!(search_hit, [11, 11, 100]);
        assert_eq!(nomed_data, [1, 0]);
    }

    #[test]
    fn test_get_mft_parent_reference() {
        let test_data = [
            73, 78, 68, 88, 40, 0, 9, 0, 44, 100, 121, 80, 19, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 40,
            0, 0, 0, 56, 0, 0, 0, 232, 15, 0, 0, 0, 0, 0, 0, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 0, 0, 0, 2, 0, 0, 0, 53,
            0, 0, 0, 0, 0, 22, 0, 2, 132, 24, 213, 184, 247, 215, 1, 2, 132, 24, 213, 184, 247,
            215, 1, 178, 79, 93, 246, 12, 232, 216, 1, 2, 132, 24, 213, 184, 247, 215, 1, 0, 0, 96,
            0, 0, 0, 0, 0, 0, 0, 96, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 24, 1, 48, 0, 48, 0,
            53, 0, 69, 0, 48, 0, 48, 0, 48, 0, 48, 0, 48, 0, 48, 0, 48, 0, 48, 0, 49, 0, 68, 0, 51,
            0, 68, 0, 55, 0, 67, 0, 53, 0, 57, 0, 53, 0, 57, 0, 66, 0, 48, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 16, 0, 0, 0, 2, 0, 0, 0, 53, 0, 0, 0, 0, 0, 22, 0, 2, 132, 24,
            213, 184, 247, 215, 1, 2, 132, 24, 213, 184, 247, 215, 1, 178, 79, 93, 246, 12, 232,
            216, 1, 2, 132, 24, 213, 184, 247, 215, 1, 0, 0, 96, 0, 0, 0, 0, 0, 0, 0, 96, 0,
        ];

        let (_, (result, record_size, size)) = get_mft_parent_reference(&test_data).unwrap();

        assert_eq!(result, [53, 0, 0, 0, 0, 0, 22, 0]);
        assert_eq!(size, 4072);
        assert_eq!(record_size, 56);
    }

    #[test]
    fn test_parse_indx_slack() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/ntfs/$I30");
        let buffer = fs::read(test_location).unwrap();
        let (_, result) = parse_indx_slack(&buffer).unwrap();
        assert_eq!(result.len(), 1);

        assert_eq!(
            result[0],
            IndxSlackEntry {
                filename: String::from("test.aut"),
                created: 133124426269050951,
                modified: 133124426362088370,
                changed: 133124426362088370,
                accessed: 133124426362088370,
                size: 699,
                flags: 0x20,
                inode: 8589934608,
                sequence_number: 0,
                parent_mft_reference: 863613,
                parent_sequence_number: 18,
                namespace: Namespace::WindowsDos,
            }
        );
    }

    #[test]
    fn test_slack_to_file_info() {
        let info = slack_to_file_info(
            IndxSlackEntry {
                filename: String::from("test.aut"),
                created: 133124426269050951,
                modified: 133124426362088370,
                changed: 133124426362088370,
                accessed: 133124426362088370,
                size: 699,
                flags: 0x20,
                inode: 8589934608,
                sequence_number: 0,
                parent_mft_reference: 863613,
                parent_sequence_number: 18,
                namespace: Namespace::WindowsDos,
            },
            "C:\\tmp",
            1,
            'C',
            "ntfs:C:",
        );

        assert_eq!(info.full_path, "C:\\tmp\\test.aut");
        assert_eq!(info.directory, "C:\\tmp");
        assert_eq!(info.filename, "test.aut");
        assert_eq!(info.extension, "aut");

        assert_eq!(info.filename_created, "2022-11-09T04:43:46.905Z");
        assert_eq!(info.filename_modified, "2022-11-09T04:43:56.208Z");
        assert_eq!(info.created, "");
        assert_eq!(info.kind, EntryKind::File);

        assert_eq!(info.inode, 8589934608);
        assert_eq!(info.parent_mft_reference, 863613);
        assert_eq!(info.parent_sequence_number, 18);
        assert_eq!(info.drive, "C:");
        assert_eq!(info.display_path, "ntfs:C:\\tmp\\test.aut");
        assert!(info.is_indx);
    }
}
