use crate::{
    artifacts::os::windows::registry::{
        cell::{CellType, get_cell_type, is_allocated},
        error::RegistryError,
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
use std::{io::BufReader, num::NonZero};

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
    /// Read Name Key bytes
    pub(crate) fn read_name_key<T: std::io::Seek + std::io::Read>(
        reader: &mut BufReader<T>,
        ntfs_file: Option<&NtfsFile<'_>>,
        offset: u32,
        size: u32,
    ) -> Result<NameKey, RegistryError> {
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

    /// Parse Name Key data
    fn parse_name(data: &[u8]) -> nom::IResult<&[u8], NameKey> {
        // Get the size of the list and check if its allocated (negative numbers = allocated, postive number = unallocated)
        let (input, (allocated, size)) = is_allocated(data)?;

        // Size includes the size itself. We nommed that away
        let adjust_cell_size = 4;
        if size < adjust_cell_size {
            return Err(nom::Err::Incomplete(Needed::Size(NonZero::new(4).unwrap())));
        }

        if !allocated {
            return Err(nom::Err::Incomplete(Needed::Size(NonZero::new(1).unwrap())));
        }

        let (_, cell_data) = take(size - adjust_cell_size)(input)?;

        let (input, cell) = get_cell_type(cell_data)?;

        // Name key cells are the only cells we want, they contain all the info needed to parse the Registry
        if cell != CellType::Nk {
            error!("[registry] Did not get name key cell type: {cell:?}");
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
    use crate::artifacts::os::windows::registry::{
        keys::nk::NameKey, parser::ParamsReader, reader::setup_registry_reader,
    };
    use std::{collections::HashSet, io::BufReader, path::PathBuf};

    #[test]
    fn test_parse_name() {
        let test_data = [
            168, 255, 255, 255, 110, 107, 44, 0, 224, 183, 80, 190, 249, 236, 216, 1, 2, 0, 0, 0,
            168, 8, 0, 0, 10, 0, 0, 0, 0, 0, 0, 0, 176, 6, 0, 0, 255, 255, 255, 255, 0, 0, 0, 0,
            255, 255, 255, 255, 112, 1, 0, 0, 255, 255, 255, 255, 30, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 82, 79, 79, 84, 0, 0, 0, 0, 160, 255, 255, 255,
        ];

        let (_, result) = NameKey::parse_name(&test_data).unwrap();
        assert_eq!(result.key_name, "ROOT");
    }

    #[test]
    fn test_read_name_key() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/registry/win10/NTUSER.DAT");

        let reader = setup_registry_reader(test_location.to_str().unwrap()).unwrap();
        let buf_reader = BufReader::new(reader);

        let mut param_reader = ParamsReader {
            reader: buf_reader,
            offset: 0,
            size: 0,
            minor_version: 0,
            start_path: String::new(),
            path_regex: None,
            filter: false,
            registry_path: String::new(),
            key_tracker: Vec::new(),
            offset_tracker: HashSet::new(),
        };

        let header = param_reader.get_header(None).unwrap();
        assert_eq!(header.filename, "\\??\\C:\\Users\\Default\\NTUSER.DAT");
        let root = NameKey::read_name_key(&mut param_reader.reader, None, 32, 4096).unwrap();

        assert_eq!(root.key_name, "ROOT");
        assert_eq!(root.key_values_offset, -1);
        assert_eq!(root.subkeys_list_offset, 1712);
        assert_eq!(root.number_key_values, 0);
        assert_eq!(root.number_subkeys, 10);
    }
}
