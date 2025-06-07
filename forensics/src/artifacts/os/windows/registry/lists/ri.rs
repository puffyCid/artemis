use super::li::LeafItem;
use crate::artifacts::os::windows::registry::parser::Params;

pub(crate) type RefItem = LeafItem;

impl RefItem {
    /// The Ri (Reference Item) cell type has the same format as Li (Leaf Item)
    pub(crate) fn parse_reference_item<'a>(
        reg_data: &'a [u8],
        ri_data: &'a [u8],
        params: &mut Params,
        minor_version: u32,
    ) -> nom::IResult<&'a [u8], ()> {
        RefItem::parse_leaf_item(reg_data, ri_data, params, minor_version)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::registry::{hbin::HiveBin, lists::ri::RefItem, parser::Params},
        filesystem::files::read_file,
    };
    use regex::Regex;
    use std::{collections::HashMap, path::PathBuf};

    #[test]
    fn test_parse_hash_leaf() {
        let test_data = [114, 105, 0, 0];

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

        let (_, result) =
            RefItem::parse_reference_item(&buffer, &test_data, &mut params, 4).unwrap();
        assert_eq!(result, ())
    }
}
