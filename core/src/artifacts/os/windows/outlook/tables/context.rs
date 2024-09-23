use super::{
    header::HeapNode,
    properties::{property_id_to_name, PropertyName},
    property::get_property_data,
};
use crate::{
    artifacts::os::windows::outlook::{
        blocks::descriptors::DescriptorData,
        error::OutlookError,
        header::NodeID,
        helper::{OutlookReader, OutlookReaderAction},
        pages::btree::{BlockType, LeafBlockData, NodeLevel},
        tables::{
            header::{get_heap_node_id, heap_page_map, table_header},
            heap_btree::parse_btree_heap,
            property::{extract_property_value, get_map_offset},
        },
    },
    utils::nom_helper::{
        nom_unsigned_four_bytes, nom_unsigned_one_byte, nom_unsigned_two_bytes, Endian,
    },
};
use log::{error, warn};
use nom::{bytes::complete::take, error::ErrorKind};
use ntfs::NtfsFile;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub(crate) struct TableRows {
    pub(crate) value: Value,
    pub(crate) column: ColumnDescriptor,
}

#[derive(Debug, Clone)]
pub(crate) struct ColumnDescriptor {
    pub(crate) property_type: PropertyType,
    pub(crate) id: u16,
    pub(crate) property_name: Vec<PropertyName>,
    offset: u16,
    size: u8,
    index: u8,
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum PropertyType {
    Int16,
    Int32,
    Float32,
    Float64,
    Currency,
    FloatTime,
    ErrorCode,
    Bool,
    Int64,
    String,
    String8,
    Time,
    Guid,
    ServerId,
    Restriction,
    Binary,
    MultiInt16,
    MultiInt32,
    MultiFloat32,
    MultiFloat64,
    MultiCurrency,
    MultiFloatTime,
    MultiInt64,
    MultiString,
    MultiString8,
    MultiTime,
    MultiGuid,
    MultiBinary,
    Unspecified,
    Null,
    Object,
    RuleAction,
    Unknown,
}

pub(crate) trait OutlookTableContext<T: std::io::Seek + std::io::Read> {
    fn table_info(
        &mut self,
        block_data: &[Vec<u8>],
        block_descriptors: &BTreeMap<u64, DescriptorData>,
    ) -> Result<TableInfo, OutlookError>;

    fn get_rows(
        &mut self,
        info: &TableInfo,
        ntfs_file: Option<&NtfsFile<'_>>,
    ) -> Result<Vec<Vec<TableRows>>, OutlookError>;
    fn parse_rows<'a>(
        &mut self,
        info: &TableInfo,
        ntfs_file: Option<&NtfsFile<'_>>,
        data: &'a [u8],
    ) -> nom::IResult<&'a [u8], Vec<Vec<TableRows>>>;
    fn get_branch_rows(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
        info: &TableInfo,
        branch: &TableBranchInfo,
    ) -> Result<Vec<Vec<TableRows>>, OutlookError>;
    fn parse_branch_rows(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
        info: &TableInfo,
        branch: &TableBranchInfo,
    ) -> nom::IResult<&[u8], Vec<Vec<TableRows>>>;
    fn parse_table_info<'a>(
        &mut self,
        data: &'a [u8],
        all_block: &[Vec<u8>],
        descriptors: &BTreeMap<u64, DescriptorData>,
    ) -> nom::IResult<&'a [u8], TableInfo>;

    fn get_descriptor_data(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
        descriptor: &DescriptorData,
    ) -> Result<Vec<Vec<u8>>, OutlookError>;
}

#[derive(Debug, Clone)]
pub(crate) struct TableInfo {
    pub(crate) block_data: Vec<Vec<u8>>,
    pub(crate) block_descriptors: BTreeMap<u64, DescriptorData>,
    pub(crate) rows: Vec<u64>,
    pub(crate) columns: Vec<TableRows>,
    pub(crate) include_cols: Vec<PropertyName>,
    pub(crate) row_size: u16,
    pub(crate) map_offset: u16,
    pub(crate) node: HeapNode,
    pub(crate) total_rows: u64,
    /**Tables may have branches. If they do, getting the data becomes very different vs non-branch tables */
    pub(crate) has_branch: Option<Vec<TableBranchInfo>>,
}

#[derive(Debug, Clone)]
pub(crate) struct TableBranchInfo {
    pub(crate) node: HeapNode,
    pub(crate) rows_info: RowsInfo,
}

impl<T: std::io::Seek + std::io::Read> OutlookTableContext<T> for OutlookReader<T> {
    fn table_info(
        &mut self,
        block_data: &[Vec<u8>],
        block_descriptors: &BTreeMap<u64, DescriptorData>,
    ) -> Result<TableInfo, OutlookError> {
        let first_block = block_data.first();
        let block = match first_block {
            Some(result) => result,
            None => return Err(OutlookError::NoBlocks),
        };

        let props_result = self.parse_table_info(block, block_data, block_descriptors);
        let table = match props_result {
            Ok((_, result)) => result,
            Err(_err) => {
                error!("[outlook] Could not get table info");
                return Err(OutlookError::TableContext);
            }
        };

        Ok(table)
    }

