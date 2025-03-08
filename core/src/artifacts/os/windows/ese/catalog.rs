use super::{
    error::EseError,
    page::PageHeader,
    pages::leaf::{DataDefinition, LeafType},
    tags::TagFlags,
};
use crate::{
    artifacts::os::windows::ese::{
        page::PageFlags,
        pages::{branch::BranchPage, leaf::PageLeaf, root::parse_root_page},
    },
    filesystem::ntfs::reader::read_bytes,
    utils::{
        nom_helper::{
            Endian, nom_signed_eight_bytes, nom_signed_four_bytes, nom_signed_two_bytes,
            nom_unsigned_one_byte, nom_unsigned_two_bytes,
        },
        strings::extract_utf8_string,
    },
};
use log::{error, warn};
use nom::{bytes::complete::take, error::ErrorKind};
use ntfs::NtfsFile;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, io::BufReader};

#[derive(Debug, Serialize)]
pub(crate) struct Catalog {
    /**Fixed data */
    pub(crate) obj_id_table: i32,
    /**Fixed data */
    pub(crate) catalog_type: CatalogType,
    /**Fixed data */
    pub(crate) id: i32,
    /** Fixed data - Column only if the `catalog_type` is Column, otherwise father data page (FDP) */
    pub(crate) column_or_father_data_page: i32,
    /**Fixed data */
    pub(crate) space_usage: i32,
    /**Fixed data - If `catalog_type` is Column then these are columns flags */
    pub(crate) flags: i32,
    /**Fixed data */
    pub(crate) pages_or_locale: i32,
    /**Fixed data */
    pub(crate) root_flag: u8,
    /**Fixed data */
    pub(crate) record_offset: i16,
    /**Fixed data */
    pub(crate) lc_map_flags: i32,
    /**Fixed data */
    pub(crate) key_most: u16,
    /**Fixed data */
    pub(crate) lv_chunk_max: i32,
    /*Fixed data */
    pub(crate) father_data_page_last_set_time: i64,
    /**Variable data */
    pub(crate) name: String,
    /**Variable data */
    pub(crate) stats: Vec<u8>,
    /**Variable data */
    pub(crate) template_table: String,
    /**Variable data */
    pub(crate) default_value: Vec<u8>,
    /**Variable data */
    pub(crate) key_fld_ids: Vec<u8>,
    /**Variable data */
    pub(crate) var_seg_mac: Vec<u8>,
    /**Variable data */
    pub(crate) conditional_columns: Vec<u8>,
    /**Variable data */
    pub(crate) tuple_limits: Vec<u8>,
    /**Variable data */
    pub(crate) version: Vec<u8>,
    /**Variable data */
    pub(crate) sort_id: Vec<u8>,
    /**Tagged data */
    pub(crate) callback_data: Vec<u8>,
    /**Tagged data */
    pub(crate) callback_dependencies: Vec<u8>,
    /**Tagged data */
    pub(crate) separate_lv: Vec<u8>,
    /**Tagged data */
    pub(crate) space_hints: Vec<u8>,
    /**Tagged data */
    pub(crate) space_deferred_lv_hints: Vec<u8>,
    /**Tagged data */
    pub(crate) local_name: Vec<u8>,
}

pub(crate) struct VariableData {
    pub(crate) column: u8,
    pub(crate) size: u16,
}

#[derive(Debug)]
pub(crate) struct TaggedData {
    pub(crate) column: u16,
    pub(crate) offset: u16,
    pub(crate) flags: Vec<TaggedDataFlag>,
    pub(crate) data: Vec<u8>,
}

