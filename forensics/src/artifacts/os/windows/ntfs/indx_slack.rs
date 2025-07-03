use super::attributes::{get_attribute_name, get_attribute_type};
use crate::{
    filesystem::ntfs::sector_reader::SectorReader,
    utils::{
        nom_helper::{Endian, nom_unsigned_eight_bytes, nom_unsigned_four_bytes},
        strings::extract_utf16_string,
        time::{filetime_to_unixepoch, unixepoch_to_iso},
    },
};
use common::windows::{CompressionType, RawFilelist};
use log::{error, info};
use nom::{
    bytes::complete::{take, take_until},
    number::complete::le_u64,
};
use ntfs::{NtfsAttributes, NtfsFile, NtfsReadSeek, structured_values::NtfsFileAttributeFlags};
use std::{ffi::OsStr, fs::File, io::BufReader, mem::size_of, path::Path};

/// Find the INDX attribute for the directory entry. We search the slack space on INDX attribute for metadata on deleted files or directories
pub(crate) fn get_indx(
    fs: &mut BufReader<SectorReader<File>>,
    ntfs_file: &NtfsFile<'_>,
    directory: &str,
    depth: usize,
) -> Vec<RawFilelist> {
    let mut attributes = ntfs_file.attributes();
    let mut attributes_allocation = attributes.clone();

    let mut file_info: Vec<RawFilelist> = Vec::new();
    // Loop through all attributes until we get $I30 name and IndexRoot type
    while let Some(attribute) = attributes.next(fs) {
        let attr_result = attribute;
        let attr = match attr_result {
            Ok(result) => result,
            Err(err) => {
                error!("[ntfs] Failed to get INDX attribute item: {err:?}");
                continue;
            }
        };

        let attr_data_result = attr.to_attribute();
        let attr_data = match attr_data_result {
            Ok(result) => result,
            Err(err) => {
                error!("[forensics] Failed to get NTFS attribute error: {err:?}");
                continue;
            }
        };
        let attr_name = get_attribute_name(&attr_data);
        if attr_name != "$I30" {
            continue;
        }

        let attr_type = get_attribute_type(&attr_data);
        // There is only one Index Root
        if attr_type == "IndexRoot" {
            file_info.append(&mut get_slack(
                fs,
                &mut attributes_allocation,
                directory,
                depth,
            ));
        }
    }
    file_info
}

/// Get the raw slack space data
fn get_slack(
    fs: &mut BufReader<SectorReader<File>>,
    attributes: &mut NtfsAttributes<'_, '_>,
    directory: &str,
    depth: usize,
) -> Vec<RawFilelist> {
    let mut slack_entries: Vec<RawFilelist> = Vec::new();

    // Loop through Attributes and get the INDX allocation
    while let Some(attribute) = attributes.next(fs) {
        let attr_result = attribute;
        let attr = match attr_result {
            Ok(result) => result,
            Err(err) => {
                error!("[ntfs] Failed to get INDX attribute item: {err:?}");
                continue;
            }
        };

        let attr_data_result = attr.to_attribute();
        let attr_data = match attr_data_result {
            Ok(result) => result,
            Err(err) => {
                error!("[forensics] Failed to get NTFS attribute error: {err:?}");
                continue;
            }
        };
        let attr_name = get_attribute_name(&attr_data);
        let attr_type = get_attribute_type(&attr_data);

        if attr_name != "$I30" || attr_type != "IndexAllocation" {
            continue;
        }

        let data_result = attr_data.value(fs);
        let mut data_attr_value = match data_result {
            Ok(result) => result,
            Err(err) => {
                error!("[ntfs] Failed to get NTFS attribute data error: {err:?}");
                continue;
            }
        };
        let temp_buff_size = 65536;
        let mut temp_buff: Vec<u8> = vec![0u8; temp_buff_size];

        // Read and get the raw INDX data
        loop {
            let bytes_result = data_attr_value.read(fs, &mut temp_buff);
            let bytes = match bytes_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[ntfs] Failed to read INDX slack: {err:?}");
                    return slack_entries;
                }
            };
            if bytes == 0 {
                break;
            }

            // Make sure our temp buff does not any have extra zeros from the intialization
            if bytes < temp_buff_size {
                temp_buff = temp_buff[0..bytes].to_vec();
            }
        }

        let slack_results = parse_indx_slack(&temp_buff, directory, depth);
        match slack_results {
            Ok((_, mut result)) => slack_entries.append(&mut result),
            Err(err) => {
                info!("[ntfs] No INDX slack entries: {err:?}");
                continue;
            }
        }

        break;
    }
    slack_entries
}