    fn get_rows(
        &mut self,
        info: &TableInfo,
        ntfs_file: Option<&NtfsFile<'_>>,
    ) -> Result<Vec<Vec<TableRows>>, OutlookError> {
        let first_block = info.block_data.first();
        let block = match first_block {
            Some(result) => result,
            None => return Err(OutlookError::NoBlocks),
        };

        let rows_result = self.parse_rows(info, ntfs_file, block);
        let rows = match rows_result {
            Ok((_, result)) => result,
            Err(_err) => {
                error!("[outlook] Could not get table rows");
                return Err(OutlookError::TableContext);
            }
        };
        Ok(rows)
    }

    fn parse_rows<'a>(
        &mut self,
        info: &TableInfo,
        ntfs_file: Option<&NtfsFile<'_>>,
        data: &'a [u8],
    ) -> nom::IResult<&'a [u8], Vec<Vec<TableRows>>> {
        let (input, _header) = table_header(data)?;
        let (input, heap_btree) = parse_btree_heap(input)?;

        let tree_header_size: u8 = 22;
        let (input, _) = take(tree_header_size)(input)?;

        let mut descriptor_data = Vec::new();
        if info.node.node == NodeID::LocalDescriptors {
            if let Some(descriptor) = info.block_descriptors.get(&(info.node.index as u64)) {
                let desc_result = self.get_descriptor_data(ntfs_file, descriptor);
                descriptor_data = match desc_result {
                    Ok(result) => result,
                    Err(err) => {
                        error!("[outlook] Failed to parse descriptor data: {err:?}");
                        return Err(nom::Err::Failure(nom::error::Error::new(
                            &[],
                            ErrorKind::Fail,
                        )));
                    }
                };
            }
        }

        let column_size = 8;
        let column_definition_size = column_size * info.columns.len();
        // Now skip column definitions
        let (input, _) = take(column_definition_size)(input)?;

        if heap_btree.level == NodeLevel::BranchNode {
            if let Some(branch_info) = &info.has_branch {
                for branch in branch_info {
                    if branch.node.block_index as usize > info.block_data.len() {
                        warn!("[outlook] The Branch block index {} is larger than the block data length {}. This should not happen.", branch.node.block_index, info.block_data.len());
                        continue;
                    }

                    // We always check to make sure block_index is less than block data length
                    let rows_result = parse_branch_row(
                        &info.block_data[branch.node.block_index as usize],
                        &descriptor_data,
                        info,
                        branch,
                    );
                    let rows = match rows_result {
                        Ok((_, result)) => result,
                        Err(_err) => {
                            error!("[outlook] Failed to parse branch rows");
                            return Err(nom::Err::Failure(nom::error::Error::new(
                                &[],
                                ErrorKind::Fail,
                            )));
                        }
                    };
                    return Ok((&[], rows));
                }
            }
        }

        if !descriptor_data.is_empty() {
            let rows_result = parse_descriptors(&descriptor_data, info);
            let rows = match rows_result {
                Ok((_, result)) => result,
                Err(_err) => {
                    error!("[outlook] Failed to parse rows from descriptors");
                    return Err(nom::Err::Failure(nom::error::Error::new(
                        &[],
                        ErrorKind::Fail,
                    )));
                }
            };
            return Ok((&[], rows));
        }

        // Rows are found in the block data. Possible if there are only a few rows

        // Now skip Row name and ID section
        let section_size = 8;
        let size = info.total_rows * section_size;
        let (input, _) = take(size)(input)?;

        get_row_data(input, info)
    }

    fn get_branch_rows(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
        info: &TableInfo,
        branch: &TableBranchInfo,
    ) -> Result<Vec<Vec<TableRows>>, OutlookError> {
        let rows_result = self.parse_branch_rows(ntfs_file, info, branch);
        let rows = match rows_result {
            Ok((_, result)) => result,
            Err(_err) => {
                error!("[outlook] Could not get table rows from branch");
                return Err(OutlookError::TableContext);
            }
        };
        Ok(rows)
    }

    fn parse_branch_rows(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
        info: &TableInfo,
        branch: &TableBranchInfo,
    ) -> nom::IResult<&[u8], Vec<Vec<TableRows>>> {
        let mut descriptor_data = Vec::new();

        if info.node.node == NodeID::LocalDescriptors {
            if let Some(descriptor) = info.block_descriptors.get(&(info.node.index as u64)) {
                let desc_result = self.get_descriptor_data(ntfs_file, descriptor);
                descriptor_data = match desc_result {
                    Ok(result) => result,
                    Err(err) => {
                        error!("[outlook] Failed to parse descriptor data: {err:?}");
                        return Err(nom::Err::Failure(nom::error::Error::new(
                            &[],
                            ErrorKind::Fail,
                        )));
                    }
                };
            }
        }

        if branch.node.block_index as usize > info.block_data.len() {
            error!("[outlook] The Branch block index {} is larger than the block data length {}. Stopping parsing.", branch.node.block_index, info.block_data.len());
            return Err(nom::Err::Failure(nom::error::Error::new(
                &[],
                ErrorKind::Fail,
            )));
        }

        // We always check to make sure block index is less than block data length
        let rows_result = parse_branch_row(
            &info.block_data[branch.node.block_index as usize],
            &descriptor_data,
            info,
            branch,
        );
        let rows = match rows_result {
            Ok((_, result)) => result,
            Err(_err) => {
                error!("[outlook] Failed to parse branch rows");
                return Err(nom::Err::Failure(nom::error::Error::new(
                    &[],
                    ErrorKind::Fail,
                )));
            }
        };

        Ok((&[], rows))
    }

    fn parse_table_info<'a>(
        &mut self,
        data: &'a [u8],
        all_block: &[Vec<u8>],
        descriptors: &BTreeMap<u64, DescriptorData>,
    ) -> nom::IResult<&'a [u8], TableInfo> {
        let (input, header) = table_header(data)?;
        let (input, heap_btree) = parse_btree_heap(input)?;

        let (input, _sig) = nom_unsigned_one_byte(input, Endian::Le)?;
        let (input, number_column_definitions) = nom_unsigned_one_byte(input, Endian::Le)?;
        let (input, _array_end_32bit) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, _array_end_16bit) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, _array_end_8bit) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, array_end_offset) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, _table_context_index_reference) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, values_array_index_reference) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let row = get_heap_node_id(&values_array_index_reference);

        let (input, _padding) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, cols) = get_column_definitions(input, &number_column_definitions)?;

        let mut info = TableInfo {
            block_data: all_block.to_vec(),
            block_descriptors: descriptors.clone(),
            rows: Vec::new(),
            columns: cols,
            include_cols: Vec::new(),
            row_size: array_end_offset,
            map_offset: header.page_map_offset,
            total_rows: 0,
            node: row,
            has_branch: None,
        };

        if heap_btree.level == NodeLevel::BranchNode {
            if heap_btree.node.block_index as usize > all_block.len() {
                error!(
                    "[outlook] Block index {} greater than the block length {}.",
                    heap_btree.node.block_index,
                    all_block.len()
                );
                return Err(nom::Err::Failure(nom::error::Error::new(
                    &[],
                    ErrorKind::Fail,
                )));
            }

            // Still not done. We only have references to the data now. We always check to make the block index is less than the block length
            let branch_result = extract_branch_details(
                &all_block[heap_btree.node.block_index as usize],
                &heap_btree.node.index,
            );
            let branch_references = match branch_result {
                Ok((_, result)) => result,
                Err(_err) => {
                    error!("[outlook] Failed to extract branch details");
                    return Err(nom::Err::Failure(nom::error::Error::new(
                        &[],
                        ErrorKind::Fail,
                    )));
                }
            };

            let mut branch_info_vec = Vec::new();
            // Loop through the references and grab the rows
            for branch in branch_references {
                if branch.block_index as usize > all_block.len() {
                    warn!(
                        "[outlook] Branch index {} greater than the block length {}.",
                        branch.block_index,
                        all_block.len()
                    );
                    continue;
                }
                // We always check to make block index is less than the block length
                let rows_result = extract_branch_row(
                    &all_block[branch.block_index as usize],
                    &(branch.index as usize),
                );
                let message_rows = match rows_result {
                    Ok((_, result)) => result,
                    Err(_err) => {
                        error!("[outlook] Failed to parse branch row");
                        return Err(nom::Err::Failure(nom::error::Error::new(
                            &[],
                            ErrorKind::Fail,
                        )));
                    }
                };
                info.total_rows += message_rows.count;

                let branch_info = TableBranchInfo {
                    node: branch,
                    rows_info: message_rows,
                };

                branch_info_vec.push(branch_info);
            }

            // We now have all the data that is needed to extract data from branches
            info.has_branch = Some(branch_info_vec);
        } else if heap_btree.node.block_index != 0 {
            if heap_btree.node.block_index as usize > all_block.len() {
                error!(
                    "[outlook] Block index {} greater than the alternative block length {}.",
                    heap_btree.node.block_index,
                    all_block.len()
                );
                return Err(nom::Err::Failure(nom::error::Error::new(
                    &[],
                    ErrorKind::Fail,
                )));
            }
            let rows_result = block_row_count(
                &all_block[heap_btree.node.block_index as usize],
                &heap_btree.node.index,
            );
            info.total_rows = match rows_result {
                Ok((_, result)) => result,
                Err(_err) => {
                    error!("[outlook] Failed to parse block row count");
                    return Err(nom::Err::Failure(nom::error::Error::new(
                        &[],
                        ErrorKind::Fail,
                    )));
                }
            };
        } else {
            info.total_rows = get_row_count(&header.page_map.allocation_table);
        };

        Ok((input, info))
    }

    fn get_descriptor_data(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
        descriptor: &DescriptorData,
    ) -> Result<Vec<Vec<u8>>, OutlookError> {
        let mut leaf_block = LeafBlockData {
            block_type: BlockType::Internal,
            index_id: 0,
            index: 0,
            block_offset: 0,
            size: 0,
            total_size: 0,
            reference_count: 0,
        };
        let mut leaf_descriptor = None;
        for block_tree in &self.block_btree {
            if let Some(block_data) = block_tree.get(&descriptor.block_data_id) {
                leaf_block = *block_data;

                if descriptor.block_descriptor_id == 0 {
                    break;
                }
            }
            if let Some(block_data) = block_tree.get(&descriptor.block_descriptor_id) {
                leaf_descriptor = Some(*block_data);
            }

            if leaf_descriptor.is_none() && leaf_block.size != 0 {
                break;
            }
        }
        let value = self.get_block_data(ntfs_file, &leaf_block, leaf_descriptor.as_ref())?;

        Ok(value.data)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RowsInfo {
    pub(crate) row_end: u16,
    pub(crate) count: u64,
}

/// Extract the rows found in branches. This involves a lot more work then non-branch rows
fn extract_branch_row<'a>(data: &'a [u8], map_index: &usize) -> nom::IResult<&'a [u8], RowsInfo> {
    let (_, map_offset) = nom_unsigned_two_bytes(data, Endian::Le)?;
    let (map_start, _) = take(map_offset)(data)?;

    let (_, map) = heap_page_map(map_start)?;

    let mut branch_row_start = 0;
    let mut branch_row_end = 0;

    if let Some(start) = map.allocation_table.get(*map_index - 1) {
        if let Some(end) = map.allocation_table.get(*map_index) {
            branch_row_start = *start;
            branch_row_end = *end;
        }
    }

    let branch_row_size = branch_row_end - branch_row_start;
    let row_size = 8;
    if branch_row_size % row_size != 0 {
        error!("[outlook] Branch row size should be a multiple of 8 bytes. Something went wrong. Got size: {branch_row_size}. Ending parsing early");
        let info = RowsInfo {
            row_end: branch_row_end,
            count: 0,
        };
        return Ok((&[], info));
    }

    let row_count = branch_row_size / row_size;

    let info = RowsInfo {
        row_end: branch_row_end,
        count: row_count as u64,
    };

    Ok((&[], info))
}

