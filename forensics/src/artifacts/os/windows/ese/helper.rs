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
    artifacts::os::{
        systeminfo::info::get_platform,
        windows::ese::{
            pages::{
                branch::BranchPage,
                leaf::{LeafType, PageLeaf},
            },
            tables::{clear_column_data, parse_row},
            tags::TagFlags,
        },
    },
    filesystem::{
        files::file_reader,
        ntfs::{
            raw_files::raw_reader, reader::read_bytes, sector_reader::SectorReader,
            setup::setup_ntfs_parser,
        },
    },
    utils::nom_helper::nom_data,
};
use common::windows::{ColumnType, TableDump};
use log::error;
use ntfs::{Ntfs, NtfsFile};
use std::{collections::HashMap, fs::File, io::BufReader};

/// Get `Catalog` data from provided ESE path
pub(crate) fn get_catalog_info(path: &str) -> Result<Vec<Catalog>, EseError> {
    let plat = get_platform();

    // On non-Windows platforms use a normal BufReader
    let catalog = if plat != "Windows" {
        let reader = setup_ese_reader(path)?;
        let mut buf_reader = BufReader::new(reader);

        let page_size = ese_page_size(None, &mut buf_reader)?;
        Catalog::grab_catalog(None, &mut buf_reader, page_size)?
    } else {
        // On Windows use a NTFS reader
        let ntfs_parser_result = setup_ntfs_parser(&path.chars().next().unwrap_or('C'));
        let mut ntfs_parser = match ntfs_parser_result {
            Ok(result) => result,
            Err(err) => {
                error!("[ese] Could not setup NTFS parser: {err:?}");
                return Err(EseError::ParseEse);
            }
        };
        let ntfs_file = setup_ese_reader_windows(&ntfs_parser.ntfs, &mut ntfs_parser.fs, path)?;

        let page_size = ese_page_size(Some(&ntfs_file), &mut ntfs_parser.fs)?;
        Catalog::grab_catalog(Some(&ntfs_file), &mut ntfs_parser.fs, page_size)?
    };

    Ok(catalog)
}

/// Get all pages from ESE table. First page can be found from the `Catalog`
pub(crate) fn get_all_pages(path: &str, first_page: u32) -> Result<Vec<u32>, EseError> {
    let plat = get_platform();

    let pages = if plat != "Windows" {
        let reader = setup_ese_reader(path)?;
        let mut buf_reader = BufReader::new(reader);

        let page_size = ese_page_size(None, &mut buf_reader)?;
        get_pages(first_page, None, &mut buf_reader, page_size)?
    } else {
        let mut ntfs_parser = setup_ntfs_parser(&path.chars().next().unwrap_or('C')).unwrap();
        let ntfs_file = setup_ese_reader_windows(&ntfs_parser.ntfs, &mut ntfs_parser.fs, path)?;
        let page_size = ese_page_size(Some(&ntfs_file), &mut ntfs_parser.fs)?;
        get_pages(first_page, Some(&ntfs_file), &mut ntfs_parser.fs, page_size)?
    };

    Ok(pages)
}

