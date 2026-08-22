/**
 * Extensible Storage Engine (`ESE`) is an open source database format used by various Windows applications  
 * Such as: Windows Search (Pre-Win11), Windows Catalog files, BITS, SRUM, Windows Updates, and lots more  
 *
 * Its an extremely complex format, currently we focus on providing the ability to dump table rows which contains the data of interest  
 * Often `ESE` files are locked so we use the NTFS parser to read the files (`raw_read_file`)
 *
 * References:  
 * `https://github.com/libyal/libesedb/blob/main/documentation/Extensible%20Storage%20Engine%20(ESE)%20Database%20File%20(EDB)%20format.asciidoc`
 * `https://github.com/Velocidex/go-ese`
 * `https://techcommunity.microsoft.com/t5/ask-the-directory-services-team/ese-deep-dive-part-1-the-anatomy-of-an-ese-database/ba-p/400496`
 * `https://github.com/microsoft/Extensible-Storage-Engine`
 *
 * Other Parsers:  
 * `https://github.com/Velocidex/velociraptor`
 */
use super::{
    catalog::Catalog,
    error::EseError,
    header::EseHeader,
    page::{PageFlags, PageHeader},
    pages::{longvalue::parse_long_value, root::parse_root_page},
    tables::{ColumnInfo, TableInfo, create_table_data},
};
use crate::{
    accessor::{access::Accessor, entry::handle::FileHandle, io::reader::AccessorReader},
    artifacts::os::windows::ese::{
        pages::{
            branch::BranchPage,
            leaf::{LeafType, PageLeaf},
        },
        tables::{clear_column_data, parse_row},
        tags::TagFlags,
    },
    utils::nom_helper::nom_data,
};
use common::windows::{ColumnType, TableDump};
use std::{collections::HashMap, io::Read};
use tracing::error;

/// Get `Catalog` data from provided ESE path
pub(crate) fn get_catalog_info(handle: &FileHandle) -> Result<Vec<Catalog>, EseError> {
    let mut reader = open_ese_handle(handle)?;
    let page_size = ese_page_size(&mut reader)?;

    Catalog::grab_catalog(&mut reader, page_size)
}

/// Get all pages from ESE table. First page can be found from the `Catalog`
pub(crate) fn get_all_pages(handle: &FileHandle, first_page: u32) -> Result<Vec<u32>, EseError> {
    let mut reader = open_ese_handle(handle)?;
    let page_size = ese_page_size(&mut reader)?;
    let pages = get_pages(first_page, &mut reader, page_size)?;

    Ok(pages)
}

/// Get all page data (rows) from table based on array of pages
pub(crate) fn get_page_data(
    handle: &FileHandle,
    pages: &[u32],
    info: &mut TableInfo,
    name: &str,
) -> Result<HashMap<String, Vec<Vec<TableDump>>>, EseError> {
    let mut total_rows = HashMap::new();
    total_rows.insert(name.to_string(), Vec::new());

    let last_page = 0;
    let mut reader = open_ese_handle(handle)?;
    let page_size = ese_page_size(&mut reader)?;
    let mut column_rows = Vec::new();

    for page in pages {
        if page == &last_page {
            continue;
        }
        let mut page_rows = page_data(*page, &mut reader, page_size, info)?;
        column_rows.append(&mut page_rows);
    }
    let mut rows = row_data(&mut column_rows, &mut reader, page_size, info, name)?;

    if let Some(values) = rows.get_mut(name) {
        total_rows
            .entry(name.to_string())
            .or_insert(Vec::new())
            .append(values);
    }

    Ok(total_rows)
}