/// Parse rows found in branches
fn extract_branch_details<'a>(
    data: &'a [u8],
    map_index: &u32,
) -> nom::IResult<&'a [u8], Vec<HeapNode>> {
    let (_, map_offset) = nom_unsigned_two_bytes(data, Endian::Le)?;
    let (map_start, _) = take(map_offset)(data)?;

    let (_, map) = heap_page_map(map_start)?;

    let mut branch_row_start = 0;
    let mut branch_row_end = 0;

    let adjust = 1;
    if let Some(start) = map.allocation_table.get(*map_index as usize - adjust) {
        if let Some(end) = map.allocation_table.get(*map_index as usize) {
            branch_row_start = *start;
            branch_row_end = *end;
        }
    }

    let branch_row_size = branch_row_end - branch_row_start;
    let row_size = 8;
    if branch_row_size % row_size != 0 {
        error!("[outlook] Branch details row size should be a multiple of 8 bytes. Something went wrong. Got size: {branch_row_size}. Ending parsing early");
        return Ok((&[], Vec::new()));
    }

    let row_count = branch_row_size / row_size;
    // Go to start of the Row branches
    let (row_start, _) = take(branch_row_start)(data)?;
    // Get the entire size of the data
    let (_, mut row_data) = take(branch_row_size)(row_start)?;

    let mut refs = Vec::new();
    let mut count = 0;
    while count < row_count {
        let (input, _id) = nom_unsigned_four_bytes(row_data, Endian::Le)?;
        let (input, table_ref) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let results = get_heap_node_id(&table_ref);
        row_data = input;
        count += 1;

        refs.push(results);
    }

    Ok((&[], refs))
}

