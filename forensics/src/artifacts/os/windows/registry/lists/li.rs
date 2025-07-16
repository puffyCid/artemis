use crate::{
    artifacts::os::windows::registry::{cell::walk_registry_list, keys::nk::NameKey},
    utils::nom_helper::{Endian, nom_unsigned_four_bytes, nom_unsigned_two_bytes},
};
use ntfs::NtfsFile;
use std::{collections::HashSet, io::BufReader};

#[derive(Debug)]
pub(crate) struct LeafItem {
    _sig: u16,
    number_entries: u16,
}

impl LeafItem {
    /// Read Leaf Item data and RItem data
    pub(crate) fn read_leaf_item<'a, T: std::io::Seek + std::io::Read>(
        reader: &mut BufReader<T>,
        ntfs_file: Option<&NtfsFile<'_>>,
        li_data: &'a [u8],
        minor_version: u32,
        offset_tracker: &mut HashSet<u32>,
        size: u32,
        names: &mut Vec<NameKey>,
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
    use crate::artifacts::os::windows::registry::{
        lists::li::LeafItem, reader::setup_registry_reader,
    };
    use std::{collections::HashSet, io::BufReader, path::PathBuf};

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
        assert_eq!(result, ());
    }
}