/// Get all filtered page data (rows) from table based on array of pages
pub(crate) fn get_filtered_page_data(
    handle: &FileHandle,
    pages: &[u32],
    info: &mut TableInfo,
    name: &str,
    column_name: &str,
    column_values: &mut HashMap<String, bool>,
) -> Result<HashMap<String, Vec<Vec<TableDump>>>, EseError> {
    let mut reader = open_ese_handle(handle)?;
    let page_size = ese_page_size(&mut reader)?;

    let mut total_rows = HashMap::new();
    total_rows.insert(name.to_string(), Vec::new());
    let mut column_rows = Vec::new();

    for page in pages {
        let mut page_rows = page_data(*page, &mut reader, page_size, info)?;
        column_rows.append(&mut page_rows);
    }

    let rows = row_data(&mut column_rows, &mut reader, page_size, info, name)?;
    if let Some(values) = rows.get(name) {
        for rows in values {
            for columns in rows {
                if columns.column_name != column_name {
                    continue;
                }

                if column_values.is_empty() {
                    return Ok(total_rows);
                }

                if column_values.get(&columns.column_data).is_some() {
                    total_rows
                        .entry(name.to_string())
                        .or_insert(Vec::new())
                        .push(rows.clone());

                    column_values.remove(&columns.column_data);
                }
                break;
            }
        }
    }

    Ok(total_rows)
}

/// Get specified columns from table
pub(crate) fn dump_table_columns(
    handle: &FileHandle,
    pages: &[u32],
    info: &mut TableInfo,
    name: &str,
    column_names: &[String],
) -> Result<HashMap<String, Vec<Vec<TableDump>>>, EseError> {
    let mut reader = open_ese_handle(handle)?;
    let page_size = ese_page_size(&mut reader)?;

    let mut total_rows = HashMap::new();
    total_rows.insert(name.to_string(), Vec::new());
    let mut column_rows = Vec::new();

    for page in pages {
        let mut page_rows = page_data(*page, &mut reader, page_size, info)?;
        column_rows.append(&mut page_rows);
    }
    let rows = row_data(&mut column_rows, &mut reader, page_size, info, name)?;

    if let Some(values) = rows.get(name) {
        for rows in values {
            let mut filter_columns = Vec::new();
            for columns in rows {
                if !column_names.contains(&columns.column_name) {
                    continue;
                }

                filter_columns.push(columns.clone());
            }
            total_rows
                .entry(name.to_string())
                .or_insert(Vec::new())
                .push(filter_columns);
        }
    }

    Ok(total_rows)
}

/// Create a `AccessorReader` from a `FileHandle`
fn open_ese_handle(handle: &FileHandle) -> Result<AccessorReader, EseError> {
    let mut accessor = Accessor::with_defaults();

    let reader = match accessor.open_reader_handle(handle) {
        Ok(results) => results,
        Err(err) => {
            error!(
                "Could not open handle to {}: {err:?}",
                handle.display_path()
            );
            return Err(EseError::ParseEse);
        }
    };

    Ok(reader)
}

/// Determine page size for ESE database
fn ese_page_size(reader: &mut AccessorReader) -> Result<u32, EseError> {
    let header_size = 668;
    let mut buf = vec![0; header_size];
    if let Err(err) = reader.read(&mut buf) {
        error!("Failed to reader header bytes: {err:?}");
        return Err(EseError::ParseEse);
    };

    let db_result = EseHeader::parse_header(&buf);
    let (_, db_header) = match db_result {
        Ok(result) => result,
        Err(_err) => {
            error!("Failed to parse ESE header");
            return Err(EseError::ParseEse);
        }
    };

    Ok(db_header.page_size)
}

