use std::{io::BufReader, num::NonZero};

use crate::{
    artifacts::os::windows::registry::{
        cell::{CellType, get_cell_type, is_allocated, walk_registry, walk_values},
        error::RegistryError,
        parser::Params,
    },
    filesystem::{files::get_filename, ntfs::reader::read_bytes},
    utils::{
        nom_helper::{
            Endian, nom_signed_four_bytes, nom_unsigned_eight_bytes, nom_unsigned_four_bytes,
            nom_unsigned_two_bytes,
        },
        regex_options::regex_check,
        strings::{extract_ascii_utf16_string, extract_utf16_string, strings_contains},
        time::{filetime_to_unixepoch, unixepoch_to_iso},
    },
};
use common::windows::RegistryData;
use log::error;
use nom::{Needed, bytes::complete::take, error::ErrorKind};
use ntfs::NtfsFile;

#[derive(Debug)]
pub(crate) struct NameKey {
    _sig: u16,
    _flags: u16,
    pub(crate) last_modified: u64,
    _accessed_bits: u32, // If Windows 8+, otherwise its Spare
    _parent: u32,
    pub(crate) number_subkeys: u32,
    _num_volatile_subkeys: u32, // Not used when parsing Registry file on disk
    pub(crate) subkeys_list_offset: i32,
    _volatile_subkeys_list_offset: i32, // Not used when parsing Registry file on disk
    pub(crate) number_key_values: u32,
    pub(crate) key_values_offset: i32,
    pub(crate) key_security_offset: i32,
    _class_name_offset: i32,
    _largest_subkey_name_length: u32,
    _largest_class_name_length: u32,
    _largest_value_name_length: u32,
    _largest_value_data_length: u32,
    _workvar: u32,
    _key_name_length: u16,
    _class_name_length: u16,
    pub(crate) key_name: String,
}

