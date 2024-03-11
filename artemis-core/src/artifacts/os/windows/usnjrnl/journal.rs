use crate::{
    filesystem::ntfs::{attributes::file_attribute_flags, sector_reader::SectorReader},
    utils::{
        nom_helper::{
            nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_two_bytes, Endian,
        },
        strings::extract_utf16_string,
        time::filetime_to_unixepoch,
    },
};
use byteorder::{LittleEndian, ReadBytesExt};
use common::windows::{AttributeFlags, Reason, Source};
use log::{error, warn};
use nom::bytes::complete::{take, take_until, take_while};
use ntfs::{structured_values::NtfsFileNamespace, Ntfs, NtfsError};
use std::{collections::HashMap, fs::File, io::BufReader};

#[derive(Debug)]
pub(crate) struct UsnJrnlFormat {
    _major_version: u16,
    _minor_version: u16,
    pub(crate) mft_entry: u64,
    pub(crate) mft_sequence: u16,
    pub(crate) parent_mft_entry: u64,
    pub(crate) parent_mft_sequence: u16,
    pub(crate) update_time: i64,
    pub(crate) update_reason: Vec<Reason>,
    pub(crate) update_source_flags: Source,
    pub(crate) security_descriptor_id: u32,
    pub(crate) update_sequence_number: u64,
    pub(crate) file_attributes: Vec<AttributeFlags>,
    _name_size: u16,
    _name_offset: u16,
    pub(crate) name: String,
    pub(crate) full_path: String,
}

#[derive(Clone)]
struct Parent {
    name: String,
    parent_id: u64,
    parent_sequence: u16,
    sequence_number: u16,
}