#[derive(Debug, PartialEq, Serialize)]
pub(crate) enum CatalogType {
    Table,
    Column,
    Index,
    LongValue,
    Callback,
    SlvAvail,
    SlvSpaceMap,
    Unknown,
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
pub(crate) enum TaggedDataFlag {
    Variable,
    Compressed,
    LongValue,
    MultiValue,
    MultiValueSizeDefinition,
    Unknown,
}

impl Catalog {
    /**
     * Catalog is a metadata table (called `MSysObjects` table in ESE db)  
     * It contains metadata/definitions on all columns and tables in the ESE db  
     * It even contains the metadata/definitions on itself
     * The Catalog is a static table that exists at Page 4 (5 once adjusted for ESE shadow header page)  
     *
     * Before any significant parsing of the ESE db can start, we must parse the Catalog  
     * Once parsed, we return an array of `Catalog` rows
     */
    pub(crate) fn grab_catalog<T: std::io::Seek + std::io::Read>(
        ntfs_file: Option<&NtfsFile<'_>>,
        fs: &mut BufReader<T>,
        page_size: u32,
    ) -> Result<Vec<Catalog>, EseError> {
        // Some documentation states Catalog is actually page four (4), but the first page of the ESE is a shadow copy of the header
        // ESE does not consider the shadow page a "real" page
        // So we have to add one (1)
        let catalog_page = 5;
        let catalog_start = catalog_page * page_size;

        let catalog_results = read_bytes(&(catalog_start as u64), page_size as u64, ntfs_file, fs);
        let catalog_data = match catalog_results {
            Ok(results) => results,
            Err(err) => {
                error!("[ese] Failed to read bytes for catalog: {err:?}");
                return Err(EseError::ReadFile);
            }
        };

        let catalog_result = Catalog::parse_catalog(&catalog_data, page_size, ntfs_file, fs);
        let (_, catalog) = if let Ok(result) = catalog_result {
            result
        } else {
            error!("[ese] Could not parse Catalog");
            return Err(EseError::Catalog);
        };

        Ok(catalog)
    }

    /// Parse the components of the Catalog
    fn parse_catalog<'a, T: std::io::Seek + std::io::Read>(
        catalog_data: &'a [u8],
        page_size: u32,
        ntfs_file: Option<&NtfsFile<'_>>,
        fs: &mut BufReader<T>,
    ) -> nom::IResult<&'a [u8], Vec<Catalog>> {
        let (page_data, catalog_page_data) = PageHeader::parse_header(catalog_data)?;

        let mut has_root = false;
        if catalog_page_data.page_flags.contains(&PageFlags::Root) {
            let (_, _root_page) = parse_root_page(page_data)?;
            has_root = true;
        }

        let mut catalog_rows: Vec<Catalog> = Vec::new();
        let mut key_data: Vec<u8> = Vec::new();
        let mut has_key = true;

        for tag in catalog_page_data.page_tags {
            // Defunct tags are not used
            if tag.flags.contains(&TagFlags::Defunct) {
                continue;
            }
            // First tag is Root, we already parsed that
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

            if catalog_page_data.page_flags.contains(&PageFlags::Leaf) {
                // Catalog only has one Root page and multiple Leaf pages (80+)
                let (leaf_start, _) = take(tag.offset)(page_data)?;
                let (_, leaf_data) = take(tag.value_size)(leaf_start)?;

                let leaf_result = PageLeaf::parse_leaf_page(
                    leaf_data,
                    &catalog_page_data.page_flags,
                    &key_data,
                    &tag.flags,
                );
                let (_, leaf_row) = match leaf_result {
                    Ok(result) => result,
                    Err(_err) => {
                        error!("[ese] Failed to parse leaf page for catalog");
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

            let adjust_page = 1;
            let branch_start = (branch.child_page + adjust_page) * page_size;
            // Now get the child page
            let child_result = read_bytes(&(branch_start as u64), page_size as u64, ntfs_file, fs);
            let child_data = match child_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[ese] Could not read child page data: {err:?}");
                    continue;
                }
            };

            // Track child pages so do not end up in a recursive loop (ex: child points back to parent)
            let mut page_tracker: HashMap<u32, bool> = HashMap::new();
            let rows_results = BranchPage::parse_branch_child_catalog(
                &child_data,
                &mut page_tracker,
                ntfs_file,
                fs,
            );
            let (_, mut rows) = if let Ok(results) = rows_results {
                results
            } else {
                error!("[ese] Could not parse branch child catalog");
                continue;
            };
            catalog_rows.append(&mut rows);
        }

        Ok((&[], catalog_rows))
    }