/// Parse and get FILENAME attributes from INDX records in slack space
fn parse_indx_slack<'a>(
    data: &'a [u8],
    directory: &'a str,
    depth: usize,
) -> nom::IResult<&'a [u8], Vec<RawFilelist>> {
    let mut indx_data = data;
    let mut slack_entries: Vec<RawFilelist> = Vec::new();

    let min_parent_size = 64;
    while !indx_data.is_empty() && indx_data.len() > min_parent_size {
        let (_, (mft_parent_reference, record_size, allocated_size)) =
            get_mft_parent_reference(indx_data)?;
        // Go to start of INDX slack
        let (indx_slack_data, _indx_data) = take(record_size)(indx_data)?;

        // Now nom the rest of the allocated slack space
        // slack_space = total size of data (allocated_size) - all INDX records (record_size)
        let (_, mut indx_slack_data) = take(allocated_size - record_size)(indx_slack_data)?;

        while !indx_slack_data.is_empty() {
            // Nom all data until we encounter the parent MFT reference
            let slack_found = search_slack(indx_slack_data, mft_parent_reference);
            let (slack_entry, data) = match slack_found {
                Ok(result) => result,
                Err(_err) => {
                    break;
                }
            };
            // Get the MFT entry for the INDX record in slack space
            let mft_entry_start = 16;
            let mft_entry_size = 8;

            let inode = if data.len() > mft_entry_start {
                let mft_entry = &data[data.len() - mft_entry_start..data.len() - mft_entry_size];
                let (_, result) = le_u64(mft_entry)?;
                result
            } else {
                // If the INDX allocated size brings us directly to the MFT parent reference, the inode is zero (0)
                0
            };

            let slack_data = slack_entry;

            let (slack_data, _mft_reference_parent) = take(size_of::<u64>())(slack_data)?;
            let (slack_data, created) = nom_unsigned_eight_bytes(slack_data, Endian::Le)?;
            let (slack_data, modified) = nom_unsigned_eight_bytes(slack_data, Endian::Le)?;
            let (slack_data, changed) = nom_unsigned_eight_bytes(slack_data, Endian::Le)?;
            let (slack_data, accessed) = nom_unsigned_eight_bytes(slack_data, Endian::Le)?;

            let (slack_data, _allocated_size) = take(size_of::<u64>())(slack_data)?;
            let (slack_data, size) = nom_unsigned_eight_bytes(slack_data, Endian::Le)?;
            let (slack_data, flags) = nom_unsigned_four_bytes(slack_data, Endian::Le)?;
            let (slack_data, _extended_flags) = take(size_of::<u32>())(slack_data)?;

            let attr_flags = NtfsFileAttributeFlags::from_bits_truncate(flags);
            let (slack_data, filename_len) = take(size_of::<u8>())(slack_data)?;
            let empty = 0;
            if filename_len[0] == empty {
                break;
            }
            let (slack_data, _namespace) = take(size_of::<u8>())(slack_data)?;
            let utf16_adjuster = 2;
            let (slack_data, filename) = take(filename_len[0] * utf16_adjuster)(slack_data)?;

            indx_slack_data = slack_data;

            let filename = extract_utf16_string(filename);
            let (_, parent_mft_reference) = le_u64(mft_parent_reference)?;
            let attributes: Vec<String> = attr_flags
                .iter_names()
                .map(|(s, _)| s.to_string())
                .collect();

            let mut slack_file = RawFilelist {
                full_path: format!("{directory}\\{filename}"),
                directory: directory.to_string(),
                filename,
                extension: String::new(),
                created: String::new(),
                modified: String::new(),
                changed: String::new(),
                accessed: String::new(),
                filename_created: unixepoch_to_iso(filetime_to_unixepoch(created)),
                filename_modified: unixepoch_to_iso(filetime_to_unixepoch(modified)),
                filename_changed: unixepoch_to_iso(filetime_to_unixepoch(changed)),
                filename_accessed: unixepoch_to_iso(filetime_to_unixepoch(accessed)),
                size,
                inode,
                sequence_number: 0,
                parent_mft_reference,
                owner: 0,
                attributes,
                md5: String::new(),
                sha1: String::new(),
                sha256: String::new(),
                is_file: false,
                is_directory: false,
                is_indx: true,
                depth: depth.to_owned(),
                usn: 0,
                sid: 0,
                user_sid: String::new(),
                group_sid: String::new(),
                drive: directory[0..2].to_string(),
                compressed_size: 0,
                compression_type: CompressionType::None,
                ads_info: Vec::new(),
                pe_info: Vec::new(),
            };
            let extension = Path::new(&slack_file.filename)
                .extension()
                .unwrap_or_else(|| OsStr::new(""));

            slack_file.extension = extension.to_str().unwrap_or("").to_string();

            slack_entries.push(slack_file);
        }

        let indx_header_size = 24;
        // No more slack entries, move on to the next INDX. We need to add the INDX header size because it is not included in the allocated size
        let (next_indx, _) = take(allocated_size + indx_header_size)(indx_data)?;
        indx_data = next_indx;
    }
    Ok((indx_data, slack_entries))
}

