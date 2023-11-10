use super::root::parse_root_page;
use crate::{
    artifacts::os::windows::ese::{
        catalog::Catalog,
        page::{PageFlags, PageHeader},
        pages::leaf::{LeafType, PageLeaf},
        tables::{clear_column_data, parse_row, ColumnInfo},
        tags::TagFlags,
    },
    filesystem::ntfs::{reader::read_bytes, sector_reader::SectorReader},
    utils::nom_helper::{nom_unsigned_four_bytes, nom_unsigned_two_bytes, Endian},
};
use log::{error, warn};
use nom::{bytes::complete::take, error::ErrorKind};
use ntfs::NtfsFile;
use std::collections::HashMap;
use std::{fs::File, io::BufReader};

#[derive(Debug)]
pub(crate) struct BranchPage {
    common_page_key_size: u16,
    local_page_key_size: u16,
    local_page_key: Vec<u8>,
    pub(crate) child_page: u32,
}

impl BranchPage {
    /**
     * Branch pages point to another page that contains the actual tagged data  
     * We ultimately follow the child page to parse the data
     */
    pub(crate) fn parse_branch_page<'a>(
        data: &'a [u8],
        tag_flag: &[TagFlags],
    ) -> nom::IResult<&'a [u8], BranchPage> {
        let mut branch_page = BranchPage {
            common_page_key_size: 0,
            local_page_key_size: 0,
            local_page_key: Vec::new(),
            child_page: 0,
        };
        let mut branch_data = data;
        if tag_flag.contains(&TagFlags::CommonKey) {
            let (input, common_page_key_size) = nom_unsigned_two_bytes(branch_data, Endian::Le)?;
            branch_data = input;
            branch_page.common_page_key_size = common_page_key_size;
        }
        let (input, local_page_key_size) = nom_unsigned_two_bytes(branch_data, Endian::Le)?;
        let (input, local_page_key) = take(local_page_key_size)(input)?;
        let (input, child_page) = nom_unsigned_four_bytes(input, Endian::Le)?;

        branch_page.local_page_key_size = local_page_key_size;
        branch_page.local_page_key = local_page_key.to_vec();
        branch_page.child_page = child_page;
        Ok((input, branch_page))
    }

    /// Parse child branch pages related to catalog. Only care about tags that have data definition type
    pub(crate) fn parse_branch_child_catalog<'a>(
        data: &'a [u8],
        page_tracker: &mut HashMap<u32, bool>,
        ntfs_file: &NtfsFile<'_>,
        fs: &mut BufReader<SectorReader<File>>,
    ) -> nom::IResult<&'a [u8], Vec<Catalog>> {
        let (page_data, branch_page_data) = PageHeader::parse_header(data)?;

        let mut has_root = false;
        if branch_page_data.page_flags.contains(&PageFlags::Root) {
            let (_, _root_page) = parse_root_page(page_data)?;
            has_root = true;
        }

        let mut catalog_rows: Vec<Catalog> = Vec::new();
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
                let (_, leaf_row) = match leaf_result {
                    Ok(result) => result,
                    Err(_err) => {
                        error!("[ese] Failed to parse leaf page for catalog branch child");
                        return Err(nom::Err::Failure(nom::error::Error::new(
                            leaf_data,
                            ErrorKind::Fail,
                        )));
                    }
                };
                if leaf_row.leaf_type != LeafType::DataDefinition {
                    continue;
                }
                let catalog = Catalog::parse_row(leaf_row);
                catalog_rows.push(catalog);
                continue;
            }

            let (branch_start, _) = take(tag.offset)(page_data)?;
            let (_, branch_data) = take(tag.value_size)(branch_start)?;
            let (_, branch) = BranchPage::parse_branch_page(branch_data, &tag.flags)?;

            if let Some(_page) = page_tracker.get(&branch.child_page) {
                warn!("[ese] Found a catalog branch child recursively pointing to same page {}. Exiting early", branch.child_page);
                return Ok((data, catalog_rows));
            }
            // Track child pages so dont end up in a rescursive loop (ex: child points back to parent)
            page_tracker.insert(branch.child_page, true);

            let adjust_page = 1;
            let branch_start = (branch.child_page + adjust_page) as usize * data.len();
            // Now get the child page
            let child_result = read_bytes(&(branch_start as u64), data.len() as u64, ntfs_file, fs);
            let child_data = match child_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[ese] Could not read child page data: {err:?}");
                    return Err(nom::Err::Failure(nom::error::Error::new(
                        &[],
                        ErrorKind::Fail,
                    )));
                }
            };
            let rows_results =
                BranchPage::parse_branch_child_catalog(&child_data, page_tracker, ntfs_file, fs);
            let (_, mut rows) = if let Ok(results) = rows_results {
                results
            } else {
                error!("[ese] Could not parse child branch");
                continue;
            };
            catalog_rows.append(&mut rows);
        }
        Ok((data, catalog_rows))
    }

    /// Parse child branch pages related to tables. Only care about tags that have data definition type
    pub(crate) fn parse_branch_child_table<'a>(
        page_branch_data: &'a [u8],
        column_info: &mut [ColumnInfo],
        column_rows: &mut Vec<Vec<ColumnInfo>>,
        page_tracker: &mut HashMap<u32, bool>,
        ntfs_file: &NtfsFile<'_>,
        fs: &mut BufReader<SectorReader<File>>,
    ) -> nom::IResult<&'a [u8], u32> {
        let (page_data, branch_page_data) = PageHeader::parse_header(page_branch_data)?;
        // Empty pages are not part of table data
        if branch_page_data.page_flags.contains(&PageFlags::Empty) {
            return Ok((page_branch_data, 0));
        }

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
                let (_, leaf_row) = match leaf_result {
                    Ok(result) => result,
                    Err(_err) => {
                        error!("[ese] Failed to parse leaf page for table branch child");
                        return Err(nom::Err::Failure(nom::error::Error::new(
                            leaf_data,
                            ErrorKind::Fail,
                        )));
                    }
                };
                if leaf_row.leaf_type != LeafType::DataDefinition {
                    continue;
                }
                parse_row(leaf_row, column_info);
                column_rows.push(column_info.to_vec());

                // Now clear column data so when we go to next row we have no leftover data from previous row
                clear_column_data(column_info);
                continue;
            }

            let (branch_start, _) = take(tag.offset)(page_data)?;
            let (_, branch_data) = take(tag.value_size)(branch_start)?;
            let (_, branch) = BranchPage::parse_branch_page(branch_data, &tag.flags)?;

            if let Some(_page) = page_tracker.get(&branch.child_page) {
                warn!("[ese] Found a table branch child recursively pointing to same page {}. Exiting early", branch.child_page);
                return Ok((page_branch_data, 0));
            }
            // Track child pages so dont end up in a rescursive loop (ex: child points back to parent)
            page_tracker.insert(branch.child_page, true);

            let adjust_page = 1;
            let branch_start = (branch.child_page + adjust_page) as usize * page_branch_data.len();

            // Now get the child page
            let child_result = read_bytes(
                &(branch_start as u64),
                page_branch_data.len() as u64,
                ntfs_file,
                fs,
            );
            let child_data = match child_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[ese] Failed to read bytes for child data: {err:?}");
                    return Err(nom::Err::Failure(nom::error::Error::new(
                        &[],
                        ErrorKind::Fail,
                    )));
                }
            };

            let result = BranchPage::parse_branch_child_table(
                &child_data,
                column_info,
                column_rows,
                page_tracker,
                ntfs_file,
                fs,
            );
            if result.is_err() {
                error!("[ese] Failed to parse branch child table");
            }
        }
        Ok((page_branch_data, branch_page_data.next_page_number))
    }
}

