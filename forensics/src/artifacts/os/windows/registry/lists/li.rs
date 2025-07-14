use std::{collections::HashSet, io::BufReader};

use ntfs::NtfsFile;

use crate::{
    artifacts::os::windows::registry::{
        cell::{walk_registry, walk_registry_list},
        keys::nk::NameKey,
        parser::Params,
    },
    utils::nom_helper::{Endian, nom_unsigned_four_bytes, nom_unsigned_two_bytes},
};

#[derive(Debug)]
pub(crate) struct LeafItem {
    _sig: u16,
    number_entries: u16,
}

impl LeafItem {
    /// Parse the Leaf Item (Li) list which points to a list of offsets
    pub(crate) fn parse_leaf_item<'a>(
        reg_data: &'a [u8],
        li_data: &'a [u8],
        params: &mut Params,
        minor_version: u32,
    ) -> nom::IResult<&'a [u8], ()> {
        let (input, sig) = nom_unsigned_two_bytes(li_data, Endian::Le)?;
        let (mut input, number_entries) = nom_unsigned_two_bytes(input, Endian::Le)?;

        let li_list = LeafItem {
            _sig: sig,
            number_entries,
        };

        let mut entry_count = 0;
        while entry_count < li_list.number_entries {
            let (li_input, offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
            entry_count += 1;
            input = li_input;

            let empty_offset = 0;
            if offset == empty_offset {
                continue;
            }

            walk_registry(reg_data, offset, params, minor_version)?;
        }
        Ok((reg_data, ()))
    }

    pub(crate) fn read_leaf_item<'a, T: std::io::Seek + std::io::Read>(
        reader: &mut BufReader<T>,
        ntfs_file: Option<&NtfsFile<'_>>,
        li_data: &'a [u8],
        minor_version: u32,
        offset_tracker: &mut HashSet<u32>,
        size: u32,
        names: &mut Vec<NameKey>,
    ) -> nom::IResult<&'a [u8], ()> {
        panic!("{li_data:?}");
        let (input, sig) = nom_unsigned_two_bytes(li_data, Endian::Le)?;
        let (mut input, number_entries) = nom_unsigned_two_bytes(input, Endian::Le)?;

        let li_list = LeafItem {
            _sig: sig,
            number_entries,
        };

        let mut entry_count = 0;
        while entry_count < li_list.number_entries {
            let (li_input, offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
            entry_count += 1;
            input = li_input;

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
        Ok((li_data, ()))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::registry::{
            hbin::HiveBin, lists::li::LeafItem, parser::Params, reader::setup_registry_reader,
        },
        filesystem::files::read_file,
    };
    use regex::Regex;
    use std::{
        collections::{HashMap, HashSet},
        io::BufReader,
        path::PathBuf,
    };

    #[test]
    fn test_parse_hash_leaf() {
        let test_data = [108, 105, 0, 0];

        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/registry/win10/hbins.raw");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let (_, result) = HiveBin::parse_hive_bin_header(&buffer).unwrap();

        assert_eq!(result.size, 4096);
        let mut params = Params {
            start_path: String::from(""),
            path_regex: Regex::new("").unwrap(),
            registry_list: Vec::new(),
            key_tracker: Vec::new(),
            offset_tracker: HashMap::new(),
            filter: false,
            registry_path: String::from("path/NTUSER.dat"),
        };

        let (_, result) = LeafItem::parse_leaf_item(&buffer, &test_data, &mut params, 4).unwrap();
        assert_eq!(result, ());
    }
    #[test]
    fn test_read_leaf_item() {
        let test_data = [108, 105, 0, 0];

        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/registry/win10/NTUSER.DAT");

        let reader = setup_registry_reader(test_location.to_str().unwrap()).unwrap();
        let mut buf_reader = BufReader::new(reader);

        let mut tracker = HashSet::new();
        let mut names = Vec::new();
        let (_, result) = LeafItem::read_leaf_item(
            &mut buf_reader,
            None,
            &test_data,
            4,
            &mut tracker,
            4096,
            &mut names,
        )
        .unwrap();
        println!("{names:?}");
        assert_eq!(result, ());
    }
}