impl NameKey {
    /// Parse Registry name key and get all all subkeys and values associated with it
    pub(crate) fn parse_name_key<'a>(
        reg_data: &'a [u8],
        name_key: &'a [u8],
        params: &mut Params,
        minor_version: u32,
    ) -> nom::IResult<&'a [u8], ()> {
        let (input, sig) = nom_unsigned_two_bytes(name_key, Endian::Le)?;
        let (input, flags) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, last_modified) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, accessed_bits) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, parent) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, number_subkeys) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let (input, num_volatile_subkeys) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, subkeys_list_offset) = nom_signed_four_bytes(input, Endian::Le)?;
        let (input, volatile_subkeys_list_offset) = nom_signed_four_bytes(input, Endian::Le)?;
        let (input, number_key_values) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, key_values_offset) = nom_signed_four_bytes(input, Endian::Le)?;
        let (input, key_security_offset) = nom_signed_four_bytes(input, Endian::Le)?;
        let (input, class_name_offset) = nom_signed_four_bytes(input, Endian::Le)?;

        let (input, largest_subkey_name_length) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, largest_class_name_length) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, largest_value_name_length) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, largest_value_data_length) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let (input, workvar) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, key_name_length) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, class_name_length) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, key_name_data) = take(key_name_length)(input)?;

        // The string can either be ASCII or UTF16
        let mut key_name = extract_ascii_utf16_string(key_name_data);
        if format!("{key_name:?}").contains("\\u{") {
            key_name = extract_utf16_string(key_name_data);
        }

        let name_key = NameKey {
            _sig: sig,
            _flags: flags,
            last_modified,
            _accessed_bits: accessed_bits,
            _parent: parent,
            number_subkeys,
            _num_volatile_subkeys: num_volatile_subkeys,
            subkeys_list_offset,
            _volatile_subkeys_list_offset: volatile_subkeys_list_offset,
            number_key_values,
            key_values_offset,
            key_security_offset,
            _class_name_offset: class_name_offset,
            _largest_subkey_name_length: largest_subkey_name_length,
            _largest_class_name_length: largest_class_name_length,
            _largest_value_name_length: largest_value_name_length,
            _largest_value_data_length: largest_value_data_length,
            _workvar: workvar,
            _key_name_length: key_name_length,
            _class_name_length: class_name_length,
            key_name,
        };

        let mut registry_entry = RegistryData {
            path: params.key_tracker.join("\\"),
            key: params.key_tracker.join("\\"),
            name: name_key.key_name.clone(),
            values: Vec::new(),
            last_modified: unixepoch_to_iso(filetime_to_unixepoch(last_modified)),
            depth: params.key_tracker.len(),
            security_offset: key_security_offset,
            registry_file: get_filename(&params.registry_path),
            registry_path: params.registry_path.clone(),
        };

        params.key_tracker.push(name_key.key_name);

        //registry_entry.path = params.key_tracker.join("\\");

        // From here we iterate through subkeys and key values
        // If any of the offsets are -1 then there are no entries
        let no_lists = -1;
        if name_key.key_values_offset != no_lists {
            let result = walk_values(
                reg_data,
                name_key.key_values_offset as u32,
                name_key.number_key_values,
                minor_version,
            );
            match result {
                Ok((_, values)) => registry_entry.values = values,
                Err(_) => {
                    error!(
                        "[registry] Failed to iterate through Values at: {}",
                        params.key_tracker.join("\\")
                    );
                }
            }
        }

        // Case sensitivity does not matter for Registry keys
        if registry_entry
            .path
            .to_lowercase()
            .starts_with(&params.start_path.to_lowercase())
            && regex_check(&params.path_regex, &registry_entry.path.to_lowercase())
        {
            params.registry_list.push(registry_entry);
        }

        if name_key.subkeys_list_offset != no_lists
            && strings_contains(
                &params.start_path.to_lowercase(),
                &params.key_tracker.join("\\").to_lowercase(),
            )
        {
            let result = walk_registry(
                reg_data,
                name_key.subkeys_list_offset as u32,
                params,
                minor_version,
            );
            match result {
                Ok((_, _)) => {}
                Err(_) => {
                    error!(
                        "[registry] Failed to iterate through sublist at: {}",
                        params.key_tracker.join("\\")
                    );
                }
            }
        }

        // pop the params.key_tracker if we finished parsing a name key
        params.key_tracker.pop();
        Ok((input, ()))
    }

    pub(crate) fn read_name_key<T: std::io::Seek + std::io::Read>(
        reader: &mut BufReader<T>,
        ntfs_file: Option<&NtfsFile<'_>>,
        offset: u32,
        size: u32,
    ) -> Result<NameKey, RegistryError> {
        // Skip Registry header
        let skip_header = 1;
        // Size should almost always be 4096
        // Skip hbin header
        let skip_hbin = 32;
        let real_offst = offset + size;
        let name_bytes = match read_bytes(real_offst as u64, size as u64, ntfs_file, reader) {
            Ok(result) => result,
            Err(err) => {
                error!("[registry] Could not read name key bytes: {err:?}");
                return Err(RegistryError::ReadRegistry);
            }
        };

        let name = match NameKey::parse_name(&name_bytes) {
            Ok((_, result)) => result,
            Err(_err) => {
                error!("[registry] Could not parse name key bytes");
                return Err(RegistryError::Parser);
            }
        };

        Ok(name)
    }

    fn parse_name(data: &[u8]) -> nom::IResult<&[u8], NameKey> {
        // Get the size of the list and check if its allocated (negative numbers = allocated, postive number = unallocated)
        let (input, (allocated, size)) = is_allocated(data)?;

        // Size includes the size itself. We nommed that away
        let adjust_cell_size = 4;
        if size < adjust_cell_size {
            panic!("tiny name?");
            return Err(nom::Err::Incomplete(Needed::Size(NonZero::new(4).unwrap())));
        }

        if !allocated {
            panic!("not allocatee");
            return Err(nom::Err::Incomplete(Needed::Size(NonZero::new(1).unwrap())));
        }

        let (_, cell_data) = take(size - adjust_cell_size)(input)?;

        let (input, cell) = get_cell_type(cell_data)?;

        // Name key cells are the only cells we want, they contain all the info needed to parse the Registry
        if cell != CellType::Nk {
            panic!("[registry] Did not get name key cell type: {cell:?}");
            return Err(nom::Err::Failure(nom::error::Error::new(
                &[],
                ErrorKind::Fail,
            )));
        }
        let (input, sig) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, flags) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, last_modified) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, accessed_bits) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, parent) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, number_subkeys) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let (input, num_volatile_subkeys) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, subkeys_list_offset) = nom_signed_four_bytes(input, Endian::Le)?;
        let (input, volatile_subkeys_list_offset) = nom_signed_four_bytes(input, Endian::Le)?;
        let (input, number_key_values) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, key_values_offset) = nom_signed_four_bytes(input, Endian::Le)?;
        let (input, key_security_offset) = nom_signed_four_bytes(input, Endian::Le)?;
        let (input, class_name_offset) = nom_signed_four_bytes(input, Endian::Le)?;

        let (input, largest_subkey_name_length) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, largest_class_name_length) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, largest_value_name_length) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, largest_value_data_length) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let (input, workvar) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, key_name_length) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, class_name_length) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, key_name_data) = take(key_name_length)(input)?;

        // The string can either be ASCII or UTF16
        let mut key_name = extract_ascii_utf16_string(key_name_data);
        if format!("{key_name:?}").contains("\\u{") {
            key_name = extract_utf16_string(key_name_data);
        }

        let name_key = NameKey {
            _sig: sig,
            _flags: flags,
            last_modified,
            _accessed_bits: accessed_bits,
            _parent: parent,
            number_subkeys,
            _num_volatile_subkeys: num_volatile_subkeys,
            subkeys_list_offset,
            _volatile_subkeys_list_offset: volatile_subkeys_list_offset,
            number_key_values,
            key_values_offset,
            key_security_offset,
            _class_name_offset: class_name_offset,
            _largest_subkey_name_length: largest_subkey_name_length,
            _largest_class_name_length: largest_class_name_length,
            _largest_value_name_length: largest_value_name_length,
            _largest_value_data_length: largest_value_data_length,
            _workvar: workvar,
            _key_name_length: key_name_length,
            _class_name_length: class_name_length,
            key_name,
        };

        Ok((input, name_key))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::registry::{hbin::HiveBin, keys::nk::NameKey, parser::Params},
        filesystem::files::read_file,
    };
    use regex::Regex;
    use std::{collections::HashMap, path::PathBuf};

    #[test]
    fn test_parse_name_key() {
        let test_data = [
            110, 107, 32, 0, 144, 202, 172, 141, 217, 236, 216, 1, 0, 0, 0, 0, 40, 8, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 0, 255, 255, 255,
            255, 232, 201, 2, 0, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 8, 0, 0, 0, 85, 115, 101, 114, 68, 97, 116, 97,
        ];
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/registry/win10/hbins.raw");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let (_, result) = HiveBin::parse_hive_bin_header(&buffer).unwrap();

        assert_eq!(result.size, 4096);
        let mut params = Params {
            start_path: String::from("ROOT"),
            path_regex: Regex::new("").unwrap(),
            registry_list: Vec::new(),
            key_tracker: Vec::new(),
            offset_tracker: HashMap::new(),
            filter: false,
            registry_path: String::from("test/test"),
        };

        let (_, result) = NameKey::parse_name_key(&buffer, &test_data, &mut params, 4).unwrap();
        assert_eq!(result, ())
    }
}