/// Determine row count in block data
fn block_row_count<'a>(data: &'a [u8], heap_index: &u32) -> nom::IResult<&'a [u8], u64> {
    let (_, map_offset) = nom_unsigned_two_bytes(data, Endian::Le)?;
    let (map_start, _) = take(map_offset)(data)?;

    let (_, map) = heap_page_map(map_start)?;

    let mut branch_row_start = 0;
    let mut branch_row_end = 0;
    let adjust = 1;
    if let Some(start) = map.allocation_table.get(*heap_index as usize - adjust) {
        if let Some(end) = map.allocation_table.get(*heap_index as usize) {
            branch_row_start = *start;
            branch_row_end = *end;
        }
    }

    let branch_row_size = branch_row_end - branch_row_start;
    let row_size = 8;
    if branch_row_size % row_size != 0 {
        error!("[outlook] Block row size should be a multiple of 8 bytes. Something went wrong. Got size: {branch_row_size}. Ending parsing early");
        return Ok((&[], 0));
    }

    let count = branch_row_size / row_size;

    Ok((&[], count as u64))
}

/// Determine the total rows
fn get_row_count(map: &[u16]) -> u64 {
    if map.len() < 4 {
        // There are no rows
        return 0;
    }

    // We check above to make the array length is 4 or higher
    let row_start = map[2];
    let row_end = map[3];
    let rows = row_end - row_start;

    let row_size = 8;
    if rows % row_size != 0 {
        warn!("[outlook] Row size should be a multiple of 8 bytes. Something went wrong. Got size: {rows}. Ending parsing early");
        return 0;
    }

    let count = rows / row_size;
    count as u64
}

