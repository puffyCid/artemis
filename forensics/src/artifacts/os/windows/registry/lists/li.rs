use crate::{
    artifacts::os::windows::registry::{cell::walk_registry, parser::Params},
    structs::toml::Output,
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
        output: &mut Option<&mut Output>,
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

            walk_registry(reg_data, offset, params, minor_version, output)?;
        }
        Ok((reg_data, ()))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::registry::{hbin::HiveBin, lists::li::LeafItem, parser::Params},
        filesystem::files::read_file,
    };
    use regex::Regex;
    use std::{collections::HashMap, path::PathBuf};

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
            start_time: 0,
        };

        let (_, result) =
            LeafItem::parse_leaf_item(&buffer, &test_data, &mut params, 4, &mut None).unwrap();
        assert_eq!(result, ());
    }
}
