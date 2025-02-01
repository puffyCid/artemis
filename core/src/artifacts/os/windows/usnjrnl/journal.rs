use crate::{
    artifacts::os::windows::mft::master::{lookup_parent, Lookups},
    filesystem::{files::file_reader, ntfs::attributes::file_attribute_flags},
    utils::{
        nom_helper::{
            nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_two_bytes, Endian,
        },
        strings::extract_utf16_string,
        time::{filetime_to_unixepoch, unixepoch_to_iso},
    },
};
use common::windows::{AttributeFlags, Reason, Source};
use log::error;
use nom::bytes::complete::{take, take_until, take_while};
use ntfs::NtfsFile;
use std::{
    collections::{HashMap, HashSet},
    io::BufReader,
};

#[derive(Debug, Clone)]
pub(crate) struct UsnJrnlFormat {
    _major_version: u16,
    _minor_version: u16,
    pub(crate) mft_entry: u32,
    pub(crate) mft_sequence: u16,
    pub(crate) parent_mft_entry: u32,
    pub(crate) parent_mft_sequence: u16,
    pub(crate) update_time: String,
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

impl UsnJrnlFormat {
    /// Parse the `UsnJrnl` format and grab all the entries
    pub(crate) fn parse_usnjrnl<'a, T: std::io::Seek + std::io::Read>(
        data: &'a [u8],
        reader: &mut BufReader<T>,
        ntfs_file: Option<&NtfsFile<'_>>,
        journal_cache: &mut HashMap<String, UsnJrnlFormat>,
    ) -> nom::IResult<&'a [u8], Vec<UsnJrnlFormat>> {
        let mut remaining_input = data;

        let mut entries = Vec::new();
        let mut cache = HashMap::new();

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
            let (input, mft_entry) = nom_unsigned_four_bytes(input, Endian::Le)?;
            let (input, _padding) = nom_unsigned_two_bytes(input, Endian::Le)?;
            let (input, mft_seq) = nom_unsigned_two_bytes(input, Endian::Le)?;

            let (input, parent_mft) = nom_unsigned_four_bytes(input, Endian::Le)?;
            let (input, _padding) = nom_unsigned_two_bytes(input, Endian::Le)?;
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

            let update_time = unixepoch_to_iso(&filetime_to_unixepoch(&usn_time));
            let update_reason = UsnJrnlFormat::reason_flags(&reason);
            let update_source_flags = UsnJrnlFormat::source_flag(&source);

            let file_attributes = file_attribute_flags(&flags);