    /// Parse each row of the catalog
    pub(crate) fn parse_row(leaf_row: PageLeaf) -> Catalog {
        let mut catalog = Catalog {
            obj_id_table: 0,
            catalog_type: CatalogType::Unknown,
            id: 0,
            column_or_father_data_page: 0,
            space_usage: 0,
            flags: 0,
            pages_or_locale: 0,
            root_flag: 0,
            record_offset: 0,
            lc_map_flags: 0,
            key_most: 0,
            lv_chunk_max: 0,
            name: String::new(),
            stats: Vec::new(),
            template_table: String::new(),
            default_value: Vec::new(),
            key_fld_ids: Vec::new(),
            var_seg_mac: Vec::new(),
            conditional_columns: Vec::new(),
            tuple_limits: Vec::new(),
            version: Vec::new(),
            sort_id: Vec::new(),
            callback_data: Vec::new(),
            callback_dependencies: Vec::new(),
            separate_lv: Vec::new(),
            space_hints: Vec::new(),
            space_deferred_lv_hints: Vec::new(),
            local_name: Vec::new(),
            father_data_page_last_set_time: 0,
        };

        if leaf_row.leaf_type != LeafType::DataDefinition {
            return catalog;
        }

        // All leaf data for the catalog has Data Definition type
        // All calls to parse_row check for Data Definition type
        let leaf_data: DataDefinition = serde_json::from_value(leaf_row.leaf_data).unwrap();

        let _ = Catalog::parse_fixed(
            leaf_data.last_fixed_data,
            &leaf_data.fixed_data,
            &mut catalog,
        );
        let _ = Catalog::parse_variable(
            leaf_data.last_variable_data,
            &leaf_data.variable_data,
            &mut catalog,
        );

        catalog
    }

