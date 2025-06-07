use crate::{
    artifacts::os::windows::registry::{cell::walk_registry, parser::Params},
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
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::registry::{hbin::HiveBin, lists::lh::HashLeaf, parser::Params},
        filesystem::files::read_file,
    };
    use regex::Regex;
    use std::{collections::HashMap, path::PathBuf};

    #[test]
    fn test_parse_hash_leaf() {
        let test_data = [
            108, 104, 10, 0, 120, 0, 0, 0, 134, 188, 172, 5, 232, 0, 0, 0, 35, 142, 192, 85, 104,
            2, 0, 0, 185, 215, 156, 189, 184, 3, 0, 0, 201, 206, 72, 111, 24, 4, 0, 0, 53, 37, 55,
            0, 32, 224, 0, 0, 135, 110, 2, 229, 16, 79, 2, 0, 38, 205, 0, 127, 112, 4, 0, 0, 114,
            216, 82, 191, 88, 5, 0, 0, 99, 20, 254, 233, 176, 5, 0, 0, 249, 208, 65, 97, 0, 0, 0,
            0, 0, 0, 0, 0,
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
            registry_path: String::from("path/NTUSER.dat"),
        };

        let (_, result) = HashLeaf::parse_hash_leaf(&buffer, &test_data, &mut params, 4).unwrap();
        assert_eq!(result, ())
    }
}