/// Get array of pages
fn get_pages(
    first_page: u32,
    reader: &mut AccessorReader,
    page_size: u32,
) -> Result<Vec<u32>, EseError> {
    // Need to adjust page number to account for header page
    let adjust_page = 1;
    let page_number = (first_page + adjust_page) * page_size;

    if let Err(err) = reader.seek_from_start(page_number as u64) {
        error!("Failed to seek to page offset {page_number}: {err:?}");
        return Err(EseError::ParseEse);
    }
    let mut buf = vec![0; page_size as usize];
    if let Err(err) = reader.read(&mut buf) {
        error!("Failed to read bytes for page start: {err:?}");
        return Err(EseError::ParseEse);
    };

    // Start parsing the page associated with the table data
    let page_header_result = PageHeader::parse_header(&buf);
    let (page_data, table_page_data) = match page_header_result {
        Ok(result) => result,
        Err(_err) => {
            error!("Failed to parse ESE header");
            return Err(EseError::ParseEse);
        }
    };

    let mut has_root = false;
    if table_page_data.page_flags.contains(&PageFlags::Root) {
        let root_page_result = parse_root_page(page_data);
        if root_page_result.is_err() {
            error!("Failed to parse root page. Stopping parsing");
            return Err(EseError::ParseEse);
        }
        has_root = true;
    }

    let mut pages = Vec::new();
    pages.push(first_page);

    for tag in table_page_data.page_tags {
        // Defunct tags are not used
        if tag.flags.contains(&TagFlags::Defunct) {
            continue;
        }
        // First tag is Root, we already parsed that
        if has_root {
            has_root = false;
            continue;
        }

        if table_page_data.page_flags.contains(&PageFlags::Leaf) {
            continue;
        }

        let branch_result = nom_data(page_data, tag.offset.into());
        let (branch_start, _) = match branch_result {
            Ok(result) => result,
            Err(_err) => {
                error!("Failed to get branch start data");
                return Err(EseError::ParseEse);
            }
        };
        let branch_result = nom_data(branch_start, tag.value_size.into());
        let (_, branch_data) = match branch_result {
            Ok(result) => result,
            Err(_err) => {
                error!("Failed to get branch data");
                return Err(EseError::ParseEse);
            }
        };
        let branch_result = BranchPage::parse_branch_page(branch_data, &tag.flags);
        let (_, branch) = match branch_result {
            Ok(result) => result,
            Err(_err) => {
                error!("Failed to get branch page data");
                return Err(EseError::ParseEse);
            }
        };

        let adjust_page = 1;
        let branch_start = (branch.child_page + adjust_page) * page_size;
        pages.push(branch.child_page);

        // Now get the child page
        if let Err(err) = reader.seek_from_start(branch_start as u64) {
            error!("Failed to seek to branch offset {branch_start}: {err:?}");
            return Err(EseError::ParseEse);
        }

        let mut buf = vec![0; page_size as usize];
        if let Err(err) = reader.read(&mut buf) {
            error!("Failed to read bytes for child data: {err:?}");
            return Err(EseError::ParseEse);
        };

        // Track child pages so do not end up in a recursive loop (ex: child points back to parent)
        let mut page_tracker: HashMap<u32, bool> = HashMap::new();
        let last_result =
            BranchPage::parse_branch_child_page(&buf, &mut pages, &mut page_tracker, reader);

        if last_result.is_err() {
            error!("Could not parse branch child table and last page in page tags");
            return Err(EseError::ParseEse);
        }
    }

    Ok(pages)
}