/// Nom (search) slack space for the parent MFT reference, return start of INDX entry in slack
fn search_slack<'a>(
    indx_slack_data: &'a [u8],
    mft_parent: &'a [u8],
) -> nom::IResult<&'a [u8], &'a [u8]> {
    take_until(mft_parent)(indx_slack_data)
}

/// All INDX entries have the same parent MFT Rereference (the parent directory). Even entries in slack space
fn get_mft_parent_reference(indx_data: &[u8]) -> nom::IResult<&[u8], (&[u8], u32, u32)> {
    // Nom past header to get entry offset value
    let indx_header_size: u32 = 24;
    let (data, _) = take(indx_header_size)(indx_data)?;

    // Grab offset of first INDX entry
    let (data, offset_size) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (data, record_size) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (_, allocated_size) = nom_unsigned_four_bytes(data, Endian::Le)?;

    let (indx_entry, _) = take(offset_size + indx_header_size)(indx_data)?;

    let mft_parent_offset: usize = 16; // Parent reference is at at offset 16 of INDX entry
    let (parent_offset, _) = take(mft_parent_offset)(indx_entry)?;
    let (_, mft_parent_reference) = take(size_of::<u64>())(parent_offset)?;

    Ok((
        parent_offset,
        (mft_parent_reference, record_size, allocated_size),
    ))
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use crate::{
        artifacts::os::windows::ntfs::{
            attributes::{get_attribute_name, get_attribute_type},
            indx_slack::{
                get_indx, get_mft_parent_reference, get_slack, parse_indx_slack, search_slack,
            },
        },
        filesystem::ntfs::sector_reader::SectorReader,
    };
    use common::windows::RawFilelist;
    use ntfs::Ntfs;
    use std::{
        fs::{self, File},
        io::BufReader,
        path::PathBuf,
    };

    #[test]
    fn test_search_slack() {
        let test_data = [1, 0, 11, 11, 100];
        let search_data = [11, 11];

        let (search_hit, nomed_data) = search_slack(&test_data, &search_data).unwrap();

        assert_eq!(search_hit, [11, 11, 100]);
        assert_eq!(nomed_data, [1, 0])
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

        let directory = "test";
        let depth = 1;

        let (_, result) = parse_indx_slack(&buffer, &directory, depth).unwrap();
        assert_eq!(result.len(), 1);

        assert_eq!(result[0].full_path, "test\\test.aut");
        assert_eq!(result[0].directory, "test");
        assert_eq!(result[0].filename, "test.aut");
        assert_eq!(result[0].extension, "aut");

        assert_eq!(result[0].created, "");
        assert_eq!(result[0].accessed, "");
        assert_eq!(result[0].changed, "");
        assert_eq!(result[0].modified, "");

        assert_eq!(result[0].filename_created, "2022-11-09T04:43:46.000Z");
        assert_eq!(result[0].filename_modified, "2022-11-09T04:43:56.000Z");
        assert_eq!(result[0].filename_accessed, "2022-11-09T04:43:56.000Z");
        assert_eq!(result[0].filename_changed, "2022-11-09T04:43:56.000Z");

        assert_eq!(result[0].size, 699);
        assert_eq!(result[0].inode, 8589934608);
        assert_eq!(result[0].parent_mft_reference, 5066549581655421);
        assert_eq!(result[0].owner, 0);

        assert_eq!(result[0].attributes, ["ARCHIVE"]);
        assert_eq!(result[0].md5, "");
        assert_eq!(result[0].sha1, "");
        assert_eq!(result[0].sha256, "");
        assert_eq!(result[0].is_file, false);
        assert_eq!(result[0].is_directory, false);
        assert_eq!(result[0].is_indx, true);

        assert_eq!(result[0].depth, 1);
        assert_eq!(result[0].usn, 0);
        assert_eq!(result[0].sid, 0);
        assert_eq!(result[0].drive, "te");
    }

    #[test]
    fn test_get_indx() {
        let drive_path = format!("\\\\.\\{}:", 'C');
        let fs = File::open(drive_path).unwrap();

        let reader_sector_size = 4096;
        let sector_reader = SectorReader::new(fs, reader_sector_size).unwrap();
        let mut fs = BufReader::new(sector_reader);
        let ntfs = Ntfs::new(&mut fs).unwrap();
        let root_dir = ntfs.root_directory(&mut fs).unwrap();

        let result = get_indx(&mut fs, &root_dir, "test", 1);
        assert!(result.len() > 0);
    }

    #[test]
    fn test_get_slack() {
        let drive_path = format!("\\\\.\\{}:", 'C');
        let fs = File::open(drive_path).unwrap();

        let reader_sector_size = 4096;
        let sector_reader = SectorReader::new(fs, reader_sector_size).unwrap();
        let mut fs = BufReader::new(sector_reader);
        let ntfs = Ntfs::new(&mut fs).unwrap();
        let root_dir = ntfs.root_directory(&mut fs).unwrap();

        let mut attributes = root_dir.attributes();
        let mut attributes_allocation = attributes.clone();

        let mut file_info: Vec<RawFilelist> = Vec::new();
        // Loop through all attributes until we get $I30 name and IndexRoot type
        while let Some(attribute) = attributes.next(&mut fs) {
            let attr_result = attribute;
            let attr = match attr_result {
                Ok(result) => result,
                Err(_err) => {
                    continue;
                }
            };

            let attr_data = attr.to_attribute().unwrap();
            let attr_name = get_attribute_name(&attr_data);

            if attr_name != "$I30" {
                continue;
            }
            let attr_type = get_attribute_type(&attr_data);

            // There is only one Index Root
            if attr_type == "IndexRoot" {
                file_info.append(&mut get_slack(
                    &mut fs,
                    &mut attributes_allocation,
                    "test",
                    1,
                ));
            }
        }
        assert!(file_info.len() > 0);
    }
}