/// Parse all descriptor blocks in order to get table row data
fn parse_descriptors<'a>(
    descriptors: &[Vec<u8>],
    info: &TableInfo,
) -> nom::IResult<&'a [u8], Vec<Vec<TableRows>>> {
    if descriptors.is_empty() {
        return Ok((&[], Vec::new()));
    }

    let mut desc_index = 0;
    let mut rows = Vec::new();
    // Determine the max number of rows per descriptor block. We check above to make sure array is not empty
    let max_rows = descriptors[0].len() / info.row_size as usize;

    // Determine which descriptor index we need to start at
    // Ex: We have 3 descriptor indexes each have 200 rows. We want row 303. We need descriptor index 1
    if let Some(first_row) = info.rows.first() {
        desc_index = *first_row as usize / max_rows;
    }

    let mut count = 0;
    for entry in &info.rows {
        let mut index = *entry;

        while index as usize >= max_rows {
            index -= max_rows as u64;
        }

        // If the number of rows exceeds the max rows in descriptor, we move to the next one
        if count == max_rows {
            desc_index += 1;
            count = 0;
        }

        if desc_index > descriptors.len() {
            warn!("[outlook] THe descriptor index {desc_index} is larger than the descriptor array length {}. This should not happen", descriptors.len());
            break;
        }

        // We always check to make sure the desc_index is less then descriptors array length
        let row_result = get_row_data_entry(&descriptors[desc_index], &index, info);
        let row = match row_result {
            Ok((_, result)) => result,
            Err(_err) => {
                error!("[outlook] Failed to parse descriptors for table");
                return Err(nom::Err::Failure(nom::error::Error::new(
                    &[],
                    ErrorKind::Fail,
                )));
            }
        };
        count += 1;
        rows.push(row);
    }

    Ok((&[], rows))
}

/// Parse row data located in Branches. This involves a lot more work vs non-branch rows
fn parse_branch_row<'a>(
    data: &'a [u8],
    descriptors: &[Vec<u8>],
    info: &TableInfo,
    branch: &TableBranchInfo,
) -> nom::IResult<&'a [u8], Vec<Vec<TableRows>>> {
    if !descriptors.is_empty() {
        return parse_descriptors(descriptors, info);
    }

    // Bypass everything until the start of the row entries
    let (row_data_start, _) = take(branch.rows_info.row_end)(data)?;
    let (_, rows) = get_row_data(row_data_start, info)?;

    Ok((&[], rows))
}

/// Get row data found in Branches
fn get_row_data_entry<'a>(
    data: &'a [u8],
    entry: &u64,
    info: &TableInfo,
) -> nom::IResult<&'a [u8], Vec<TableRows>> {
    // Go to the start of the row
    let (row_start, _) = take(entry * info.row_size as u64)(data)?;
    let (_, row_data) = take(info.row_size)(row_start)?;

    // Give each row column info
    let mut col = info.columns.clone();
    for column in col.iter_mut() {
        let (_, value) = parse_row_data(
            &info.block_data,
            row_data,
            &column.column.property_type,
            &column.column.offset,
            &column.column.size,
        )?;

        column.value = value;
    }

    Ok((data, col))
}

/// Get row data. This is where are Outlook data exists
fn get_row_data<'a>(
    data: &'a [u8],
    info: &TableInfo,
) -> nom::IResult<&'a [u8], Vec<Vec<TableRows>>> {
    let mut rows = Vec::new();

    // Get the rows we want
    for entry in &info.rows {
        // Go to the start of the row
        let (row_start, _) = take(entry * info.row_size as u64)(data)?;
        let (_, row_data) = take(info.row_size)(row_start)?;

        // Give each row column info
        let mut col = info.columns.clone();
        for column in col.iter_mut() {
            let (_, value) = parse_row_data(
                &info.block_data,
                row_data,
                &column.column.property_type,
                &column.column.offset,
                &column.column.size,
            )?;

            column.value = value;
        }

        rows.push(col);
    }

    Ok((data, rows))
}