/// Get all page data (rows) from table based on array of pages
pub(crate) fn get_page_data(
    path: &str,
    pages: &[u32],
    info: &mut TableInfo,
    name: &str,
) -> Result<HashMap<String, Vec<Vec<TableDump>>>, EseError> {
    let plat = get_platform();
    let mut total_rows = HashMap::new();
    total_rows.insert(name.to_string(), Vec::new());

    let page_size;
    let last_page = 0;
    let mut rows = if plat != "Windows" {
        let reader = setup_ese_reader(path)?;
        let mut buf_reader = BufReader::new(reader);

        page_size = ese_page_size(None, &mut buf_reader)?;
        let mut rows = Vec::new();
        for page in pages {
            if page == &last_page {
                continue;
            }
            let mut page_rows = page_data(*page, None, &mut buf_reader, page_size, info)?;
            rows.append(&mut page_rows);
        }
        row_data(&mut rows, None, &mut buf_reader, page_size, info, name)?
    } else {
        let mut ntfs_parser = setup_ntfs_parser(&path.chars().next().unwrap_or('C')).unwrap();
        let ntfs_file = setup_ese_reader_windows(&ntfs_parser.ntfs, &mut ntfs_parser.fs, path)?;

        page_size = ese_page_size(Some(&ntfs_file), &mut ntfs_parser.fs)?;
        let mut rows = Vec::new();

        for page in pages {
            if page == &last_page {
                continue;
            }
            let mut page_rows = page_data(
                *page,
                Some(&ntfs_file),
                &mut ntfs_parser.fs,
                page_size,
                info,
            )?;

            rows.append(&mut page_rows);
        }
        row_data(
            &mut rows,
            Some(&ntfs_file),
            &mut ntfs_parser.fs,
            page_size,
            info,
            name,
        )?
    };

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
    path: &str,
    pages: &[u32],
    info: &mut TableInfo,
    name: &str,
    column_name: &str,
    column_values: &mut HashMap<String, bool>,
) -> Result<HashMap<String, Vec<Vec<TableDump>>>, EseError> {
    let plat = get_platform();
    let mut total_rows = HashMap::new();
    total_rows.insert(name.to_string(), Vec::new());

    let page_size;
    let rows = if plat != "Windows" {
        let reader = setup_ese_reader(path)?;
        let mut buf_reader = BufReader::new(reader);

        page_size = ese_page_size(None, &mut buf_reader)?;
        let mut rows = Vec::new();
        for page in pages {
            let mut page_rows = page_data(*page, None, &mut buf_reader, page_size, info)?;
            rows.append(&mut page_rows);
        }
        row_data(&mut rows, None, &mut buf_reader, page_size, info, name)?
    } else {
        // On Windows use a NTFS reader
        let mut ntfs_parser = setup_ntfs_parser(&path.chars().next().unwrap_or('C')).unwrap();
        let ntfs_file = setup_ese_reader_windows(&ntfs_parser.ntfs, &mut ntfs_parser.fs, path)?;

        page_size = ese_page_size(Some(&ntfs_file), &mut ntfs_parser.fs)?;
        let mut rows = Vec::new();

        for page in pages {
            let mut page_rows = page_data(
                *page,
                Some(&ntfs_file),
                &mut ntfs_parser.fs,
                page_size,
                info,
            )?;

            rows.append(&mut page_rows);
        }
        row_data(
            &mut rows,
            Some(&ntfs_file),
            &mut ntfs_parser.fs,
            page_size,
            info,
            name,
        )?
    };

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
    path: &str,
    pages: &[u32],
    info: &mut TableInfo,
    name: &str,
    column_names: &[String],
) -> Result<HashMap<String, Vec<Vec<TableDump>>>, EseError> {
    let plat = get_platform();
    let mut total_rows = HashMap::new();
    total_rows.insert(name.to_string(), Vec::new());

    let page_size;
    let rows = if plat != "Windows" {
        let reader = setup_ese_reader(path)?;
        let mut buf_reader = BufReader::new(reader);

        page_size = ese_page_size(None, &mut buf_reader)?;
        let mut rows = Vec::new();
        for page in pages {
            let mut page_rows = page_data(*page, None, &mut buf_reader, page_size, info)?;
            rows.append(&mut page_rows);
        }
        row_data(&mut rows, None, &mut buf_reader, page_size, info, name)?
    } else {
        let mut ntfs_parser = setup_ntfs_parser(&path.chars().next().unwrap_or('C')).unwrap();
        let ntfs_file = setup_ese_reader_windows(&ntfs_parser.ntfs, &mut ntfs_parser.fs, path)?;

        page_size = ese_page_size(Some(&ntfs_file), &mut ntfs_parser.fs)?;
        let mut rows = Vec::new();

        for page in pages {
            let mut page_rows = page_data(
                *page,
                Some(&ntfs_file),
                &mut ntfs_parser.fs,
                page_size,
                info,
            )?;

            rows.append(&mut page_rows);
        }
        row_data(
            &mut rows,
            Some(&ntfs_file),
            &mut ntfs_parser.fs,
            page_size,
            info,
            name,
        )?
    };

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

/// Setup Windows ESE reader using NTFS parser
fn setup_ese_reader_windows<'a>(
    ntfs_file: &'a Ntfs,
    fs: &mut BufReader<SectorReader<File>>,
    path: &str,
) -> Result<NtfsFile<'a>, EseError> {
    let reader_result = raw_reader(path, ntfs_file, fs);
    let ntfs_file = match reader_result {
        Ok(result) => result,
        Err(err) => {
            error!("[ese] Could not setup reader: {err:?}");
            return Err(EseError::ReadFile);
        }
    };

    Ok(ntfs_file)
}