impl UsnJrnlFormat {
    /// Parse the `UsnJrnl` format and grab all the entries
    pub(crate) fn parse_usnjrnl<'a>(
        data: &'a [u8],
        ntfs: &Ntfs,
        fs: &mut BufReader<SectorReader<File>>,
    ) -> nom::IResult<&'a [u8], Vec<UsnJrnlFormat>> {
        let mut remaining_input = data;

        let mut entries = Vec::new();
        let mut cache_ids: HashMap<u64, Parent> = HashMap::new();
        while !remaining_input.is_empty() {
            // Nom any padding data, if we nom'd everything then we are done
            let result = UsnJrnlFormat::nom_padding(remaining_input);
            let input = match result {
                Ok((usnjrnl_data, _)) => usnjrnl_data,
                Err(_) => break,
            };
            if input.is_empty() {
                break;
            }

            let (input, _major_version) = nom_unsigned_two_bytes(input, Endian::Le)?;
            let (input, _minor_version) = nom_unsigned_two_bytes(input, Endian::Le)?;

            let entry_size: u8 = 6;
            let (input, mut entry_data) = take(entry_size)(input)?;
            let (input, mft_seq) = nom_unsigned_two_bytes(input, Endian::Le)?;

            let (input, mut parent_entry_data) = take(entry_size)(input)?;
            let (input, parent_mft_seq) = nom_unsigned_two_bytes(input, Endian::Le)?;

            let (input, update_sequence_number) = nom_unsigned_eight_bytes(input, Endian::Le)?;
            let (input, usn_time) = nom_unsigned_eight_bytes(input, Endian::Le)?;
            let (input, reason) = nom_unsigned_four_bytes(input, Endian::Le)?;
            let (input, source) = nom_unsigned_four_bytes(input, Endian::Le)?;
            let (input, security_descriptor_id) = nom_unsigned_four_bytes(input, Endian::Le)?;
            let (input, flags) = nom_unsigned_four_bytes(input, Endian::Le)?;
            let (input, name_size) = nom_unsigned_two_bytes(input, Endian::Le)?;
            let (input, name_offset) = nom_unsigned_two_bytes(input, Endian::Le)?;

            let offset_position = 60;
            if name_offset != offset_position {
                remaining_input = input;
                continue;
            }

            // The name always follows the name offset. So we actually do not need it
            let (input, name_data) = take(name_size)(input)?;
            let name = extract_utf16_string(name_data);

            let update_time = filetime_to_unixepoch(&usn_time);
            let update_reason = UsnJrnlFormat::reason_flags(&reason);
            let update_source_flags = UsnJrnlFormat::source_flag(&source);

            let file_attributes = file_attribute_flags(&flags);
            let mut parents: Vec<String> = Vec::new();
            let parent_entry = parent_entry_data.read_u48::<LittleEndian>().unwrap_or(5);

            // Check our cache and if found make sure the sequence numbers match
            if let Some(cache) = cache_ids.clone().get(&parent_entry) {
                if parent_mft_seq != cache.sequence_number {
                    parents.push(String::from("Could not find parent"));
                } else {
                    let path_result = UsnJrnlFormat::iterate_parents(
                        parent_entry,
                        parent_mft_seq,
                        ntfs,
                        &mut parents,
                        fs,
                        &mut cache_ids,
                    );
                    match path_result {
                        Ok(_) => {}
                        Err(err) => {
                            error!("[usnjrnl] Could not determine parent directories: {err:?}");
                        }
                    };
                }
            } else {
                let path_result = UsnJrnlFormat::iterate_parents(
                    parent_entry,
                    parent_mft_seq,
                    ntfs,
                    &mut parents,
                    fs,
                    &mut cache_ids,
                );
                match path_result {
                    Ok(_) => {}
                    Err(err) => {
                        error!("[usnjrnl] Could not determine parent directories: {err:?}");
                    }
                };
            }

            let entry = UsnJrnlFormat {
                _major_version,
                _minor_version,
                update_time,
                update_reason,
                update_source_flags,
                security_descriptor_id,
                file_attributes,
                _name_size: name_size,
                _name_offset: name_offset,
                name,
                mft_entry: entry_data.read_u48::<LittleEndian>().unwrap_or(0),
                mft_sequence: mft_seq,
                parent_mft_entry: parent_entry,
                parent_mft_sequence: parent_mft_seq,
                update_sequence_number,
                full_path: parents.join("\\"),
            };

            entries.push(entry);

            remaining_input = input;
        }

        Ok((remaining_input, entries))
    }

    /// Parse the `UsnJrnl` but do not lookup any parent info
    pub(crate) fn parse_usnjrnl_no_parent(data: &[u8]) -> nom::IResult<&[u8], Vec<UsnJrnlFormat>> {
        let mut remaining_input = data;

        let mut entries = Vec::new();
        while !remaining_input.is_empty() {
            // Nom any padding data, if we nom'd everything then we are done
            let result = UsnJrnlFormat::nom_padding(remaining_input);
            let input = match result {
                Ok((usnjrnl_data, _)) => usnjrnl_data,
                Err(_) => break,
            };
            if input.is_empty() {
                break;
            }

            let (input, _major_version) = nom_unsigned_two_bytes(input, Endian::Le)?;
            let (input, _minor_version) = nom_unsigned_two_bytes(input, Endian::Le)?;

            let entry_size: u8 = 6;
            let (input, mut entry_data) = take(entry_size)(input)?;
            let (input, mft_seq) = nom_unsigned_two_bytes(input, Endian::Le)?;

            let (input, mut parent_entry_data) = take(entry_size)(input)?;
            let (input, parent_mft_seq) = nom_unsigned_two_bytes(input, Endian::Le)?;

            let (input, update_sequence_number) = nom_unsigned_eight_bytes(input, Endian::Le)?;
            let (input, usn_time) = nom_unsigned_eight_bytes(input, Endian::Le)?;
            let (input, reason) = nom_unsigned_four_bytes(input, Endian::Le)?;
            let (input, source) = nom_unsigned_four_bytes(input, Endian::Le)?;
            let (input, security_descriptor_id) = nom_unsigned_four_bytes(input, Endian::Le)?;
            let (input, flags) = nom_unsigned_four_bytes(input, Endian::Le)?;
            let (input, name_size) = nom_unsigned_two_bytes(input, Endian::Le)?;
            let (input, name_offset) = nom_unsigned_two_bytes(input, Endian::Le)?;

            let offset_position = 60;
            if name_offset != offset_position {
                remaining_input = input;
                continue;
            }

            // The name always follows the name offset. So we actually do not need it
            let (input, name_data) = take(name_size)(input)?;
            let name = extract_utf16_string(name_data);

            let update_time = filetime_to_unixepoch(&usn_time);
            let update_reason = UsnJrnlFormat::reason_flags(&reason);
            let update_source_flags = UsnJrnlFormat::source_flag(&source);

            let file_attributes = file_attribute_flags(&flags);
            let parent_entry = parent_entry_data.read_u48::<LittleEndian>().unwrap_or(5);

            let entry = UsnJrnlFormat {
                _major_version,
                _minor_version,
                update_time,
                update_reason,
                update_source_flags,
                security_descriptor_id,
                file_attributes,
                _name_size: name_size,
                _name_offset: name_offset,
                name,
                mft_entry: entry_data.read_u48::<LittleEndian>().unwrap_or(0),
                mft_sequence: mft_seq,
                parent_mft_entry: parent_entry,
                parent_mft_sequence: parent_mft_seq,
                update_sequence_number,
                full_path: String::new(),
            };

            entries.push(entry);

            remaining_input = input;
        }

        Ok((remaining_input, entries))
    }

    /// Nom any zero (0) padding at the end of an `UsnJrnl` entry
    /// Then scan for the version details
    fn nom_padding(data: &[u8]) -> nom::IResult<&[u8], ()> {
        let padding = 0;
        let (remaining_input, _) = take_while(|b| b == padding)(data)?;
        if remaining_input.is_empty() {
            return Ok((remaining_input, ()));
        }

        /*
         * Since we nom'd all zeros there is a chance we nom'd part of the size data :`(
         * Ex: Size = 4096 (0x1000, nom'd 00)
         * So now we jump to the version details: Major version 2, minor version: 0
         * We only support version 2
         */
        let version_details = [2, 0, 0, 0];
        let (remaining_input, _) = take_until(version_details.as_slice())(remaining_input)?;
        Ok((remaining_input, ()))
    }

    /// Recursively search for parents by looking up the parent MFT entry ID
    fn iterate_parents(
        entry: u64,
        parent_sequence: u16,
        ntfs: &Ntfs,
        parents: &mut Vec<String>,
        fs: &mut BufReader<SectorReader<File>>,
        cache_ids: &mut HashMap<u64, Parent>,
    ) -> Result<(), NtfsError> {
        let root = 5;
        if entry == root {
            return Ok(());
        }

        // We should not encounter an infinite recursive loop. But just in case we have a hard limit of parent directories
        let max_parents = 45;
        if parents.len() > max_parents {
            warn!("[usnjrnl] Reached {max_parents} parents. We might have encountered an infinite loop (parent-child point to each other as their respective parents)");
            return Ok(());
        }

        if let Some(cache) = cache_ids.clone().get(&entry) {
            let result = UsnJrnlFormat::iterate_parents(
                cache.parent_id,
                cache.parent_sequence,
                ntfs,
                parents,
                fs,
                cache_ids,
            );
            match result {
                Ok(_) => parents.push(cache.name.clone()),
                Err(_) => {
                    // We do not have a parent, we can start building the full directory now
                    parents.push(String::from("Could not find parent"));
                }
            }
            return Ok(());
        }

        let parent = ntfs.file(fs, entry)?;
        if let Some(fileinfo) = parent.name(fs, Some(NtfsFileNamespace::Win32AndDos), None) {
            let info = fileinfo?;
            let parent_ref = info.parent_directory_reference().file_record_number();
            let parent_seq = info.parent_directory_reference().sequence_number();

            if parent_sequence != parent.sequence_number() {
                // If the sequence numbers do no match then that means the MFT record has been deleted and reused. Cannot recreate paths
                parents.push(String::from("Could not find parent"));
                cache_ids.insert(
                    entry,
                    Parent {
                        name: info.name().to_string_lossy(),
                        parent_id: parent_ref,
                        parent_sequence: parent_seq,
                        sequence_number: parent.sequence_number(),
                    },
                );
                return Ok(());
            }

            let result = UsnJrnlFormat::iterate_parents(
                parent_ref, parent_seq, ntfs, parents, fs, cache_ids,
            );
            match result {
                Ok(_) => {
                    parents.push(info.name().to_string_lossy());
                    cache_ids.insert(
                        entry,
                        Parent {
                            name: info.name().to_string_lossy(),
                            parent_id: parent_ref,
                            parent_sequence: parent_seq,
                            sequence_number: parent.sequence_number(),
                        },
                    );
                }
                Err(_) => {
                    // We do not have a parent, we can start building the full directory now
                    parents.push(String::from("Could not find parent"));
                }
            }
        } else if let Some(fileinfo) = parent.name(fs, Some(NtfsFileNamespace::Win32), None) {
            let info = fileinfo?;
            let parent_ref = info.parent_directory_reference().file_record_number();
            let parent_seq = info.parent_directory_reference().sequence_number();

            if parent_sequence != parent.sequence_number() {
                // If the sequence numbers do no match then that means the MFT record has been deleted and reused. Cannot recreate paths
                parents.push(String::from("Could not find parent"));
                cache_ids.insert(
                    entry,
                    Parent {
                        name: info.name().to_string_lossy(),
                        parent_id: parent_ref,
                        parent_sequence: parent_seq,
                        sequence_number: parent.sequence_number(),
                    },
                );
                return Ok(());
            }

            let result = UsnJrnlFormat::iterate_parents(
                parent_ref, parent_seq, ntfs, parents, fs, cache_ids,
            );
            match result {
                Ok(_) => {
                    parents.push(info.name().to_string_lossy());
                    cache_ids.insert(
                        entry,
                        Parent {
                            name: info.name().to_string_lossy(),
                            parent_id: parent_ref,
                            parent_sequence: parent_seq,
                            sequence_number: parent.sequence_number(),
                        },
                    );
                }
                Err(_) => {
                    // We do not have a parent, we can start building the full directory now
                    parents.push(String::from("Could not find parent"));
                }
            }
        } else if let Some(fileinfo) = parent.name(fs, Some(NtfsFileNamespace::Posix), None) {
            let info = fileinfo?;
            let parent_ref = info.parent_directory_reference().file_record_number();
            let parent_seq = info.parent_directory_reference().sequence_number();

            if parent_sequence != parent.sequence_number() {
                // If the sequence numbers do no match then that means the MFT record has been deleted and reused. Cannot recreate paths
                parents.push(String::from("Could not find parent"));
                cache_ids.insert(
                    entry,
                    Parent {
                        name: info.name().to_string_lossy(),
                        parent_id: parent_ref,
                        parent_sequence: parent_seq,
                        sequence_number: parent.sequence_number(),
                    },
                );
                return Ok(());
            }

            let result = UsnJrnlFormat::iterate_parents(
                parent_ref, parent_seq, ntfs, parents, fs, cache_ids,
            );
            match result {
                Ok(_) => {
                    parents.push(info.name().to_string_lossy());
                    cache_ids.insert(
                        entry,
                        Parent {
                            name: info.name().to_string_lossy(),
                            parent_id: parent_ref,
                            parent_sequence: parent_seq,
                            sequence_number: parent.sequence_number(),
                        },
                    );
                }
                Err(_) => {
                    // We do not have a parent, we can start building the full directory now
                    parents.push(String::from("Could not find parent"));
                }
            }
        }
        Ok(())
    }

    /// Get `UsnJrnl` update reason flags
    fn reason_flags(flag: &u32) -> Vec<Reason> {
        let mut reasons = Vec::new();

        let overwrite = 0x1;
        let extend = 0x2;
        let truncate = 0x4;
        let name_overwrite = 0x10;
        let name_extend = 0x20;
        let name_truncate = 0x40;
        let file_create = 0x100;
        let file_delete = 0x200;
        let ea_change = 0x400;
        let sec_change = 0x800;
        let old_name = 0x1000;
        let new_name = 0x2000;
        let indexable = 0x4000;
        let info_change = 0x8000;
        let link_change = 0x10000;
        let compression_change = 0x20000;
        let encrypt_change = 0x40000;
        let object_change = 0x80000;
        let reparse_change = 0x100000;
        let stream_change = 0x200000;
        let transacted_change = 0x400000;
        let close = 0x80000000;

        if (flag & overwrite) == overwrite {
            reasons.push(Reason::Overwrite);
        }
        if (flag & extend) == extend {
            reasons.push(Reason::Extend);
        }
        if (flag & truncate) == truncate {
            reasons.push(Reason::Truncation);
        }
        if (flag & name_overwrite) == name_overwrite {
            reasons.push(Reason::NamedOverwrite);
        }
        if (flag & name_extend) == name_extend {
            reasons.push(Reason::NamedExtend);
        }
        if (flag & name_truncate) == name_truncate {
            reasons.push(Reason::NamedTruncation);
        }
        if (flag & file_create) == file_create {
            reasons.push(Reason::FileCreate);
        }
        if (flag & file_delete) == file_delete {
            reasons.push(Reason::FileDelete);
        }
        if (flag & ea_change) == ea_change {
            reasons.push(Reason::EAChange);
        }
        if (flag & sec_change) == sec_change {
            reasons.push(Reason::SecurityChange);
        }
        if (flag & old_name) == old_name {
            reasons.push(Reason::RenameOldName);
        }
        if (flag & new_name) == new_name {
            reasons.push(Reason::RenameNewName);
        }
        if (flag & indexable) == indexable {
            reasons.push(Reason::IndexableChange);
        }
        if (flag & info_change) == info_change {
            reasons.push(Reason::BasicInfoChange);
        }
        if (flag & link_change) == link_change {
            reasons.push(Reason::HardLinkChange);
        }
        if (flag & compression_change) == compression_change {
            reasons.push(Reason::CompressionChange);
        }
        if (flag & encrypt_change) == encrypt_change {
            reasons.push(Reason::EncryptionChange);
        }
        if (flag & object_change) == object_change {
            reasons.push(Reason::ObjectIDChange);
        }
        if (flag & reparse_change) == reparse_change {
            reasons.push(Reason::ReparsePointChange);
        }
        if (flag & stream_change) == stream_change {
            reasons.push(Reason::StreamChange);
        }
        if (flag & transacted_change) == transacted_change {
            reasons.push(Reason::TransactedChange);
        }
        if (flag & close) == close {
            reasons.push(Reason::Close);
        }
        reasons
    }

    /// Get `UsnJrnl` source flags (none seen so far)
    fn source_flag(flags: &u32) -> Source {
        let data_manage = 0x1;
        let aux_data = 0x2;
        let replicated_manage = 0x4;

        if flags == &data_manage {
            Source::DataManagement
        } else if flags == &aux_data {
            Source::AuxiliaryData
        } else if flags == &replicated_manage {
            Source::ReplicationManagement
        } else {
            Source::None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::UsnJrnlFormat;
    use crate::artifacts::os::windows::usnjrnl::journal::Reason::{Close, Extend, Overwrite};
    use crate::artifacts::os::windows::usnjrnl::journal::Source::{DataManagement, None};
    use crate::filesystem::ntfs::setup::setup_ntfs_parser;
    use common::windows::AttributeFlags::Archive;
    use std::collections::HashMap;

    #[test]
    fn test_parse_usnjrnl() {
        let test_data = [
            144, 0, 0, 0, 2, 0, 0, 0, 51, 88, 5, 0, 0, 0, 13, 0, 211, 87, 5, 0, 0, 0, 13, 0, 0, 0,
            54, 96, 6, 0, 0, 0, 220, 174, 212, 97, 67, 52, 217, 1, 2, 0, 0, 128, 0, 0, 0, 0, 0, 0,
            0, 0, 32, 0, 0, 0, 84, 0, 60, 0, 98, 0, 57, 0, 55, 0, 102, 0, 56, 0, 54, 0, 48, 0, 50,
            0, 45, 0, 100, 0, 57, 0, 98, 0, 54, 0, 45, 0, 52, 0, 51, 0, 56, 0, 55, 0, 45, 0, 97, 0,
            53, 0, 99, 0, 56, 0, 45, 0, 98, 0, 99, 0, 53, 0, 99, 0, 50, 0, 55, 0, 51, 0, 102, 0,
            52, 0, 51, 0, 51, 0, 51, 0, 46, 0, 106, 0, 115, 0, 111, 0, 110, 0, 108, 0, 0, 0, 0, 0,
        ];
        let mut parser = setup_ntfs_parser(&'C').unwrap();
        let (_, results) =
            UsnJrnlFormat::parse_usnjrnl(&test_data, &parser.ntfs, &mut parser.fs).unwrap();
        assert_eq!(results[0]._major_version, 2);
        assert_eq!(results[0]._minor_version, 0);
        assert_eq!(results[0].mft_entry, 350259);
        assert_eq!(results[0].mft_sequence, 13);
        assert_eq!(results[0].parent_mft_entry, 350163);
        assert_eq!(results[0].parent_mft_sequence, 13);
        assert_eq!(results[0].update_time, 1675039199);
        assert_eq!(results[0].update_reason, vec![Extend, Close]);
        assert_eq!(results[0].update_source_flags, None);
        assert_eq!(results[0].security_descriptor_id, 0);
        assert_eq!(results[0].file_attributes, vec![Archive]);
        assert_eq!(results[0]._name_size, 84);
        assert_eq!(results[0]._name_offset, 60);
        assert_eq!(
            results[0].name,
            "b97f8602-d9b6-4387-a5c8-bc5c273f4333.jsonl"
        );
    }

    #[test]
    fn test_parse_usnjrnl_no_parent() {
        let test_data = [
            144, 0, 0, 0, 2, 0, 0, 0, 51, 88, 5, 0, 0, 0, 13, 0, 211, 87, 5, 0, 0, 0, 13, 0, 0, 0,
            54, 96, 6, 0, 0, 0, 220, 174, 212, 97, 67, 52, 217, 1, 2, 0, 0, 128, 0, 0, 0, 0, 0, 0,
            0, 0, 32, 0, 0, 0, 84, 0, 60, 0, 98, 0, 57, 0, 55, 0, 102, 0, 56, 0, 54, 0, 48, 0, 50,
            0, 45, 0, 100, 0, 57, 0, 98, 0, 54, 0, 45, 0, 52, 0, 51, 0, 56, 0, 55, 0, 45, 0, 97, 0,
            53, 0, 99, 0, 56, 0, 45, 0, 98, 0, 99, 0, 53, 0, 99, 0, 50, 0, 55, 0, 51, 0, 102, 0,
            52, 0, 51, 0, 51, 0, 51, 0, 46, 0, 106, 0, 115, 0, 111, 0, 110, 0, 108, 0, 0, 0, 0, 0,
        ];
        let (_, results) = UsnJrnlFormat::parse_usnjrnl_no_parent(&test_data).unwrap();
        assert_eq!(results[0]._major_version, 2);
        assert_eq!(results[0]._minor_version, 0);
        assert_eq!(results[0].mft_entry, 350259);
        assert_eq!(results[0].mft_sequence, 13);
        assert_eq!(results[0].parent_mft_entry, 350163);
        assert_eq!(results[0].parent_mft_sequence, 13);
        assert_eq!(results[0].update_time, 1675039199);
        assert_eq!(results[0].update_reason, vec![Extend, Close]);
        assert_eq!(results[0].update_source_flags, None);
        assert_eq!(results[0].security_descriptor_id, 0);
        assert_eq!(results[0].file_attributes, vec![Archive]);
        assert_eq!(results[0]._name_size, 84);
        assert_eq!(results[0]._name_offset, 60);
        assert_eq!(
            results[0].name,
            "b97f8602-d9b6-4387-a5c8-bc5c273f4333.jsonl"
        );
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_iterate_parents() {
        let test = 955759;
        let mut cache = HashMap::new();
        let mut parents = Vec::new();
        let mut parser = setup_ntfs_parser(&'C').unwrap();
        let results = UsnJrnlFormat::iterate_parents(
            test,
            5,
            &parser.ntfs,
            &mut parents,
            &mut parser.fs,
            &mut cache,
        )
        .unwrap();

        assert_eq!(results, ());
    }

    #[test]
    fn test_nom_padding() {
        let test = [0, 1, 2, 0, 0, 0];
        let (result, _) = UsnJrnlFormat::nom_padding(&test).unwrap();
        assert_eq![result[0], 2];
    }

    #[test]
    fn test_reason_flags() {
        let test = 1;
        let reason = UsnJrnlFormat::reason_flags(&test);
        assert_eq!(reason[0], Overwrite);
    }

    #[test]
    fn test_source_flag() {
        let test = 1;
        let result = UsnJrnlFormat::source_flag(&test);
        assert_eq!(result, DataManagement);
    }
}