    /**
     * Our fixed data is basically column data with fixed sizes
     * `Last_fixed_data` represents the last column
     * Ex: `Last_fixed_data` = 8, means our `fixed_data` contains data for columns 1-8
     * Since this is a static table we already know our column names and byte sizes
     * `obj_id_table` = column one (1) (4 bytes), `catalog_type` = column two (2) (2 bytes), etc
     */
    fn parse_fixed<'a>(
        last_fixed_data: u8,
        fixed_data: &'a [u8],
        catalog: &mut Catalog,
    ) -> nom::IResult<&'a [u8], ()> {
        let mut column = 1;
        let mut data = fixed_data;
        while column <= last_fixed_data {
            match column {
                1 => {
                    let (input, obj_id_table) = nom_signed_four_bytes(data, Endian::Le)?;
                    catalog.obj_id_table = obj_id_table;
                    data = input;
                }
                2 => {
                    let (input, id) = nom_unsigned_two_bytes(data, Endian::Le)?;
                    match id {
                        1 => catalog.catalog_type = CatalogType::Table,
                        2 => catalog.catalog_type = CatalogType::Column,
                        3 => catalog.catalog_type = CatalogType::Index,
                        4 => catalog.catalog_type = CatalogType::LongValue,
                        5 => catalog.catalog_type = CatalogType::Callback,
                        6 => catalog.catalog_type = CatalogType::SlvAvail,
                        7 => catalog.catalog_type = CatalogType::SlvSpaceMap,
                        _ => catalog.catalog_type = CatalogType::Unknown,
                    }
                    data = input;
                }
                3 => {
                    let (input, id) = nom_signed_four_bytes(data, Endian::Le)?;
                    catalog.id = id;
                    data = input;
                }
                4 => {
                    let (input, column_fdp) = nom_signed_four_bytes(data, Endian::Le)?;
                    catalog.column_or_father_data_page = column_fdp;
                    data = input;
                }
                5 => {
                    let (input, space_usage) = nom_signed_four_bytes(data, Endian::Le)?;
                    catalog.space_usage = space_usage;
                    data = input;
                }
                6 => {
                    let (input, flags) = nom_signed_four_bytes(data, Endian::Le)?;
                    catalog.flags = flags;
                    data = input;
                }
                7 => {
                    let (input, pages_or_locale) = nom_signed_four_bytes(data, Endian::Le)?;
                    catalog.pages_or_locale = pages_or_locale;
                    data = input;
                }
                8 => {
                    let (input, root_flag) = nom_unsigned_one_byte(data, Endian::Le)?;
                    catalog.root_flag = root_flag;
                    data = input;
                }
                9 => {
                    let (input, record_offset) = nom_signed_two_bytes(data, Endian::Le)?;
                    catalog.record_offset = record_offset;
                    data = input;
                }
                10 => {
                    let (input, lc_map_flags) = nom_signed_four_bytes(data, Endian::Le)?;
                    catalog.lc_map_flags = lc_map_flags;
                    data = input;
                }
                11 => {
                    let (input, key_most) = nom_unsigned_two_bytes(data, Endian::Le)?;
                    catalog.key_most = key_most;
                    data = input;
                }
                12 => {
                    let (input, lv_chunk_max) = nom_signed_four_bytes(data, Endian::Le)?;
                    catalog.lv_chunk_max = lv_chunk_max;
                    data = input;
                }
                13 => {
                    let (input, father_data_page_last_set_time) =
                        nom_signed_eight_bytes(data, Endian::Le)?;
                    catalog.father_data_page_last_set_time = father_data_page_last_set_time;
                    data = input;
                }
                _ => {
                    warn!("[ese] Catalog Unknown fixed data value {column}");
                    break;
                }
            }
            column += 1;
        }

        Ok((data, ()))
    }

    /**
     * Our variable data is basically column data with variable sizes (Ex: strings)
     * `last_variable` represents the last column
     * Ex: `last_variable` = 3, means our `variable_data` may have data for columns 1-3
     * Since this is a static table we already know our column names
     */
    fn parse_variable<'a>(
        last_variable: u8,
        variable_data: &'a [u8],
        catalog: &mut Catalog,
    ) -> nom::IResult<&'a [u8], ()> {
        let mut start_column = 128;
        let mut data = variable_data;
        // The first part of the variable data is the sizes of each variable column data
        let mut var_sizes: Vec<VariableData> = Vec::new();
        while start_column <= last_variable {
            let (input, size) = nom_unsigned_two_bytes(data, Endian::Le)?;
            let var_data = VariableData {
                column: start_column,
                size,
            };
            var_sizes.push(var_data);
            data = input;
            start_column += 1;
        }

        // We now have data sizes of all column data, rest of data is actual column data

        let is_empty = 0x8000;
        let mut previous_size = 0;
        for var_data in var_sizes {
            // Check if most significant bit is set
            if (var_data.size & is_empty) > 0 {
                continue;
            }

            // We have subtract previous column sizes from current size to get an accurate size
            let size = var_data.size - previous_size;

            match var_data.column {
                128 => {
                    let (input, name_data) = take(size)(data)?;
                    catalog.name = extract_utf8_string(name_data);
                    data = input;
                }
                129 => {
                    let (input, stats_data) = take(size)(data)?;
                    catalog.stats = stats_data.to_vec();
                    data = input;
                }
                130 => {
                    let (input, template_data) = take(size)(data)?;
                    catalog.template_table = extract_utf8_string(template_data);
                    data = input;
                }
                131 => {
                    let (input, default_value_data) = take(size)(data)?;
                    catalog.default_value = default_value_data.to_vec();
                    data = input;
                }
                132 => {
                    let (input, key_fld_ids_data) = take(size)(data)?;
                    catalog.key_fld_ids = key_fld_ids_data.to_vec();
                    data = input;
                }
                133 => {
                    let (input, var_seg_mac_data) = take(size)(data)?;
                    catalog.var_seg_mac = var_seg_mac_data.to_vec();
                    data = input;
                }
                134 => {
                    let (input, conditional_columns_data) = take(size)(data)?;
                    catalog.conditional_columns = conditional_columns_data.to_vec();
                    data = input;
                }
                135 => {
                    let (input, tuple_limits_data) = take(size)(data)?;
                    catalog.tuple_limits = tuple_limits_data.to_vec();
                    data = input;
                }
                136 => {
                    let (input, version_data) = take(size)(data)?;
                    catalog.version = version_data.to_vec();
                    data = input;
                }
                137 => {
                    let (input, sort_id_data) = take(size)(data)?;
                    catalog.sort_id = sort_id_data.to_vec();
                    data = input;
                }
                _ => {
                    warn!("[ese] Unknown variable data value {}", var_data.column);
                }
            }

            previous_size = var_data.size;
        }

        if !data.is_empty() {
            Catalog::parse_tagged(data, catalog)?;
        }

        Ok((data, ()))
    }

    /**
     * Our tagged data is also column data with variable sizes (Ex: strings)
     * Any data remaining after parsing the variable columns are tagged columns
     * Since this is a static table we already know our column names
     */
    fn parse_tagged<'a>(
        tagged_data: &'a [u8],
        catalog: &mut Catalog,
    ) -> nom::IResult<&'a [u8], ()> {
        let (input, column) = nom_unsigned_two_bytes(tagged_data, Endian::Le)?;
        let (_, mut offset) = nom_unsigned_two_bytes(input, Endian::Le)?;

        /*
         * If the 0x4000 bit is set then the flags are part of the offset data
         * We also need to subtract 0x4000
         */
        let bit_flag = 0x4000;
        if offset > bit_flag {
            offset -= bit_flag;
        }

        let mut tags: Vec<TaggedData> = Vec::new();
        let tag = TaggedData {
            column,
            offset,
            flags: vec![TaggedDataFlag::Unknown],
            data: Vec::new(),
        };
        tags.push(tag);

        /*
         * tagged_data can contain one (1) or more columns, but we have no idea how many
         * So we get the first tag column, and using the size we nom to the start of tagged column data
         * We then divide the tags_meta data by four (4) which is the size of tag metadata
         * If we get one (1) we only have one tagged column and we parsed it already above
         * If we have more than one (1) then we have more tagged columns
         */
        let (mut tag_data_start, tags_meta) = take(offset)(tagged_data)?;

        let min_tag_size: u8 = 4;
        let (tags_meta, _) = take(min_tag_size)(tags_meta)?;

        // We have more tagged columns!
        if !tags_meta.is_empty() {
            Catalog::get_tags(tags_meta, &mut tags)?;
        }

        let mut full_tags: Vec<TaggedData> = Vec::new();
        let mut peek_tags = tags.iter().peekable();
        while let Some(value) = peek_tags.next() {
            // We need to subtract the current tags offset from the next tags offset to get the tag data size
            // Last tag consumes the rest of the data

            if let Some(next_value) = peek_tags.peek() {
                /*
                 * If the 0x4000 bit is set then the flags are part of the offset data
                 * We also need to subtract 0x4000
                 */
                if value.offset > bit_flag {
                    let flag = value.offset ^ bit_flag;
                    let tag_size = if next_value.offset > bit_flag {
                        (next_value.offset - bit_flag) - (value.offset - bit_flag)
                    } else {
                        next_value.offset - (value.offset - bit_flag)
                    };

                    let (input, data) = take(tag_size)(tag_data_start)?;
                    tag_data_start = input;
                    let (tag_data, _unknown_size_flag) = nom_unsigned_one_byte(data, Endian::Le)?;
                    let flags = Catalog::get_flags(&flag);

                    let tag = TaggedData {
                        column: value.column,
                        offset: value.offset,
                        flags,
                        data: tag_data.to_vec(),
                    };

                    full_tags.push(tag);
                    continue;
                }

                let tag_size = next_value.offset - value.offset;
                let (input, data) = take(tag_size)(tag_data_start)?;
                tag_data_start = input;
                let (tag_data, flag) = nom_unsigned_one_byte(data, Endian::Le)?;
                let flags = Catalog::get_flags(&flag.into());

                let tag = TaggedData {
                    column: value.column,
                    offset: value.offset,
                    flags,
                    data: tag_data.to_vec(),
                };

                full_tags.push(tag);
                continue;
            }

            /*
             * If the 0x4000 bit is set then the flags are part of the offset data
             * We also need to subtract 0x4000
             */
            if value.offset > bit_flag {
                let flag = value.offset ^ bit_flag;
                let (tag_data, _unknown_size_flag) =
                    nom_unsigned_one_byte(tag_data_start, Endian::Le)?;
                let flags = Catalog::get_flags(&flag);

                let tag = TaggedData {
                    column: value.column,
                    offset: value.offset,
                    flags,
                    data: tag_data.to_vec(),
                };

                full_tags.push(tag);
                continue;
            }

            let (tag_data, flag) = nom_unsigned_one_byte(tag_data_start, Endian::Le)?;
            let flags = Catalog::get_flags(&flag.into());

            let tag = TaggedData {
                column: value.column,
                offset: value.offset,
                flags,
                data: tag_data.to_vec(),
            };
            full_tags.push(tag);
        }

        // Nearly done, need to update catalog now
        for tag in full_tags {
            match tag.column {
                256 => catalog.callback_data = tag.data,
                257 => catalog.callback_dependencies = tag.data,
                258 => catalog.separate_lv = tag.data,
                259 => catalog.space_hints = tag.data,
                260 => catalog.space_deferred_lv_hints = tag.data,
                261 => catalog.local_name = tag.data,
                _ => {
                    warn!("[ese] Unknown tagged data value {}", tag.column);
                }
            }
        }

        Ok((tagged_data, ()))
    }

    /// Get additional tag columns
    pub(crate) fn get_tags<'a>(
        data: &'a [u8],
        tags: &mut Vec<TaggedData>,
    ) -> nom::IResult<&'a [u8], ()> {
        let mut tagged_data = data;
        while !tagged_data.is_empty() {
            let (input, column) = nom_unsigned_two_bytes(tagged_data, Endian::Le)?;
            let (input, offset) = nom_unsigned_two_bytes(input, Endian::Le)?;

            let tag = TaggedData {
                column,
                offset,
                flags: vec![TaggedDataFlag::Unknown],
                data: Vec::new(),
            };
            tags.push(tag);
            tagged_data = input;
        }

        Ok((tagged_data, ()))
    }

    /// Get flags associated with tagged columns
    pub(crate) fn get_flags(flags: &u16) -> Vec<TaggedDataFlag> {
        let variable = 1;
        let compressed = 2;
        let long_value = 4;
        let multi_value = 8;
        let multi_value_size = 16;
        let mut flags_data = Vec::new();
        if (flags & variable) == variable {
            flags_data.push(TaggedDataFlag::Variable);
        }
        if (flags & compressed) == compressed {
            flags_data.push(TaggedDataFlag::Compressed);
        }
        if (flags & long_value) == long_value {
            flags_data.push(TaggedDataFlag::LongValue);
        }
        if (flags & multi_value) == multi_value {
            flags_data.push(TaggedDataFlag::MultiValue);
        }
        if (flags & multi_value_size) == multi_value_size {
            flags_data.push(TaggedDataFlag::MultiValueSizeDefinition);
        }
        flags_data
    }
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::{Catalog, CatalogType};
    use crate::{
        artifacts::os::windows::ese::{
            catalog::TaggedDataFlag,
            header::EseHeader,
            pages::leaf::{LeafType, PageLeaf},
        },
        filesystem::ntfs::{raw_files::raw_reader, reader::read_bytes, setup::setup_ntfs_parser},
    };
    use serde_json::json;
    use std::path::PathBuf;

    #[test]
    fn test_grab_catalog() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\ese\\win10\\qmgr.db");

        let binding = test_location.display().to_string();
        let mut ntfs_parser =
            setup_ntfs_parser(&test_location.to_str().unwrap().chars().next().unwrap()).unwrap();

        let reader = raw_reader(&binding, &ntfs_parser.ntfs, &mut ntfs_parser.fs).unwrap();
        let header_bytes = read_bytes(&0, 668, Some(&reader), &mut ntfs_parser.fs).unwrap();

        let (_, header) = EseHeader::parse_header(&header_bytes).unwrap();
        let results =
            Catalog::grab_catalog(Some(&reader), &mut ntfs_parser.fs, header.page_size).unwrap();

        assert_eq!(results[0].name, "MSysObjects");
        assert_eq!(results[0].obj_id_table, 2);
        assert_eq!(results[0].catalog_type, CatalogType::Table);

        assert_eq!(results[74].name, "Blob");
        assert_eq!(results[74].obj_id_table, 8);
        assert_eq!(results[74].catalog_type, CatalogType::Column);
    }

    #[test]
    fn test_parse_fixed() {
        let test = [
            2, 0, 0, 0, 1, 0, 2, 0, 0, 0, 4, 0, 0, 0, 80, 0, 0, 0, 0, 0, 0, 192, 20, 0, 0, 0, 255,
            0,
        ];
        let mut catalog = Catalog {
            obj_id_table: 0,
            catalog_type: CatalogType::Unknown,
            id: 0,
            column_or_father_data_page: 0,
            space_usage: 0,
            flags: 0,
            pages_or_locale: 0,
            root_flag: 0,
            record_offset: 0,
            lc_map_flags: 0,
            key_most: 0,
            lv_chunk_max: 0,
            name: String::new(),
            stats: Vec::new(),
            template_table: String::new(),
            default_value: Vec::new(),
            key_fld_ids: Vec::new(),
            var_seg_mac: Vec::new(),
            conditional_columns: Vec::new(),
            tuple_limits: Vec::new(),
            version: Vec::new(),
            sort_id: Vec::new(),
            callback_data: Vec::new(),
            callback_dependencies: Vec::new(),
            separate_lv: Vec::new(),
            space_hints: Vec::new(),
            space_deferred_lv_hints: Vec::new(),
            local_name: Vec::new(),
            father_data_page_last_set_time: 0,
        };
        let fixed_col = 8;
        let (_, _) = Catalog::parse_fixed(fixed_col, &test, &mut catalog).unwrap();

        assert_eq!(catalog.obj_id_table, 2);
        assert_eq!(catalog.catalog_type, CatalogType::Table);
        assert_eq!(catalog.id, 2);
        assert_eq!(catalog.column_or_father_data_page, 4);
        assert_eq!(catalog.space_usage, 80);
        assert_eq!(catalog.flags, -1073741824);
        assert_eq!(catalog.root_flag, 255);
        assert_eq!(catalog.pages_or_locale, 20);
    }

    #[test]
    fn test_parse_variable() {
        let test = [11, 0, 77, 83, 121, 115, 79, 98, 106, 101, 99, 116, 115];
        let mut catalog = Catalog {
            obj_id_table: 0,
            catalog_type: CatalogType::Unknown,
            id: 0,
            column_or_father_data_page: 0,
            space_usage: 0,
            flags: 0,
            pages_or_locale: 0,
            root_flag: 0,
            record_offset: 0,
            lc_map_flags: 0,
            key_most: 0,
            lv_chunk_max: 0,
            name: String::new(),
            stats: Vec::new(),
            template_table: String::new(),
            default_value: Vec::new(),
            key_fld_ids: Vec::new(),
            var_seg_mac: Vec::new(),
            conditional_columns: Vec::new(),
            tuple_limits: Vec::new(),
            version: Vec::new(),
            sort_id: Vec::new(),
            callback_data: Vec::new(),
            callback_dependencies: Vec::new(),
            separate_lv: Vec::new(),
            space_hints: Vec::new(),
            space_deferred_lv_hints: Vec::new(),
            local_name: Vec::new(),
            father_data_page_last_set_time: 0,
        };
        let variable_column = 128;
        let (_, _) = Catalog::parse_variable(variable_column, &test, &mut catalog).unwrap();

        assert_eq!(catalog.name, "MSysObjects");
    }

    #[test]
    fn test_parse_tagged() {
        let test = [5, 1, 4, 0, 1, 101, 0, 110, 0, 45, 0, 85, 0, 83];
        let mut catalog = Catalog {
            obj_id_table: 0,
            catalog_type: CatalogType::Unknown,
            id: 0,
            column_or_father_data_page: 0,
            space_usage: 0,
            flags: 0,
            pages_or_locale: 0,
            root_flag: 0,
            record_offset: 0,
            lc_map_flags: 0,
            key_most: 0,
            lv_chunk_max: 0,
            name: String::new(),
            stats: Vec::new(),
            template_table: String::new(),
            default_value: Vec::new(),
            key_fld_ids: Vec::new(),
            var_seg_mac: Vec::new(),
            conditional_columns: Vec::new(),
            tuple_limits: Vec::new(),
            version: Vec::new(),
            sort_id: Vec::new(),
            callback_data: Vec::new(),
            callback_dependencies: Vec::new(),
            separate_lv: Vec::new(),
            space_hints: Vec::new(),
            space_deferred_lv_hints: Vec::new(),
            local_name: Vec::new(),
            father_data_page_last_set_time: 0,
        };
        let (_, _) = Catalog::parse_tagged(&test, &mut catalog).unwrap();

        assert_eq!(catalog.local_name, [101, 0, 110, 0, 45, 0, 85, 0, 83]);
    }

    #[test]
    fn test_parse_row() {
        let leaf = PageLeaf {
            _common_page_key_size: 0,
            _local_page_key_size: 17,
            key_suffix: vec![
                127, 43, 225, 58, 86, 101, 176, 150, 173, 108, 68, 116, 217, 172, 4, 101, 38,
            ],
            key_prefix: Vec::new(),
            leaf_type: LeafType::DataDefinition,
            leaf_data: json!({"last_fixed_data":1, "last_variable_data":127, "variable_data_offset":21, "fixed_data": vec![1,0,0,0,116,217,108,68,150,173,43,225,58,86,101,176,254], "variable_data": vec![0,1,4,0,5,12,0,0,128,0,0,0,128]}),
        };
        let result = Catalog::parse_row(leaf);
        assert_eq!(result.obj_id_table, 1);
    }

    #[test]
    fn test_get_tags() {
        let test = [5, 1, 4, 0, 1, 101, 0, 110, 0, 45, 0, 85, 0, 83, 0, 0];
        let mut tags = Vec::new();
        let (_, _) = Catalog::get_tags(&test, &mut tags).unwrap();

        assert_eq!(tags[0].column, 261);
        assert_eq!(tags[0].offset, 4);
    }

    #[test]
    fn test_get_flags() {
        let flag = 1;
        let flags = Catalog::get_flags(&flag);
        assert_eq!(flags, vec![TaggedDataFlag::Variable]);
    }

    #[test]
    fn test_srum_catalog() {
        let mut ntfs_parser = setup_ntfs_parser(&'C').unwrap();

        let reader = raw_reader(
            "C:\\Windows\\System32\\sru\\SRUDB.dat",
            &ntfs_parser.ntfs,
            &mut ntfs_parser.fs,
        )
        .unwrap();
        let header_bytes = read_bytes(&0, 668, Some(&reader), &mut ntfs_parser.fs).unwrap();

        let (_, header) = EseHeader::parse_header(&header_bytes).unwrap();
        let results =
            Catalog::grab_catalog(Some(&reader), &mut ntfs_parser.fs, header.page_size).unwrap();
        assert!(results.len() > 100);
    }

    #[test]
    fn test_updates_catalog() {
        let mut ntfs_parser = setup_ntfs_parser(&'C').unwrap();

        let reader = raw_reader(
            "C:\\Windows\\SoftwareDistribution\\DataStore\\DataStore.edb",
            &ntfs_parser.ntfs,
            &mut ntfs_parser.fs,
        )
        .unwrap();
        let header_bytes = read_bytes(&0, 668, Some(&reader), &mut ntfs_parser.fs).unwrap();
        if header_bytes.is_empty() {
            return;
        }
        let (_, header) = EseHeader::parse_header(&header_bytes).unwrap();
        let results =
            Catalog::grab_catalog(Some(&reader), &mut ntfs_parser.fs, header.page_size).unwrap();

        assert!(results.len() > 10);
    }
}