/// Start parsing the page data to get rows
fn page_data(
    page: u32,
    reader: &mut AccessorReader,
    page_size: u32,
    info: &mut TableInfo,
) -> Result<Vec<Vec<ColumnInfo>>, EseError> {
    // Need to adjust page number to account for header page
    let adjust_page = 1;
    let page_number = (page + adjust_page) * page_size;
    if let Err(err) = reader.seek_from_start(page_number as u64) {
        error!("Failed to seek to page offset {page_number}: {err:?}");
        return Err(EseError::ParseEse);
    }

    let mut buf = vec![0; page_size as usize];
    if let Err(err) = reader.read(&mut buf) {
        error!("Failed to read bytes for page start: {err:?}");
        return Err(EseError::ParseEse);
    };

    // Start parsing the page associated with the table data
    let page_header_result = PageHeader::parse_header(&buf);
    let (page_data, table_page_data) = match page_header_result {
        Ok(result) => result,
        Err(_err) => {
            error!("Failed to parse ESE header");
            return Err(EseError::ParseEse);
        }
    };

    let mut has_root = false;
    if table_page_data.page_flags.contains(&PageFlags::Root) {
        let root_page_result = parse_root_page(page_data);
        if root_page_result.is_err() {
            error!("Failed to parse root page. Stopping parsing");
            return Err(EseError::ParseEse);
        }
        has_root = true;
    }

    let mut column_rows: Vec<Vec<ColumnInfo>> = Vec::new();
    let mut has_key = true;
    let mut key_data: Vec<u8> = Vec::new();

    for tag in table_page_data.page_tags {
        // Defunct tags are not used
        if tag.flags.contains(&TagFlags::Defunct) {
            continue;
        }
        // First tag is Root, we already parsed that
        if has_root {
            has_root = false;
            has_key = false;
            continue;
        }

        if key_data.is_empty() && has_key {
            let key_result = nom_data(page_data, tag.offset.into());
            let (key_start, _) = match key_result {
                Ok(result) => result,
                Err(_err) => {
                    error!("Failed to get key data");
                    return Err(EseError::ParseEse);
                }
            };
            let page_key_data_result = nom_data(key_start, tag.value_size.into());
            let (_, page_key_data) = match page_key_data_result {
                Ok(result) => result,
                Err(_err) => {
                    error!("Failed to get page key data");
                    return Err(EseError::ParseEse);
                }
            };
            key_data = page_key_data.to_vec();
            continue;
        }

        if !table_page_data.page_flags.contains(&PageFlags::Leaf) {
            continue;
        }

        let leaf_result = nom_data(page_data, tag.offset.into());
        let (leaf_start, _) = match leaf_result {
            Ok(result) => result,
            Err(_err) => {
                error!("Failed to get leaf data");
                return Err(EseError::ParseEse);
            }
        };
        let leaf_result = nom_data(leaf_start, tag.value_size.into());
        let (_, leaf_data) = match leaf_result {
            Ok(result) => result,
            Err(_err) => {
                error!("Failed to get leaf data");
                return Err(EseError::ParseEse);
            }
        };

        let leaf_result = PageLeaf::parse_leaf_page(
            leaf_data,
            &table_page_data.page_flags,
            &key_data,
            &tag.flags,
        );
        let (_, leaf_row) = match leaf_result {
            Ok(result) => result,
            Err(_err) => {
                error!("Failed to parse leaf page {page}");
                return Err(EseError::ParseEse);
            }
        };
        if leaf_row.leaf_type != LeafType::DataDefinition {
            continue;
        }

        parse_row(leaf_row, &mut info.column_info);
        column_rows.push(info.column_info.clone());
        // Now clear column data so when we go to next row we have no leftover data from previous row
        clear_column_data(&mut info.column_info);
    }

    Ok(column_rows)
}

/// Extract row data into generic ESE `TableDump`
fn row_data(
    rows: &mut Vec<Vec<ColumnInfo>>,
    reader: &mut AccessorReader,
    page_size: u32,
    info: &mut TableInfo,
    name: &str,
) -> Result<HashMap<String, Vec<Vec<TableDump>>>, EseError> {
    if info.long_value_page == 0 {
        let table_data = create_table_data(rows, name);
        return Ok(table_data);
    }

    let adjust_page = 1;
    // Need to adjust page number to account for header page
    let page_number = (info.long_value_page as u32 + adjust_page) * page_size;

    if let Err(err) = reader.seek_from_start(page_number as u64) {
        error!("Failed to seek to page offset {page_number}: {err:?}");
        return Err(EseError::ParseEse);
    }
    let mut buf = vec![0; page_size as usize];
    if let Err(err) = reader.read(&mut buf) {
        error!("Failed to read bytes for child data: {err:?}");
        return Err(EseError::ParseEse);
    };

    let long_result = parse_long_value(&buf, reader);
    let (_, long_values) = match long_result {
        Ok(result) => result,
        Err(_err) => {
            error!("Could not get long value data");
            return Err(EseError::ParseEse);
        }
    };

    // Now we check if columns have longbinary, longtext column types
    // And update the data
    for column_row in &mut *rows {
        for column in column_row {
            if (column.column_type == ColumnType::LongBinary
                || column.column_type == ColumnType::LongText)
                && !column.column_data.is_empty()
            {
                let mut col = column.column_data.clone();
                // Long value key is actually Big Endian
                col.reverse();

                let mut final_prefix = vec![0, 0, 0, 0];
                col.append(&mut final_prefix);
                if let Some(value) = long_values.get(&col) {
                    column.column_data.clone_from(value);
                }
            }
        }
    }

    // Finally done, now just need to create an abstracted table dump where we parse non-binary column data
    let table_data = create_table_data(rows, name);

    Ok(table_data)
}

