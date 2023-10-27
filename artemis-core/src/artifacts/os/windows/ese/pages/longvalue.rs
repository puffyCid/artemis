use super::root::parse_root_page;
use crate::{
    artifacts::os::windows::ese::{
        page::{PageFlags, PageHeader},
        pages::{
            branch::BranchPage,
            leaf::{LeafType, PageLeaf},
        },
        tags::TagFlags,
    },
    filesystem::ntfs::{reader::read_bytes, sector_reader::SectorReader},
};
use log::{error, warn};
use nom::{bytes::complete::take, error::ErrorKind};
use ntfs::NtfsFile;
use std::collections::HashMap;
use std::{fs::File, io::BufReader};

/**
 * Parse long value page into a `HashMap`  
 * long value is data too large to fit in a cell
 * Columns that have long value data can use this `HashMap` to lookup the column actual data
 */
pub(crate) fn parse_long_value<'a>(
    page_lv_data: &'a [u8],
    ntfs_file: &NtfsFile<'_>,
    fs: &mut BufReader<SectorReader<File>>,
) -> nom::IResult<&'a [u8], HashMap<Vec<u8>, Vec<u8>>> {
    let (page_data, table_page_data) = PageHeader::parse_header(page_lv_data)?;
    let mut has_root = false;
    if table_page_data.page_flags.contains(&PageFlags::Root) {
        let (_, _root_page) = parse_root_page(page_data)?;
        has_root = true;
    }

    let mut key_data: Vec<u8> = Vec::new();
    let mut values: HashMap<Vec<u8>, Vec<u8>> = HashMap::new();
    let mut has_key = true;
    for tag in &table_page_data.page_tags {
        // Defunct tags are not used
        if tag.flags.contains(&TagFlags::Defunct) {
            continue;
        }
        // If first tag is Root, we already parsed that
        if has_root {
            has_root = false;
            has_key = false;
            continue;
        } else if key_data.is_empty() && has_key {
            let (key_start, _) = take(tag.offset)(page_data)?;
            let (_, page_key_data) = take(tag.value_size)(key_start)?;
            key_data = page_key_data.to_vec();
            continue;
        }

        if table_page_data.page_flags.contains(&PageFlags::Leaf) {
            let (leaf_start, _) = take(tag.offset)(page_data)?;
            let (_, leaf_data) = take(tag.value_size)(leaf_start)?;
            if leaf_data.is_empty() {
                continue;
            }

            let leaf_result = PageLeaf::parse_leaf_page(
                leaf_data,
                &table_page_data.page_flags,
                &key_data,
                &tag.flags,
            );
            let (_, mut leaf_row) = match leaf_result {
                Ok(result) => result,
                Err(_err) => {
                    error!("[ese] Failed to parse leaf page for leaf long value");
                    return Err(nom::Err::Failure(nom::error::Error::new(
                        leaf_data,
                        ErrorKind::Fail,
                    )));
                }
            };
            if leaf_row.leaf_type != LeafType::LongValue {
                continue;
            }

            // If long value has prefix data then we need to append the suffix data to it
            // Otherwise just use the suffix data
            let long_key = if key_data.is_empty() {
                leaf_row.key_suffix
            } else {
                let mut prefix = leaf_row.key_prefix;
                prefix.append(&mut leaf_row.key_suffix);
                prefix
            };
            // The serde type for LeafType::LongValue is always Vec<u8>
            let value_data: Vec<u8> =
                serde_json::from_value(leaf_row.leaf_data).unwrap_or_default();
            values.insert(long_key, value_data);

            continue;
        }

        let (branch_start, _) = take(tag.offset)(page_data)?;
        let (_, branch_data) = take(tag.value_size)(branch_start)?;
        let (_, branch) = BranchPage::parse_branch_page(branch_data, &tag.flags)?;

        let adjust_page = 1;
        let branch_start = (branch.child_page + adjust_page) as usize * page_lv_data.len();

        // Now get the child page
        let child_result = read_bytes(
            &(branch_start as u64),
            page_lv_data.len() as u64,
            ntfs_file,
            fs,
        );
        let child_data = match child_result {
            Ok(result) => result,
            Err(err) => {
                error!("[ese] Failed to read bytes for long value child data: {err:?}");
                return Err(nom::Err::Failure(nom::error::Error::new(
                    &[],
                    ErrorKind::Fail,
                )));
            }
        };

        let result = parse_long_value_child(&child_data, &mut values);
        if result.is_err() {
            error!("[ese] Failed to parse long value child");
        }
    }

    Ok((page_data, values))
}