            let path = if let Some(cache_hit) = cache.get(&format!("{parent_mft}_{parent_mft_seq}"))
            {
                cache_hit
            } else {
                let mut tracker = Lookups {
                    parent_index: parent_mft,
                    parent_sequence: parent_mft_seq,
                    size: 0,
                    tracker: HashSet::new(),
                };
                &lookup_parent(reader, ntfs_file, &mut cache, &HashMap::new(), &mut tracker)
                    .unwrap_or_default()
            };
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
                name: name.clone(),
                mft_entry,
                mft_sequence: mft_seq,
                parent_mft_entry: parent_mft,
                parent_mft_sequence: parent_mft_seq,
                update_sequence_number,
                full_path: format!("{path}\\{name}"),
            };

            if entry.file_attributes.contains(&AttributeFlags::Directory) {
                journal_cache.insert(format!("{}_{}", mft_entry, mft_seq), entry.clone());
            }
            entries.push(entry);

            remaining_input = input;
        }

        Ok((remaining_input, entries))
    }

    /// Parse the `UsnJrnl` but do not lookup any parent info
    pub(crate) fn parse_usnjrnl_no_parent<'a>(
        data: &'a [u8],
        mft_path: &Option<String>,
        journal_cache: &mut HashMap<String, UsnJrnlFormat>,
    ) -> nom::IResult<&'a [u8], Vec<UsnJrnlFormat>> {
        let mut remaining_input = data;

        let mut reader = if let Some(path) = mft_path {
            match file_reader(path) {
                Ok(result) => Some(BufReader::new(result)),
                Err(err) => {
                    error!("[usnjrnl] Could create reader for alt MFT file: {err:?}");
                    None
                }
            }
        } else {
            None
        };

        let mut cache: HashMap<String, String> = HashMap::new();

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

            let (input, mft_entry) = nom_unsigned_four_bytes(input, Endian::Le)?;
            let (input, _padding) = nom_unsigned_two_bytes(input, Endian::Le)?;
            let (input, mft_seq) = nom_unsigned_two_bytes(input, Endian::Le)?;

            let (input, parent_mft) = nom_unsigned_four_bytes(input, Endian::Le)?;
            let (input, _padding) = nom_unsigned_two_bytes(input, Endian::Le)?;
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

            let update_time = unixepoch_to_iso(&filetime_to_unixepoch(&usn_time));
            let update_reason = UsnJrnlFormat::reason_flags(&reason);
            let update_source_flags = UsnJrnlFormat::source_flag(&source);

            let file_attributes = file_attribute_flags(&flags);

            let mut path = String::new();
            if reader.is_some() {
                path = if let Some(cache_hit) = cache.get(&format!("{parent_mft}_{parent_mft_seq}"))
                {
                    cache_hit.to_string()
                } else {
                    let mut tracker = Lookups {
                        parent_index: parent_mft,
                        parent_sequence: parent_mft_seq,
                        size: 0,
                        tracker: HashSet::new(),
                    };
                    // unwrap is safe because we check to for some value above
                    let lookup = reader.as_mut().unwrap();
                    lookup_parent(lookup, None, &mut cache, &HashMap::new(), &mut tracker)
                        .unwrap_or_default()
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
                name: name.clone(),
                mft_entry,
                mft_sequence: mft_seq,
                parent_mft_entry: parent_mft,
                parent_mft_sequence: parent_mft_seq,
                update_sequence_number,
                full_path: format!("{path}\\{name}"),
            };

            if entry.file_attributes.contains(&AttributeFlags::Directory) {
                journal_cache.insert(format!("{}_{}", mft_entry, mft_seq), entry.clone());
            }
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

    /// Get `UsnJrnl` update reason flags
    fn reason_flags(flag: &u32) -> Vec<Reason> {
        let mut reasons = Vec::new();

        if (flag & 0x1) == 0x1 {
            reasons.push(Reason::Overwrite);
        }
        if (flag & 0x2) == 0x2 {
            reasons.push(Reason::Extend);
        }
        if (flag & 0x4) == 0x4 {
            reasons.push(Reason::Truncation);
        }
        if (flag & 0x10) == 0x10 {
            reasons.push(Reason::NamedOverwrite);
        }
        if (flag & 0x20) == 0x20 {
            reasons.push(Reason::NamedExtend);
        }
        if (flag & 0x40) == 0x40 {
            reasons.push(Reason::NamedTruncation);
        }
        if (flag & 0x100) == 0x100 {
            reasons.push(Reason::FileCreate);
        }
        if (flag & 0x200) == 0x200 {
            reasons.push(Reason::FileDelete);
        }
        if (flag & 0x400) == 0x400 {
            reasons.push(Reason::EAChange);
        }
        if (flag & 0x800) == 0x800 {
            reasons.push(Reason::SecurityChange);
        }
        if (flag & 0x1000) == 0x1000 {
            reasons.push(Reason::RenameOldName);
        }
        if (flag & 0x2000) == 0x2000 {
            reasons.push(Reason::RenameNewName);
        }
        if (flag & 0x4000) == 0x4000 {
            reasons.push(Reason::IndexableChange);
        }
        if (flag & 0x8000) == 0x8000 {
            reasons.push(Reason::BasicInfoChange);
        }
        if (flag & 0x10000) == 0x10000 {
            reasons.push(Reason::HardLinkChange);
        }
        if (flag & 0x20000) == 0x20000 {
            reasons.push(Reason::CompressionChange);
        }
        if (flag & 0x40000) == 0x40000 {
            reasons.push(Reason::EncryptionChange);
        }
        if (flag & 0x80000) == 0x80000 {
            reasons.push(Reason::ObjectIDChange);
        }
        if (flag & 0x100000) == 0x100000 {
            reasons.push(Reason::ReparsePointChange);
        }
        if (flag & 0x200000) == 0x200000 {
            reasons.push(Reason::StreamChange);
        }
        if (flag & 0x400000) == 0x400000 {
            reasons.push(Reason::TransactedChange);
        }
        if (flag & 0x80000000) == 0x80000000 {
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
#[cfg(target_os = "windows")]
mod tests {
    use super::UsnJrnlFormat;
    use crate::artifacts::os::windows::mft::reader::setup_mft_reader_windows;
    use crate::artifacts::os::windows::usnjrnl::journal::Reason::{Close, Extend};
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
        let ntfs_file = setup_mft_reader_windows(&parser.ntfs, &mut parser.fs, "C:\\$MFT").unwrap();
        let (_, results) = UsnJrnlFormat::parse_usnjrnl(
            &test_data,
            &mut parser.fs,
            Some(&ntfs_file),
            &mut HashMap::new(),
        )
        .unwrap();
        assert_eq!(results[0]._major_version, 2);
        assert_eq!(results[0]._minor_version, 0);
        assert_eq!(results[0].mft_entry, 350259);
        assert_eq!(results[0].mft_sequence, 13);
        assert_eq!(results[0].parent_mft_entry, 350163);
        assert_eq!(results[0].parent_mft_sequence, 13);
        assert_eq!(results[0].update_time, "2023-01-30T00:39:59.000Z");
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
        let (_, results) =
            UsnJrnlFormat::parse_usnjrnl_no_parent(&test_data, &Option::None, &mut HashMap::new())
                .unwrap();
        assert_eq!(results[0]._major_version, 2);
        assert_eq!(results[0]._minor_version, 0);
        assert_eq!(results[0].mft_entry, 350259);
        assert_eq!(results[0].mft_sequence, 13);
        assert_eq!(results[0].parent_mft_entry, 350163);
        assert_eq!(results[0].parent_mft_sequence, 13);
        assert_eq!(results[0].update_time, "2023-01-30T00:39:59.000Z");
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
    fn test_nom_padding() {
        let test = [0, 1, 2, 0, 0, 0];
        let (result, _) = UsnJrnlFormat::nom_padding(&test).unwrap();
        assert_eq![result[0], 2];
    }

    #[test]
    fn test_reason_flags() {
        let test = [
            0x1, 0x2, 0x4, 0x10, 0x20, 0x40, 0x100, 0x200, 0x400, 0x800, 0x1000, 0x2000, 0x4000,
            0x8000, 0x10000, 0x20000, 0x80000, 0x100000, 0x200000, 0x400000, 0x80000000,
        ];
        for entry in test {
            let reason = UsnJrnlFormat::reason_flags(&entry);
            assert!(!reason.is_empty());
        }
    }

    #[test]
    fn test_source_flag() {
        let test = 1;
        let result = UsnJrnlFormat::source_flag(&test);
        assert_eq!(result, DataManagement);
    }
}