#[cfg(test)]
mod tests {
    use super::{
        dump_table_columns, get_all_pages, get_catalog_info, get_filtered_page_data, get_page_data,
    };
    use crate::{
        accessor::entry::handle::FileHandle,
        artifacts::os::windows::ese::{
            catalog::CatalogType,
            tables::{ColumnInfo, TableInfo, get_column_flags, get_column_type},
        },
    };
    use common::windows::ColumnType;
    use std::{collections::HashMap, path::PathBuf};

    #[test]
    fn test_get_catalog_info() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\ese\\win10\\qmgr.db");
        let handle = FileHandle::host(test_location);

        let results = get_catalog_info(&handle).unwrap();
        assert_eq!(results.len(), 82);
    }

    #[test]
    fn test_get_all_pages() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\ese\\win10\\qmgr.db");
        let handle = FileHandle::host(test_location);

        let results = get_catalog_info(&handle).unwrap();

        let pages = get_all_pages(&handle, results[0].column_or_father_data_page as u32).unwrap();
        assert_eq!(pages.len(), 1);
    }

    #[test]
    fn test_dump_table_columns() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\ese\\win10\\qmgr.db");
        let handle = FileHandle::host(test_location);

        let catalog = get_catalog_info(&handle).unwrap();

        let pages = get_all_pages(&handle, catalog[0].column_or_father_data_page as u32).unwrap();
        let mut info = TableInfo {
            obj_id_table: catalog[0].obj_id_table,
            table_page: catalog[0].column_or_father_data_page,
            table_name: catalog[0].name.clone(),
            column_info: Vec::new(),
            long_value_page: 0,
        };
        // Get metadata from Catalog associated with the table we want
        for entry in &catalog {
            if entry.obj_id_table == info.obj_id_table
                && !info.table_name.is_empty()
                && entry.catalog_type == CatalogType::Column
            {
                let column_info = ColumnInfo {
                    column_type: get_column_type(entry.column_or_father_data_page),
                    column_name: entry.name.clone(),
                    column_data: Vec::new(),
                    column_id: entry.id,
                    column_flags: get_column_flags(entry.flags),
                    column_space_usage: entry.space_usage,
                    column_tagged_flags: Vec::new(),
                };

                info.column_info.push(column_info);
            } else if entry.obj_id_table == info.obj_id_table
                && !info.table_name.is_empty()
                && entry.catalog_type == CatalogType::LongValue
            {
                info.long_value_page = entry.column_or_father_data_page;
            }
        }

        let name = info.table_name.clone();
        let col_name = info.column_info[0].column_name.clone();

        let cols = dump_table_columns(&handle, &pages, &mut info, &name, &vec![col_name]).unwrap();
        assert_eq!(cols.len(), 1);
    }

    #[test]
    fn test_get_filtered_page_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\ese\\win10\\qmgr.db");
        let handle = FileHandle::host(test_location);

        let catalog = get_catalog_info(&handle).unwrap();

        let mut info = TableInfo {
            obj_id_table: 0,
            table_page: 0,
            table_name: String::new(),
            column_info: Vec::new(),
            long_value_page: 0,
        };
        // Get metadata from Catalog associated with the table we want
        for entry in &catalog {
            if entry.name == "MSysObjects" {
                info.table_name = entry.name.clone();
                info.table_page = entry.column_or_father_data_page;
                info.obj_id_table = entry.obj_id_table;
            }

            if entry.obj_id_table == info.obj_id_table
                && !info.table_name.is_empty()
                && entry.catalog_type == CatalogType::Column
            {
                let column_info = ColumnInfo {
                    column_type: get_column_type(entry.column_or_father_data_page),
                    column_name: entry.name.clone(),
                    column_data: Vec::new(),
                    column_id: entry.id,
                    column_flags: get_column_flags(entry.flags),
                    column_space_usage: entry.space_usage,
                    column_tagged_flags: Vec::new(),
                };

                info.column_info.push(column_info);
            } else if entry.obj_id_table == info.obj_id_table
                && !info.table_name.is_empty()
                && entry.catalog_type == CatalogType::LongValue
            {
                info.long_value_page = entry.column_or_father_data_page;
            }
        }
        let pages = get_all_pages(&handle, info.table_page as u32).unwrap();

        let name = info.table_name.clone();
        let mut values = HashMap::from([(String::from("JobsById"), true)]);

        let cols =
            get_filtered_page_data(&handle, &pages, &mut info, &name, "Name", &mut values).unwrap();
        assert_eq!(cols.get("MSysObjects").unwrap().len(), 1);
    }

    #[test]
    fn test_get_page_data_catalog() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\ese\\win10\\qmgr.db");
        let handle = FileHandle::host(test_location);

        let catalog = get_catalog_info(&handle).unwrap();

        let pages = get_all_pages(&handle, catalog[0].column_or_father_data_page as u32).unwrap();

        let mut info = TableInfo {
            obj_id_table: catalog[0].obj_id_table,
            table_page: catalog[0].column_or_father_data_page,
            table_name: String::new(),
            column_info: Vec::new(),
            long_value_page: 0,
        };
        // Get metadata from Catalog associated with the table we want
        for entry in &catalog {
            if entry.name != "MSysObjects" {
                continue;
            }
            if entry.obj_id_table == info.obj_id_table
                && !info.table_name.is_empty()
                && entry.catalog_type == CatalogType::Column
            {
                let column_info = ColumnInfo {
                    column_type: get_column_type(entry.column_or_father_data_page),
                    column_name: entry.name.clone(),
                    column_data: Vec::new(),
                    column_id: entry.id,
                    column_flags: get_column_flags(entry.flags),
                    column_space_usage: entry.space_usage,
                    column_tagged_flags: Vec::new(),
                };

                info.column_info.push(column_info);
            } else if entry.obj_id_table == info.obj_id_table
                && !info.table_name.is_empty()
                && entry.catalog_type == CatalogType::LongValue
            {
                info.long_value_page = entry.column_or_father_data_page;
            }
        }

        let results = get_page_data(&handle, &pages, &mut info, &catalog[0].name).unwrap();
        let catalog = results.get("MSysObjects").unwrap();
        assert_eq!(catalog.len(), 82);
    }

    #[test]
    fn test_get_page_data_bits_jobs() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\ese\\win10\\qmgr.db");
        let handle = FileHandle::host(test_location);

        let catalog = get_catalog_info(&handle).unwrap();

        let mut info = TableInfo {
            obj_id_table: 0,
            table_page: 0,
            table_name: String::new(),
            column_info: Vec::new(),
            long_value_page: 0,
        };
        // Get metadata from Catalog associated with the table we want
        for entry in &catalog {
            if entry.name == "Jobs" {
                info.table_name = entry.name.clone();
                info.table_page = entry.column_or_father_data_page;
                info.obj_id_table = entry.obj_id_table;
            }

            if entry.obj_id_table == info.obj_id_table
                && !info.table_name.is_empty()
                && entry.catalog_type == CatalogType::Column
            {
                let column_info = ColumnInfo {
                    column_type: get_column_type(entry.column_or_father_data_page),
                    column_name: entry.name.clone(),
                    column_data: Vec::new(),
                    column_id: entry.id,
                    column_flags: get_column_flags(entry.flags),
                    column_space_usage: entry.space_usage,
                    column_tagged_flags: Vec::new(),
                };

                info.column_info.push(column_info);
            } else if entry.obj_id_table == info.obj_id_table
                && !info.table_name.is_empty()
                && entry.catalog_type == CatalogType::LongValue
            {
                info.long_value_page = entry.column_or_father_data_page;
            }
        }

        let pages = get_all_pages(&handle, info.table_page as u32).unwrap();

        let name = info.table_name.clone();

        let results = get_page_data(&handle, &pages, &mut info, &name).unwrap();
        let job = results.get("Jobs").unwrap();
        assert_eq!(job[0][0].column_name, "Id");
        assert_eq!(job[0][0].column_type, ColumnType::Guid);
        assert_eq!(
            job[0][0].column_data,
            "266504ac-d974-446c-96ad-2be13a5665b0"
        );

        assert_eq!(job[0][1].column_name, "Blob");
        assert_eq!(job[0][1].column_type, ColumnType::LongBinary);
        assert_eq!(job[0][1].column_data.len(), 2740);
    }
}