/// Parse the child page associated with the long value data
fn parse_long_value_child<'a>(
    data: &'a [u8],
    values: &mut HashMap<Vec<u8>, Vec<u8>>,
) -> nom::IResult<&'a [u8], ()> {
    let (page_data, branch_page_data) = PageHeader::parse_header(data)?;

    let mut has_root = false;
    if branch_page_data.page_flags.contains(&PageFlags::Root) {
        let (_, _root_page) = parse_root_page(page_data)?;
        has_root = true;
    }

    let mut key_data: Vec<u8> = Vec::new();
    let mut has_key = true;
    for tag in branch_page_data.page_tags {
        // Defunct tags are not used
        if tag.flags.contains(&TagFlags::Defunct) {
            continue;
        }
        // If first tag is Root, we already parsed that
        if has_root {
            has_root = false;
            has_key = false;
            continue;
        } else if key_data.is_empty() && has_key {
            let (key_start, _) = take(tag.offset)(page_data)?;
            let (_, page_key_data) = take(tag.value_size)(key_start)?;
            key_data = page_key_data.to_vec();
            continue;
        }

        if branch_page_data.page_flags.contains(&PageFlags::Leaf) {
            let (leaf_start, _) = take(tag.offset)(page_data)?;
            let (_, leaf_data) = take(tag.value_size)(leaf_start)?;
            if leaf_data.is_empty() {
                continue;
            }

            let leaf_result = PageLeaf::parse_leaf_page(
                leaf_data,
                &branch_page_data.page_flags,
                &key_data,
                &tag.flags,
            );
            let (_, mut leaf_row) = match leaf_result {
                Ok(result) => result,
                Err(_err) => {
                    error!("[ese] Failed to parse leaf page for child long value");
                    return Err(nom::Err::Failure(nom::error::Error::new(
                        leaf_data,
                        ErrorKind::Fail,
                    )));
                }
            };
            if leaf_row.leaf_type != LeafType::LongValue {
                continue;
            }

            // If long value has prefix data then we need to append the suffix data to it
            // Otherwise just use the suffix data
            let long_key = if key_data.is_empty() {
                leaf_row.key_suffix
            } else {
                let mut prefix = leaf_row.key_prefix;
                prefix.append(&mut leaf_row.key_suffix);
                prefix
            };
            // The serde type for LeafType::LongValue is always Vec<u8>
            let value_data: Vec<u8> =
                serde_json::from_value(leaf_row.leaf_data).unwrap_or_default();
            values.insert(long_key, value_data);

            continue;
        }
        warn!(
            "[ese] Non-leaf type page flag: {:?}",
            branch_page_data.page_flags
        );
    }
    Ok((data, ()))
}

#[cfg(test)]
mod tests {
    use super::parse_long_value;
    use crate::{
        artifacts::os::windows::ese::pages::longvalue::parse_long_value_child,
        filesystem::{
            files::read_file,
            ntfs::{raw_files::raw_reader, setup::setup_ntfs_parser},
        },
    };
    use std::{collections::HashMap, path::PathBuf};

    #[test]
    fn test_parse_long_value() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\ese\\win10\\longvalue_page.raw");

        let lv = read_file(test_location.to_str().unwrap()).unwrap();
        test_location.pop();
        test_location.push("qmgr.db");
        let binding = test_location.display().to_string();
        let mut ntfs_parser = setup_ntfs_parser(&'C').unwrap();

        let reader = raw_reader(&binding, &ntfs_parser.ntfs, &mut ntfs_parser.fs).unwrap();
        let (_, results) = parse_long_value(&lv, &reader, &mut ntfs_parser.fs).unwrap();
        assert_eq!(results.len(), 94);
    }

    #[test]
    fn test_parse_long_value_child() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/ese/win10/long_value_child.raw");

        let lv = read_file(test_location.to_str().unwrap()).unwrap();
        let mut values = HashMap::new();
        let (_, _) = parse_long_value_child(&lv, &mut values).unwrap();
        assert_eq!(values.len(), 12);
    }
}