/// Finally parse the row data and return the Outlook data
fn parse_row_data<'a>(
    all_blocks: &[Vec<u8>],
    row_data: &'a [u8],
    prop_type: &PropertyType,
    offset: &u16,
    value_size: &u8,
) -> nom::IResult<&'a [u8], Value> {
    let mut value = Value::Null;
    let (value_start, _) = take(*offset)(row_data)?;
    let (_, value_data) = take(*value_size)(value_start)?;

    let multi_values = [
        PropertyType::String,
        PropertyType::String8,
        PropertyType::MultiBinary,
        PropertyType::Binary,
        PropertyType::MultiString,
        PropertyType::MultiString8,
        PropertyType::MultiCurrency,
        PropertyType::MultiFloat32,
        PropertyType::MultiFloat64,
        PropertyType::FloatTime,
        PropertyType::MultiGuid,
        PropertyType::Guid,
        PropertyType::MultiInt16,
        PropertyType::MultiInt32,
        PropertyType::MultiInt64,
        PropertyType::MultiTime,
        PropertyType::ServerId,
    ];
    if multi_values.contains(prop_type) {
        let (_, offset) = nom_unsigned_four_bytes(value_data, Endian::Le)?;
        if offset == 0 {
            return Ok((row_data, value));
        }
        let (block_index, map_start) = get_map_offset(&offset);
        if let Some(block_data) = all_blocks.get(block_index as usize) {
            let prop_result = get_property_data(block_data, prop_type, &map_start, &false);
            let prop_value = match prop_result {
                Ok((_, result)) => result,
                Err(_err) => {
                    error!("[outlook] Failed to parse the property data associated with {prop_type:?}. Data could be malformed.");
                    return Ok((&[], value));
                }
            };
            value = prop_value;
        }
    } else {
        let (_, prop_value) = extract_property_value(value_data, prop_type)?;
        value = prop_value;
    }

    Ok((row_data, value))
}

/// Extract column definitions for our table. There can be a lot
fn get_column_definitions<'a>(
    data: &'a [u8],
    column_count: &u8,
) -> nom::IResult<&'a [u8], Vec<TableRows>> {
    let mut col_data = data;
    let mut count = 0;

    let mut values = Vec::new();

    while &count < column_count {
        let (input, property_type) = nom_unsigned_two_bytes(col_data, Endian::Le)?;
        let (input, id) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, offset) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, size) = nom_unsigned_one_byte(input, Endian::Le)?;
        let (input, index) = nom_unsigned_one_byte(input, Endian::Le)?;

        col_data = input;

        let column = ColumnDescriptor {
            property_type: get_property_type(&property_type),
            property_name: property_id_to_name(&format!(
                "0x{:04x?}_0x{:04x?}",
                &id, &property_type
            )),
            id,
            offset,
            size,
            index,
        };

        let row = TableRows {
            value: Value::Null,
            column,
        };

        values.push(row);
        count += 1;
    }

    Ok((col_data, values))
}

