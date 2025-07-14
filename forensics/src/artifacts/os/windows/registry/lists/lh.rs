use std::{collections::HashSet, io::BufReader};

use log::error;
use ntfs::NtfsFile;

use crate::{
    artifacts::os::windows::registry::{
        cell::{walk_registry, walk_registry_list},
        error::RegistryError,
        keys::nk::NameKey,
        parser::Params,
    },
    filesystem::ntfs::reader::read_bytes,
    utils::nom_helper::{Endian, nom_unsigned_four_bytes, nom_unsigned_two_bytes},
};

#[derive(Debug)]
pub(crate) struct HashLeaf {
    _sig: u16,
    number_entries: u16,
}

impl HashLeaf {
    pub(crate) fn parse_hash_leaf<'a>(
        reg_data: &'a [u8],
        lh_data: &'a [u8],
        params: &mut Params,
        minor_version: u32,
    ) -> nom::IResult<&'a [u8], ()> {
        let (input, sig) = nom_unsigned_two_bytes(lh_data, Endian::Le)?;
        let (mut input, number_entries) = nom_unsigned_two_bytes(input, Endian::Le)?;

        let lh_list = HashLeaf {
            _sig: sig,
            number_entries,
        };

        let mut entry_count = 0;
        while entry_count < lh_list.number_entries {
            let (lh_input, offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
            let (lh_input, _hash) = nom_unsigned_four_bytes(lh_input, Endian::Le)?;
            entry_count += 1;
            input = lh_input;

            let empty_offset = 0;
            if offset == empty_offset {
                continue;
            }

            walk_registry(reg_data, offset, params, minor_version)?;
        }
        Ok((reg_data, ()))
    }

    pub(crate) fn read_hash_leaf<'a, T: std::io::Seek + std::io::Read>(
        reader: &mut BufReader<T>,
        ntfs_file: Option<&NtfsFile<'_>>,
        lh_data: &'a [u8],
        minor_version: u32,
        offset_tracker: &mut HashSet<u32>,
        size: u32,
        names: &mut Vec<NameKey>,
    ) -> nom::IResult<&'a [u8], ()> {
        let (input, sig) = nom_unsigned_two_bytes(lh_data, Endian::Le)?;
        let (mut input, number_entries) = nom_unsigned_two_bytes(input, Endian::Le)?;

        let lh_list = HashLeaf {
            _sig: sig,
            number_entries,
        };

        let mut entry_count = 0;
        while entry_count < lh_list.number_entries {
            let (lh_input, offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
            let (lh_input, _hash) = nom_unsigned_four_bytes(lh_input, Endian::Le)?;
            entry_count += 1;
            input = lh_input;

            let empty_offset = 0;
            if offset == empty_offset {
                continue;
            }

            let _ = walk_registry_list(
                reader,
                ntfs_file,
                minor_version,
                offset_tracker,
                offset,
                size,
                names,
            );
        }
        Ok((lh_data, ()))
    }
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::registry::{
        lists::lh::HashLeaf, reader::setup_registry_reader,
    };
    use std::{collections::HashSet, io::BufReader, path::PathBuf};

    #[test]
    fn test_read_hash_leaf() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/registry/win10/NTUSER.DAT");

        let reader = setup_registry_reader(test_location.to_str().unwrap()).unwrap();
        let mut buf_reader = BufReader::new(reader);

        let test_data = [
            108, 104, 10, 0, 120, 0, 0, 0, 134, 188, 172, 5, 232, 0, 0, 0, 35, 142, 192, 85, 104,
            2, 0, 0, 185, 215, 156, 189, 184, 3, 0, 0, 201, 206, 72, 111, 24, 4, 0, 0, 53, 37, 55,
            0, 32, 224, 0, 0, 135, 110, 2, 229, 16, 79, 2, 0, 38, 205, 0, 127, 112, 4, 0, 0, 114,
            216, 82, 191, 88, 5, 0, 0, 99, 20, 254, 233, 176, 5, 0, 0, 249, 208, 65, 97, 0, 0, 0,
            0, 0, 0, 0, 0,
        ];

        let mut tracker = HashSet::new();
        let mut names = Vec::new();
        let (_, _) = HashLeaf::read_hash_leaf(
            &mut buf_reader,
            None,
            &test_data,
            3,
            &mut tracker,
            4096,
            &mut names,
        )
        .unwrap();

        assert_eq!(names.len(), 10);
        assert_eq!(names[8].key_name, "Network")
    }
}