#[cfg(test)]
mod tests {
    use common::windows::ColumnType;

    use super::BranchPage;
    use crate::{
        artifacts::os::windows::ese::tables::{ColumnFlags, ColumnInfo},
        filesystem::{
            files::read_file,
            ntfs::{raw_files::raw_reader, setup::setup_ntfs_parser},
        },
    };
    use std::{collections::HashMap, path::PathBuf};

    #[test]
    fn test_parse_branch_child_catalog() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\ese\\win10\\branch_child_search.raw");
        let test_data = read_file(test_location.to_str().unwrap()).unwrap();
        let mut tracker = HashMap::new();

        let mut ntfs_parser =
            setup_ntfs_parser(&test_location.to_str().unwrap().chars().next().unwrap()).unwrap();
        let ntfs_file = raw_reader(
            test_location.to_str().unwrap(),
            &ntfs_parser.ntfs,
            &mut ntfs_parser.fs,
        )
        .unwrap();

        let (_, results) = BranchPage::parse_branch_child_catalog(
            &test_data,
            &mut tracker,
            &ntfs_file,
            &mut ntfs_parser.fs,
        )
        .unwrap();

        assert_eq!(results.len(), 241);
        assert_eq!(results[0].name, "MSysObjects");
        assert_eq!(results[84].obj_id_table, 10);
        assert_eq!(results[185].flags, 4096);
        assert_eq!(results[230].root_flag, 0);
    }

    #[test]
    fn test_parse_branch_child_table() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\ese\\win10\\branch_child_table.raw");
        let test_data = read_file(test_location.to_str().unwrap()).unwrap();
        let mut tracker = HashMap::new();
        let mut info = vec![ColumnInfo {
            column_type: ColumnType::Long,
            column_name: String::from("IdFileLocal"),
            column_data: Vec::new(),
            column_id: 1,
            column_flags: vec![ColumnFlags::AutoIncrement],
            column_space_usage: 4,
            column_tagged_flags: Vec::new(),
        }];
        let mut rows = Vec::new();
        let mut ntfs_parser =
            setup_ntfs_parser(&test_location.to_str().unwrap().chars().next().unwrap()).unwrap();

        let reader = raw_reader(
            test_location.to_str().unwrap(),
            &ntfs_parser.ntfs,
            &mut ntfs_parser.fs,
        )
        .unwrap();
        let (_, last_page) = BranchPage::parse_branch_child_table(
            &test_data,
            &mut info,
            &mut rows,
            &mut tracker,
            &reader,
            &mut ntfs_parser.fs,
        )
        .unwrap();

        assert_eq!(rows.len(), 10);
        assert_eq!(rows[0][0].column_name, "IdFileLocal");
        assert_eq!(rows[0][0].column_data, [3, 0, 0, 0]);
        assert_eq!(rows[4][0].column_data, [56, 0, 0, 0]);
        assert_eq!(rows[9][0].column_data, [61, 0, 0, 0]);
        assert_eq!(last_page, 607);
    }

    #[test]
    fn test_parse_branch_page() {
        let test_data = [
            13, 0, 127, 128, 0, 0, 3, 127, 128, 2, 127, 128, 0, 1, 4, 13, 0, 0, 0,
        ];

        let (_, results) = BranchPage::parse_branch_page(&test_data, &Vec::new()).unwrap();
        assert_eq!(results.common_page_key_size, 0);
        assert_eq!(results.local_page_key_size, 13);
        assert_eq!(
            results.local_page_key,
            [127, 128, 0, 0, 3, 127, 128, 2, 127, 128, 0, 1, 4]
        );
        assert_eq!(results.child_page, 13);
    }
}