/// Return the `PropertyType` name
pub(crate) fn get_property_type(prop: &u16) -> PropertyType {
    match prop {
        1 => PropertyType::Null,
        0 => PropertyType::Unspecified,
        13 => PropertyType::Object,
        2 => PropertyType::Int16,
        3 => PropertyType::Int32,
        4 => PropertyType::Float32,
        5 => PropertyType::Float64,
        6 => PropertyType::Currency,
        7 => PropertyType::FloatTime,
        10 => PropertyType::ErrorCode,
        11 => PropertyType::Bool,
        20 => PropertyType::Int64,
        31 => PropertyType::String,
        30 => PropertyType::String8,
        64 => PropertyType::Time,
        72 => PropertyType::Guid,
        251 => PropertyType::ServerId,
        253 => PropertyType::Restriction,
        254 => PropertyType::RuleAction,
        258 => PropertyType::Binary,
        4098 => PropertyType::MultiInt16,
        4099 => PropertyType::MultiInt32,
        4100 => PropertyType::MultiFloat32,
        4101 => PropertyType::MultiFloat64,
        4102 => PropertyType::MultiCurrency,
        4103 => PropertyType::MultiFloatTime,
        4116 => PropertyType::MultiInt64,
        4127 => PropertyType::MultiString,
        4126 => PropertyType::MultiString8,
        4160 => PropertyType::MultiTime,
        4168 => PropertyType::MultiGuid,
        4354 => PropertyType::MultiBinary,
        _ => PropertyType::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        block_row_count, get_column_definitions, get_property_type, get_row_count, get_row_data,
        ColumnDescriptor, TableRows,
    };
    use crate::artifacts::os::windows::outlook::{
        header::NodeID,
        tables::{
            context::{PropertyType, TableInfo},
            header::HeapNode,
            properties::PropertyName,
        },
    };
    use serde_json::Value;
    use std::collections::BTreeMap;

    #[test]
    fn test_get_row_count() {
        let test = [0, 1, 8, 16];
        let rows = get_row_count(&test);
        assert_eq!(rows, 1);
    }

    #[test]
    fn test_get_row_data() {
        let data = [
            2, 32, 0, 0, 60, 0, 0, 0, 160, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 1, 255, 0, 162, 32, 0, 0, 62, 2, 0, 0, 192, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 255, 0, 82, 0, 111, 0, 111,
            0, 116, 0, 32, 0, 45, 0, 32, 0, 80, 0, 117, 0, 98, 0, 108, 0, 105, 0, 99, 0, 82, 0,
            111, 0, 111, 0, 116, 0, 32, 0, 45, 0, 32, 0, 77, 0, 97, 0, 105, 0, 108, 0, 98, 0, 111,
            0, 120, 0, 6, 0, 0, 0, 12, 0, 20, 0, 162, 0, 178, 0, 56, 1, 82, 1, 110, 1,
        ];

        let info = TableInfo {
            block_data: vec![vec![
                110, 1, 236, 124, 64, 0, 0, 0, 0, 0, 0, 0, 181, 4, 4, 0, 96, 0, 0, 0, 124, 15, 64,
                0, 64, 0, 65, 0, 67, 0, 32, 0, 0, 0, 128, 0, 0, 0, 0, 0, 0, 0, 2, 1, 48, 14, 32, 0,
                4, 8, 20, 0, 51, 14, 36, 0, 8, 9, 2, 1, 52, 14, 44, 0, 4, 10, 3, 0, 56, 14, 48, 0,
                4, 11, 31, 0, 1, 48, 8, 0, 4, 2, 3, 0, 2, 54, 12, 0, 4, 3, 3, 0, 3, 54, 16, 0, 4,
                4, 11, 0, 10, 54, 64, 0, 1, 5, 31, 0, 19, 54, 52, 0, 4, 12, 3, 0, 53, 102, 56, 0,
                4, 13, 3, 0, 54, 102, 60, 0, 4, 14, 3, 0, 56, 102, 20, 0, 4, 6, 3, 0, 242, 103, 0,
                0, 4, 0, 3, 0, 243, 103, 4, 0, 4, 1, 20, 0, 244, 103, 24, 0, 8, 7, 2, 32, 0, 0, 0,
                0, 0, 0, 162, 32, 0, 0, 1, 0, 0, 0, 2, 32, 0, 0, 60, 0, 0, 0, 160, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 255, 0, 162,
                32, 0, 0, 62, 2, 0, 0, 192, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 255, 0, 82, 0, 111, 0, 111, 0, 116, 0, 32, 0, 45, 0,
                32, 0, 80, 0, 117, 0, 98, 0, 108, 0, 105, 0, 99, 0, 82, 0, 111, 0, 111, 0, 116, 0,
                32, 0, 45, 0, 32, 0, 77, 0, 97, 0, 105, 0, 108, 0, 98, 0, 111, 0, 120, 0, 6, 0, 0,
                0, 12, 0, 20, 0, 162, 0, 178, 0, 56, 1, 82, 1, 110, 1,
            ]],
            block_descriptors: BTreeMap::new(),
            rows: vec![0, 1],
            columns: vec![
                TableRows {
                    value: Value::Null,
                    column: ColumnDescriptor {
                        property_type: PropertyType::String,
                        id: 12289,
                        property_name: vec![PropertyName::PidTagDisplayNameW],
                        offset: 8,
                        size: 4,
                        index: 2,
                    },
                },
                TableRows {
                    value: Value::Null,
                    column: ColumnDescriptor {
                        property_type: PropertyType::Int32,
                        id: 13826,
                        property_name: vec![PropertyName::PidTagContentCount],
                        offset: 12,
                        size: 4,
                        index: 3,
                    },
                },
                TableRows {
                    value: Value::Null,
                    column: ColumnDescriptor {
                        property_type: PropertyType::Int32,
                        id: 13827,
                        property_name: vec![PropertyName::PidTagContentUnreadCount],
                        offset: 16,
                        size: 4,
                        index: 4,
                    },
                },
                TableRows {
                    value: Value::Null,
                    column: ColumnDescriptor {
                        property_type: PropertyType::Bool,
                        id: 13834,
                        property_name: vec![PropertyName::PidTagSubfolders],
                        offset: 64,
                        size: 1,
                        index: 5,
                    },
                },
                TableRows {
                    value: Value::Null,
                    column: ColumnDescriptor {
                        property_type: PropertyType::Int32,
                        id: 26610,
                        property_name: vec![PropertyName::PidTagLtpRowId],
                        offset: 0,
                        size: 4,
                        index: 0,
                    },
                },
                TableRows {
                    value: Value::Null,
                    column: ColumnDescriptor {
                        property_type: PropertyType::Int32,
                        id: 26611,
                        property_name: vec![PropertyName::PidTagLtpRowVer],
                        offset: 4,
                        size: 4,
                        index: 1,
                    },
                },
            ],
            include_cols: Vec::new(),
            row_size: 67,
            map_offset: 366,
            node: HeapNode {
                node: NodeID::HeapNode,
                index: 4,
                block_index: 0,
            },
            total_rows: 2,
            has_branch: None,
        };
        let (_, results) = get_row_data(&data, &info).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(
            results[0][0].value,
            Value::String(String::from("Root - Public"))
        );
    }

    #[test]
    fn test_get_column_definitions() {
        let test = [
            2, 1, 48, 14, 32, 0, 4, 8, 20, 0, 51, 14, 36, 0, 8, 9, 2, 1, 52, 14, 44, 0, 4, 10, 3,
            0, 56, 14, 48, 0, 4, 11, 31, 0, 1, 48, 8, 0, 4, 2, 3, 0, 2, 54, 12, 0, 4, 3, 3, 0, 3,
            54, 16, 0, 4, 4, 11, 0, 10, 54, 64, 0, 1, 5, 31, 0, 19, 54, 52, 0, 4, 12, 3, 0, 53,
            102, 56, 0, 4, 13, 3, 0, 54, 102, 60, 0, 4, 14, 3, 0, 56, 102, 20, 0, 4, 6, 3, 0, 242,
            103, 0, 0, 4, 0, 3, 0, 243, 103, 4, 0, 4, 1, 20, 0, 244, 103, 24, 0, 8, 7, 34, 32, 0,
            0, 0, 0, 0, 0, 66, 32, 0, 0, 1, 0, 0, 0, 34, 32, 0, 0, 11, 0, 0, 0, 160, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 0, 66, 32, 0, 0,
            61, 0, 0, 0, 192, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 1, 255, 0, 73, 0, 80, 0, 77, 0, 95, 0, 83, 0, 85, 0, 66, 0, 84, 0, 82, 0, 69,
            0, 69, 0, 78, 0, 79, 0, 78, 0, 95, 0, 73, 0, 80, 0, 77, 0, 95, 0, 83, 0, 85, 0, 66, 0,
            84, 0, 82, 0, 69, 0, 69, 0, 6, 0, 0, 0, 12, 0, 20, 0, 162, 0, 178, 0, 56, 1, 78, 1,
            108, 1,
        ];
        let (_, rows) = get_column_definitions(&test, &15).unwrap();
        assert_eq!(rows.len(), 15);
    }

    #[test]
    fn test_get_property_type() {
        let test = 13;
        let prop = get_property_type(&test);
        assert_eq!(prop, PropertyType::Object);
    }

    #[test]
    fn test_block_row_count() {
        let test = [
            0, 0, 233, 0, 22, 0, 2, 0, 74, 0, 142, 0, 164, 0, 254, 0, 30, 1, 92, 1, 108, 1, 106, 3,
            129, 3, 151, 3, 111, 7, 127, 7, 35, 8, 195, 8, 217, 8, 81, 9, 133, 9, 255, 9, 15, 10,
            13, 12, 36, 12, 58, 12, 74, 12, 218, 12, 102, 13, 124, 13, 244, 13, 40, 14, 162, 14,
            178, 14, 176, 16, 199, 16, 221, 16, 237, 16, 145, 17, 49, 18, 71, 18, 191, 18, 243, 18,
            109, 19, 125, 19, 123, 21, 146, 21, 168, 21, 184, 21, 44, 22, 156, 22, 178, 22, 42, 23,
            94, 23, 216, 23, 232, 23, 230, 25, 253, 25, 19, 26, 19, 26, 19, 26, 19, 26, 19, 26, 19,
            26, 19, 26, 19, 26, 19, 26, 19, 26, 19, 26, 19, 26, 19, 26, 19, 26, 19, 26, 19, 26, 19,
            26, 19, 26, 19, 26, 19, 26, 19, 26, 117, 26, 237, 26, 101, 27, 221, 27, 85, 28, 205,
            28, 69, 29, 197, 29, 69, 30, 189, 30, 53, 31, 173, 31, 37, 32, 157, 32, 21, 33, 141,
            33, 239, 33, 103, 34, 223, 34, 87, 35, 207, 35, 71, 36, 191, 36, 55, 37, 153, 37, 17,
            38, 137, 38, 1, 39, 99, 39, 219, 39, 83, 40, 203, 40, 45, 41, 165, 41, 29, 42, 149, 42,
            13, 43, 133, 43, 231, 43, 95, 44, 215, 44, 57, 45, 177, 45, 41, 46, 139, 46, 45, 47,
            165, 47, 71, 48, 191, 48, 55, 49, 199, 49, 63, 50, 183, 50, 49, 51, 211, 51, 77, 52,
            175, 52, 39, 53, 137, 53, 1, 54, 121, 54, 241, 54, 83, 55, 205, 55, 71, 56, 193, 56,
            35, 57, 157, 57, 45, 58, 163, 58, 29, 59, 149, 59, 15, 60, 135, 60, 1, 61, 121, 61,
            243, 61, 109, 62, 231, 62, 73, 63, 193, 63, 35, 64, 155, 64, 43, 65, 141, 65, 5, 66,
            103, 66, 223, 66, 87, 67, 207, 67, 49, 68, 147, 68, 11, 69, 155, 69, 19, 70, 117, 70,
            215, 70, 79, 71, 223, 71, 65, 72, 185, 72, 41, 73, 161, 73, 3, 74, 123, 74, 221, 74,
            55, 75, 175, 75, 17, 76, 107, 76, 227, 76, 69, 77, 197, 77, 69, 78, 189, 78, 23, 79,
            143, 79, 7, 80, 127, 80, 247, 80, 247, 80, 247, 80, 7, 81, 129, 81, 249, 81, 111, 82,
            133, 82, 253, 82, 49, 83, 171, 83, 187, 83, 185, 85, 208, 85, 230, 85, 246, 85, 140,
            86, 4, 87, 150, 87, 172, 87, 36, 88, 88, 88, 210, 88, 226, 88, 224, 90, 247, 90, 13,
            91, 29, 91, 185, 91, 49, 92, 201, 92, 223, 92, 87, 93, 139, 93, 5, 94, 21, 94, 19, 96,
            42, 96, 64, 96,
        ];

        let (_, rows) = block_row_count(&test, &11).unwrap();
        assert_eq!(rows, 0);
    }
}
