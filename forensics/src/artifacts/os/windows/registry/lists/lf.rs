use super::lh::HashLeaf;
use crate::{artifacts::os::windows::registry::parser::Params, structs::toml::Output};

pub(crate) type Leaf = HashLeaf;

impl Leaf {
    /// The lf (leaf) cell type has the same format as lh (hash leaf)
    pub(crate) fn parse_leaf<'a>(
        reg_data: &'a [u8],
        lf_data: &'a [u8],
        params: &mut Params,
        minor_version: u32,
        output: &mut Option<&mut Output>,
    ) -> nom::IResult<&'a [u8], ()> {
        Leaf::parse_hash_leaf(reg_data, lf_data, params, minor_version, output)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::registry::{hbin::HiveBin, lists::lf::Leaf, parser::Params},
        filesystem::files::read_file,
    };
    use regex::Regex;
    use std::{collections::HashMap, path::PathBuf};

    #[test]
    fn parse_leaf() {
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
            start_time: 0,
        };

        let (_, result) = Leaf::parse_leaf(&buffer, &test_data, &mut params, 4, &mut None).unwrap();
        assert_eq!(result, ())
    }
}