/// Setup ESE using normal reader
fn setup_ese_reader(path: &str) -> Result<File, EseError> {
    let reader_result = file_reader(path);
    let reader = match reader_result {
        Ok(reader) => reader,
        Err(err) => {
            error!("[ese] Could not setup API reader: {err:?}");
            return Err(EseError::ReadFile);
        }
    };

    Ok(reader)
}

/// Determine page size for ESE database
fn ese_page_size<T: std::io::Seek + std::io::Read>(
    ntfs_file: Option<&NtfsFile<'_>>,
    fs: &mut BufReader<T>,
) -> Result<u32, EseError> {
    let header_size = 668;
    let offset = 0;

    let header_result = read_bytes(&offset, header_size, ntfs_file, fs);
    let header_data = match header_result {
        Ok(result) => result,
        Err(err) => {
            error!("[ese] Failed to reader header bytes: {err:?}");
            return Err(EseError::ParseEse);
        }
    };

    let db_result = EseHeader::parse_header(&header_data);
    let (_, db_header) = match db_result {
        Ok(result) => result,
        Err(_err) => {
            error!("[ese] Failed to parse ESE header");
            return Err(EseError::ParseEse);
        }
    };

    Ok(db_header.page_size)
}

/// Get array of pages
fn get_pages<T: std::io::Seek + std::io::Read>(
    first_page: u32,
    ntfs_file: Option<&NtfsFile<'_>>,
    fs: &mut BufReader<T>,
    page_size: u32,
) -> Result<Vec<u32>, EseError> {
    // Need to adjust page number to account for header page
    let adjust_page = 1;
    let page_number = (first_page + adjust_page) * page_size;

    let start_result = read_bytes(&(page_number as u64), page_size as u64, ntfs_file, fs);
    let page_start = match start_result {
        Ok(result) => result,
        Err(err) => {
            error!("[ese] Failed to read bytes for page start: {err:?}");
            return Err(EseError::ParseEse);
        }
    };

    // Start parsing the page associated with the table data
    let page_header_result = PageHeader::parse_header(&page_start);
    let (page_data, table_page_data) = match page_header_result {
        Ok(result) => result,
        Err(_err) => {
            error!("[ese] Failed to parse ESE header");
            return Err(EseError::ParseEse);
        }
    };

    let mut has_root = false;
    if table_page_data.page_flags.contains(&PageFlags::Root) {
        let root_page_result = parse_root_page(page_data);
        if root_page_result.is_err() {
            error!("[ese] Failed to parse root page. Stopping parsing");
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
                error!("[ese] Failed to get branch start data");
                return Err(EseError::ParseEse);
            }
        };
        let branch_result = nom_data(branch_start, tag.value_size.into());
        let (_, branch_data) = match branch_result {
            Ok(result) => result,
            Err(_err) => {
                error!("[ese] Failed to get branch data");
                return Err(EseError::ParseEse);
            }
        };
        let branch_result = BranchPage::parse_branch_page(branch_data, &tag.flags);
        let (_, branch) = match branch_result {
            Ok(result) => result,
            Err(_err) => {
                error!("[ese] Failed to get branch page data");
                return Err(EseError::ParseEse);
            }
        };

        let adjust_page = 1;
        let branch_start = (branch.child_page + adjust_page) * page_size;
        pages.push(branch.child_page);

        // Now get the child page
        let child_result = read_bytes(&(branch_start as u64), page_size as u64, ntfs_file, fs);
        let child_data = match child_result {
            Ok(result) => result,
            Err(err) => {
                error!("[ese] Failed to read bytes for child data: {err:?}");
                return Err(EseError::ParseEse);
            }
        };

        // Track child pages so do not end up in a recursive loop (ex: child points back to parent)
        let mut page_tracker: HashMap<u32, bool> = HashMap::new();
        let last_result = BranchPage::parse_branch_child_page(
            &child_data,
            &mut pages,
            &mut page_tracker,
            ntfs_file,
            fs,
        );
        if last_result.is_err() {
            error!("[ese] Could not parse branch child table and last page in page tags");
            return Err(EseError::ParseEse);
        }
    }

    Ok(pages)
}

/// Start parsing the page data to get rows
fn page_data<T: std::io::Seek + std::io::Read>(
    page: u32,
    ntfs_file: Option<&NtfsFile<'_>>,
    fs: &mut BufReader<T>,
    page_size: u32,
    info: &mut TableInfo,
) -> Result<Vec<Vec<ColumnInfo>>, EseError> {
    // Need to adjust page number to account for header page
    let adjust_page = 1;
    let page_number = (page + adjust_page) * page_size;

    let start_result = read_bytes(&(page_number as u64), page_size as u64, ntfs_file, fs);
    let page_start = match start_result {
        Ok(result) => result,
        Err(err) => {
            error!("[ese] Failed to read bytes for page start: {err:?}");
            return Err(EseError::ParseEse);
        }
    };

    // Start parsing the page associated with the table data
    let page_header_result = PageHeader::parse_header(&page_start);
    let (page_data, table_page_data) = match page_header_result {
        Ok(result) => result,
        Err(_err) => {
            error!("[ese] Failed to parse ESE header");
            return Err(EseError::ParseEse);
        }
    };

    let mut has_root = false;
    if table_page_data.page_flags.contains(&PageFlags::Root) {
        let root_page_result = parse_root_page(page_data);
        if root_page_result.is_err() {
            error!("[ese] Failed to parse root page. Stopping parsing");
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
                    error!("[ese] Failed to get key data");
                    return Err(EseError::ParseEse);
                }
            };
            let page_key_data_result = nom_data(key_start, tag.value_size.into());
            let (_, page_key_data) = match page_key_data_result {
                Ok(result) => result,
                Err(_err) => {
                    error!("[ese] Failed to get page key data");
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
                error!("[ese] Failed to get leaf data");
                return Err(EseError::ParseEse);
            }
        };
        let leaf_result = nom_data(leaf_start, tag.value_size.into());
        let (_, leaf_data) = match leaf_result {
            Ok(result) => result,
            Err(_err) => {
                error!("[ese] Failed to get leaf data");
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
                error!("[ese] Failed to parse leaf page {page}");
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
fn row_data<T: std::io::Seek + std::io::Read>(
    rows: &mut Vec<Vec<ColumnInfo>>,
    ntfs_file: Option<&NtfsFile<'_>>,
    fs: &mut BufReader<T>,
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

    let page_result = read_bytes(&(page_number as u64), page_size as u64, ntfs_file, fs);
    let page_start = match page_result {
        Ok(result) => result,
        Err(err) => {
            error!("[ese] Failed to read bytes for child data: {err:?}");
            return Err(EseError::ParseEse);
        }
    };

    let long_result = parse_long_value(&page_start, ntfs_file, fs);
    let (_, long_values) = match long_result {
        Ok(result) => result,
        Err(_err) => {
            error!("[ese] Could not get long value data");
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
#[cfg(target_os = "windows")]
mod tests {
    use super::{
        dump_table_columns, get_all_pages, get_catalog_info, get_filtered_page_data, get_page_data,
    };
    use crate::artifacts::os::windows::ese::{
        catalog::CatalogType,
        tables::{ColumnInfo, TableInfo, get_column_flags, get_column_type},
    };
    use common::windows::ColumnType;
    use std::{collections::HashMap, path::PathBuf};

    #[test]
    fn test_get_catalog_info() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\ese\\win10\\qmgr.db");

        let results = get_catalog_info(test_location.to_str().unwrap()).unwrap();
        assert_eq!(results.len(), 82);
    }

    #[test]
    fn test_get_all_pages() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\ese\\win10\\qmgr.db");

        let results = get_catalog_info(test_location.to_str().unwrap()).unwrap();

        let pages = get_all_pages(
            test_location.to_str().unwrap(),
            results[0].column_or_father_data_page as u32,
        )
        .unwrap();
        assert_eq!(pages.len(), 1);
    }

    #[test]
    fn test_dump_table_columns() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\ese\\win10\\qmgr.db");

        let catalog = get_catalog_info(test_location.to_str().unwrap()).unwrap();

        let pages = get_all_pages(
            test_location.to_str().unwrap(),
            catalog[0].column_or_father_data_page as u32,
        )
        .unwrap();
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
                    column_type: get_column_type(&entry.column_or_father_data_page),
                    column_name: entry.name.clone(),
                    column_data: Vec::new(),
                    column_id: entry.id,
                    column_flags: get_column_flags(&entry.flags),
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

        let cols = dump_table_columns(
            test_location.to_str().unwrap(),
            &pages,
            &mut info,
            &name,
            &vec![col_name],
        )
        .unwrap();
        assert_eq!(cols.len(), 1);
    }

    #[test]
    fn test_get_filtered_page_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\ese\\win10\\qmgr.db");

        let catalog = get_catalog_info(test_location.to_str().unwrap()).unwrap();

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
                    column_type: get_column_type(&entry.column_or_father_data_page),
                    column_name: entry.name.clone(),
                    column_data: Vec::new(),
                    column_id: entry.id,
                    column_flags: get_column_flags(&entry.flags),
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
        let pages = get_all_pages(test_location.to_str().unwrap(), info.table_page as u32).unwrap();

        let name = info.table_name.clone();
        let mut values = HashMap::from([(String::from("JobsById"), true)]);

        let cols = get_filtered_page_data(
            test_location.to_str().unwrap(),
            &pages,
            &mut info,
            &name,
            "Name",
            &mut values,
        )
        .unwrap();
        assert_eq!(cols.get("MSysObjects").unwrap().len(), 1);
    }

    #[test]
    fn test_get_page_data_catalog() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\ese\\win10\\qmgr.db");

        let catalog = get_catalog_info(test_location.to_str().unwrap()).unwrap();

        let pages = get_all_pages(
            test_location.to_str().unwrap(),
            catalog[0].column_or_father_data_page as u32,
        )
        .unwrap();

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
                    column_type: get_column_type(&entry.column_or_father_data_page),
                    column_name: entry.name.clone(),
                    column_data: Vec::new(),
                    column_id: entry.id,
                    column_flags: get_column_flags(&entry.flags),
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

        let results = get_page_data(
            test_location.to_str().unwrap(),
            &pages,
            &mut info,
            &catalog[0].name,
        )
        .unwrap();
        let catalog = results.get("MSysObjects").unwrap();
        assert_eq!(catalog.len(), 82);
    }

    #[test]
    fn test_get_page_data_bits_jobs() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\ese\\win10\\qmgr.db");

        let catalog = get_catalog_info(test_location.to_str().unwrap()).unwrap();

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
                    column_type: get_column_type(&entry.column_or_father_data_page),
                    column_name: entry.name.clone(),
                    column_data: Vec::new(),
                    column_id: entry.id,
                    column_flags: get_column_flags(&entry.flags),
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

        let pages = get_all_pages(test_location.to_str().unwrap(), info.table_page as u32).unwrap();

        let name = info.table_name.clone();

        let results =
            get_page_data(test_location.to_str().unwrap(), &pages, &mut info, &name).unwrap();
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
