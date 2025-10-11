use super::{
    catalog::{Catalog, CatalogType, VariableData},
    pages::leaf::{DataDefinition, LeafType},
};
use crate::{
    artifacts::os::windows::ese::{
        catalog::{TaggedData, TaggedDataFlag},
        pages::leaf::PageLeaf,
    },
    utils::{
        compression::decompress::{XpressType, decompress_seven_bit, decompress_xpress},
        encoding::base64_encode_standard,
        nom_helper::{
            Endian, nom_signed_eight_bytes, nom_signed_four_bytes, nom_signed_two_bytes,
            nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_one_byte,
            nom_unsigned_two_bytes,
        },
        strings::extract_ascii_utf16_string,
        time::{filetime_to_unixepoch, ole_automationtime_to_unixepoch, unixepoch_to_iso},
        uuid::format_guid_le_bytes,
    },
};
use common::windows::{ColumnType, TableDump};
use log::{error, warn};
use nom::{
    bytes::complete::take,
    number::complete::{le_f32, le_f64},
};
use serde::Deserialize;
use std::{collections::HashMap, mem::size_of};

#[derive(Debug, Deserialize)]
pub(crate) struct TableInfo {
    pub(crate) obj_id_table: i32,
    pub(crate) table_page: i32,
    pub(crate) table_name: String,
    pub(crate) column_info: Vec<ColumnInfo>,
    pub(crate) long_value_page: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ColumnInfo {
    pub(crate) column_type: ColumnType,
    pub(crate) column_name: String,
    pub(crate) column_data: Vec<u8>,
    pub(crate) column_id: i32,
    pub(crate) column_flags: Vec<ColumnFlags>,
    pub(crate) column_space_usage: i32,
    /**Flags associated with tagged. Lets us know if the the data is compressed */
    pub(crate) column_tagged_flags: Vec<TaggedDataFlag>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub(crate) enum ColumnFlags {
    NotNull,
    Version,
    AutoIncrement,
    MultiValued,
    Default,
    EscrowUpdate,
    Finalize,
    UserDefinedDefault,
    TemplateColumnESE98,
    DeleteOnZero,
    PrimaryIndexPlaceholder,
    Compressed,
    Encrypted,
    Versioned,
    Deleted,
    VersionedAdd,
}

/// Get table info from provided catalog
pub(crate) fn table_info(catalog: &[Catalog], name: &str) -> TableInfo {
    let mut info = TableInfo {
        obj_id_table: 0,
        table_page: 0,
        table_name: String::new(),
        column_info: Vec::new(),
        long_value_page: 0,
    };
    // Get metadata from Catalog associated with the table we want
    for entry in catalog {
        if entry.name == name {
            info.table_name.clone_from(&entry.name);
            info.obj_id_table = entry.obj_id_table;
            info.table_page = entry.column_or_father_data_page;
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
    info
}

/// Create hashmap that represents our table data
pub(crate) fn create_table_data(
    column_rows: &[Vec<ColumnInfo>],
    table_name: &str,
) -> HashMap<String, Vec<Vec<TableDump>>> {
    let mut table_data: HashMap<String, Vec<Vec<TableDump>>> = HashMap::new();

    let mut rows: Vec<Vec<TableDump>> = Vec::new();
    for column_row in column_rows {
        let mut table_dump: Vec<TableDump> = Vec::new();
        for column in column_row {
            let mut dump = TableDump {
                column_type: column.column_type.clone(),
                column_name: column.column_name.clone(),
                column_data: String::new(),
            };

            if column.column_data.is_empty() {
                dump.column_data = String::new();
                continue;
            }

            let result = column_data_to_string(
                &dump.column_type,
                &column.column_data,
                &column.column_flags,
                &column.column_tagged_flags,
            );
            if let Ok((_, value)) = result {
                dump.column_data = value;
            } else {
                warn!(
                    "[ese] Could not transform column {} data to string for table: {table_name}",
                    column.column_name
                );
                dump.column_data = base64_encode_standard(&column.column_data);
            }
            table_dump.push(dump);
        }
        rows.push(table_dump);
    }
    table_data.insert(table_name.to_string(), rows);

    table_data
}

/// Parse the column data based flag type
fn column_data_to_string<'a>(
    column_type: &ColumnType,
    data: &'a [u8],
    flags: &[ColumnFlags],
    tagged_flags: &[TaggedDataFlag],
) -> nom::IResult<&'a [u8], String> {
    // MultiValues have multiple values (obviously). So we loop through a get them all
    if tagged_flags.contains(&TaggedDataFlag::MultiValue)
        && !tagged_flags.contains(&TaggedDataFlag::LongValue)
    {
        let (_, multi_data) = parse_multivalue_data(data, tagged_flags)?;
        let mut multi_string = String::new();
        for multi in multi_data {
            let result = extract_column_data_to_string(column_type, &multi, flags, tagged_flags);
            let multi_value = match result {
                Ok((_, result)) => result,
                Err(_err) => {
                    error!("[ese] Failed to extract multivalue data");
                    base64_encode_standard(&multi)
                }
            };
            // Right now multivalues are comma separated
            multi_string = format!("{multi_string},{multi_value}");
        }
        return Ok((&[], multi_string));
    }
    extract_column_data_to_string(column_type, data, flags, tagged_flags)
}

/// Based on `ColumnType` extract the data into a string
fn extract_column_data_to_string<'a>(
    column_type: &ColumnType,
    data: &'a [u8],
    flags: &[ColumnFlags],
    tagged_flags: &[TaggedDataFlag],
) -> nom::IResult<&'a [u8], String> {
    let (input, value) = match column_type {
        ColumnType::Nil => (data, String::new()),
        ColumnType::Bit | ColumnType::UnsignedByte => {
            let (input, value) = nom_unsigned_one_byte(data, Endian::Le)?;
            (input, format!("{value}"))
        }
        ColumnType::Short => {
            let (input, value) = nom_signed_two_bytes(data, Endian::Le)?;
            (input, format!("{value}"))
        }
        ColumnType::Long => {
            let (input, value) = nom_signed_four_bytes(data, Endian::Le)?;
            (input, format!("{value}"))
        }
        ColumnType::Currency | ColumnType::LongLong => {
            let (input, value) = nom_signed_eight_bytes(data, Endian::Le)?;
            (input, format!("{value}"))
        }
        ColumnType::Float32 => {
            let (input, float_data) = take(size_of::<u32>())(data)?;
            let (_, value) = le_f32(float_data)?;
            (input, format!("{value}"))
        }
        ColumnType::Float64 => {
            let (input, float_data) = take(size_of::<u64>())(data)?;
            let (_, value) = le_f64(float_data)?;
            (input, format!("{value}"))
        }
        ColumnType::DateTime => {
            // DateTime value can either be a FILETIME or OLETIME/VARIANTTIME
            // https://github.com/libyal/libesedb/blob/main/documentation/Extensible%20Storage%20Engine%20(ESE)%20Database%20File%20(EDB)%20format.asciidoc#61-column-type
            // https://github.com/Velocidex/go-ese/blob/master/parser/catalog.go#L165
            // https://github.com/strozfriedberg/ese_parser/blob/main/lib/src/vartime.rs#L150

            // Though official Microsoft docs state only VARIANTTIME is returned
            // https://learn.microsoft.com/en-us/windows/win32/extensible-storage-engine/jet-coltyp

            // Appears if flags contain NotNull, then the time is FILETIME
            if flags.contains(&ColumnFlags::NotNull) {
                let (input, filetime_data) = nom_unsigned_eight_bytes(data, Endian::Le)?;
                let filetime = filetime_to_unixepoch(filetime_data);

                (input, unixepoch_to_iso(filetime))
            } else {
                let (input, float_data) = take(size_of::<u64>())(data)?;
                let (_, float_value) = le_f64(float_data)?;
                let oletime = ole_automationtime_to_unixepoch(float_value);

                (input, unixepoch_to_iso(oletime))
            }
        }
        ColumnType::LongBinary | ColumnType::Binary => {
            let value = if (tagged_flags.contains(&TaggedDataFlag::Compressed)
                && !tagged_flags.contains(&TaggedDataFlag::MultiValue))
                || flags.contains(&ColumnFlags::Compressed)
            {
                let (_, decompressed_data) = get_decompressed_data(data)?;
                base64_encode_standard(&decompressed_data)
            } else {
                base64_encode_standard(data)
            };
            (data, value)
        }
        ColumnType::LongText | ColumnType::Text => {
            let value = if (tagged_flags.contains(&TaggedDataFlag::Compressed)
                && !tagged_flags.contains(&TaggedDataFlag::MultiValue))
                || flags.contains(&ColumnFlags::Compressed)
            {
                let (_, decompressed_data) = get_decompressed_data(data)?;
                extract_ascii_utf16_string(&decompressed_data)
            } else {
                extract_ascii_utf16_string(data)
            };
            (data, value)
        }
        ColumnType::SuperLong => {
            warn!("[ese] Super long column type is obsolete");
            let value = base64_encode_standard(data);
            (data, value)
        }
        ColumnType::UnsignedLong => {
            let (input, value) = nom_unsigned_four_bytes(data, Endian::Le)?;
            (input, format!("{value}"))
        }
        ColumnType::Guid => {
            let value = format_guid_le_bytes(data);
            (data, value)
        }
        ColumnType::UnsignedShort => {
            let (input, value) = nom_unsigned_two_bytes(data, Endian::Le)?;
            (input, format!("{value}"))
        }
        ColumnType::Unknown => {
            warn!("[ese] Got unknown column type");
            let value = base64_encode_standard(data);
            (data, value)
        }
    };

    Ok((input, value))
}

/// Decompress ESE data based on compression type
fn get_decompressed_data(data: &[u8]) -> nom::IResult<&[u8], Vec<u8>> {
    if data.is_empty() {
        return Ok((data, data.to_vec()));
    }

    let bit_check = 3;
    let check = data[0] >> bit_check;
    // If the first shifted bytes is not a 1, 2, or 3. Then data is not actually compressed (even though the flag is set)
    if check != 1 && check != 2 && check != 3 {
        return Ok((data, data.to_vec()));
    }

    let (input, compression_type) = nom_unsigned_one_byte(data, Endian::Le)?;
    let huffman = 0x18;
    let decompressed_data = if compression_type == huffman {
        let (input, decompress_size) = nom_unsigned_two_bytes(input, Endian::Le)?;
        decompress_ese(&mut input.to_owned(), decompress_size as u32)
    } else {
        // Any other value means seven bit compression
        decompress_seven_bit(input)
    };

    Ok((input, decompressed_data))
}

#[cfg(target_os = "windows")]
/// Decompress ESE data with API
fn decompress_ese(data: &mut [u8], decom_size: u32) -> Vec<u8> {
    use crate::utils::compression::xpress::api::decompress_huffman_api;

    let decom_result = decompress_huffman_api(data, &XpressType::Lz77, decom_size);
    match decom_result {
        Ok(result) => result,
        Err(err) => {
            error!(
                "[ese] Could not decompress Lz77 data with API: {err:?}. Will try manual decompression"
            );
            let decom_result = decompress_xpress(data, decom_size, &XpressType::Lz77);
            match decom_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[ese] Could not decompress Lz77 data with API or manually: {err:?}");
                    data.to_vec()
                }
            }
        }
    }
}

#[cfg(target_family = "unix")]
/// Decompress ESE data
fn decompress_ese(data: &mut [u8], decom_size: u32) -> Vec<u8> {
    let decom_result = decompress_xpress(data, decom_size, &XpressType::Lz77);
    match decom_result {
        Ok(result) => result,
        Err(err) => {
            error!("[ese] Could not decompress Lz77 data: {err:?}");
            data.to_vec()
        }
    }
}

/// Parse the row of the table
pub(crate) fn parse_row(leaf_row: PageLeaf, column_info: &mut [ColumnInfo]) {
    if leaf_row.leaf_type != LeafType::DataDefinition {
        return;
    }
    // All leaf data for the table has Data Definition type
    // All calls to parse_row check for Data Definition type
    let leaf_data: DataDefinition = serde_json::from_value(leaf_row.leaf_data).unwrap();

    let _ = parse_fixed_data(
        leaf_data.last_fixed_data,
        &leaf_data.fixed_data,
        column_info,
    );

    let _ = parse_variable_data(
        leaf_data.last_variable_data,
        &leaf_data.variable_data,
        column_info,
    );
}

/// Parse the fixed data of a column
fn parse_fixed_data<'a>(
    last_fixed_data: u8,
    fixed_data: &'a [u8],
    column_info: &mut [ColumnInfo],
) -> nom::IResult<&'a [u8], ()> {
    let mut column = 1;
    let mut data = fixed_data;
    while column <= last_fixed_data {
        for entry in column_info.iter_mut() {
            if entry.column_id == column as i32 {
                let (input, column_data) =
                    nom_fixed_column(&entry.column_type, data, entry.column_space_usage)?;
                data = input;
                entry.column_data = column_data;
            }
        }
        column += 1;
    }

    Ok((fixed_data, ()))
}

/// Parse the variable data of a column. Follows fixed data
fn parse_variable_data<'a>(
    last_variable: u8,
    variable_data: &'a [u8],
    column_info: &mut [ColumnInfo],
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
        for entry in column_info.iter_mut() {
            if entry.column_id == var_data.column as i32 {
                let (input, column_data) = take(size)(data)?;
                data = input;
                entry.column_data = column_data.to_vec();
            }
        }
        previous_size = var_data.size;
    }

    if !data.is_empty() {
        parse_tagged_data(data, column_info)?;
    }
    Ok((data, ()))
}

/// Parsed the tagged data of a column. Follows variable data
fn parse_tagged_data<'a>(
    tagged_data: &'a [u8],
    column_info: &mut [ColumnInfo],
) -> nom::IResult<&'a [u8], ()> {
    let (input, column) = nom_unsigned_two_bytes(tagged_data, Endian::Le)?;
    let (_input, mut offset) = nom_unsigned_two_bytes(input, Endian::Le)?;

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
                let tag_size = if next_value.offset > bit_flag {
                    (next_value.offset - bit_flag) - (value.offset - bit_flag)
                } else {
                    next_value.offset - (value.offset - bit_flag)
                };

                let (input, data) = take(tag_size)(tag_data_start)?;
                tag_data_start = input;
                let (tag_data, flag) = nom_unsigned_one_byte(data, Endian::Le)?;
                let flags = Catalog::get_flags(flag);

                let tag = TaggedData {
                    column: value.column,
                    offset: value.offset,
                    flags,
                    data: tag_data.to_vec(),
                };

                full_tags.push(tag);
                continue;
            }

            // Check and make sure next tag offset is lower than the bit flag
            let tag_size = if next_value.offset > bit_flag {
                (next_value.offset - bit_flag) - value.offset
            } else {
                next_value.offset - value.offset
            };

            let (input, data) = take(tag_size)(tag_data_start)?;
            tag_data_start = input;
            let (tag_data, flag) = nom_unsigned_one_byte(data, Endian::Le)?;
            let flags = Catalog::get_flags(flag);

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
            let (tag_data, flag) = nom_unsigned_one_byte(tag_data_start, Endian::Le)?;
            let flags = Catalog::get_flags(flag);

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
        let flags = Catalog::get_flags(flag);

        let tag = TaggedData {
            column: value.column,
            offset: value.offset,
            flags,
            data: tag_data.to_vec(),
        };
        full_tags.push(tag);
    }

    // Nearly done, need to update columns now
    for tag in full_tags {
        for entry in column_info.iter_mut() {
            if entry.column_id == tag.column as i32 {
                entry.column_data.clone_from(&tag.data);
                entry.column_tagged_flags.clone_from(&tag.flags);
            }
        }
    }

    Ok((tagged_data, ()))
}

pub(crate) fn parse_multivalue_data<'a>(
    tag_data: &'a [u8],
    flags: &[TaggedDataFlag],
) -> nom::IResult<&'a [u8], Vec<Vec<u8>>> {
    let mut data = tag_data;
    let mut offsets_sizes = Vec::new();

    let mut multi_data = Vec::new();
    if flags.contains(&TaggedDataFlag::MultiValue) {
        let (input, offset) = nom_unsigned_two_bytes(data, Endian::Le)?;
        data = input;
        let adjust = 1;

        // determine the number of offsets. We already got the first one
        let mut offset_count = (offset / 2) - adjust;
        if offset_count as usize > data.len() {
            warn!(
                "[ese] Multivalue offset count too large, got offset {offset_count}. Data length is: {}",
                data.len()
            );
            return Ok((data, multi_data));
        }
        offsets_sizes.push(offset);
        while offset_count != 0 {
            let (input, offset) = nom_unsigned_two_bytes(data, Endian::Le)?;
            offsets_sizes.push(offset);
            data = input;

            offset_count -= adjust;
        }

        let mut peek_offsets = offsets_sizes.iter().peekable();

        while let Some(value) = peek_offsets.next() {
            // Need to subtract the current offset from next offset to get data size
            let size = if let Some(next_offset) = peek_offsets.peek() {
                *next_offset - value
            } else {
                // remaining data is used for size
                data.len() as u16
            };
            if size as usize > data.len() {
                warn!("[ese] Multivalue offset length too large");
                return Ok((data, multi_data));
            }
            let (input, value_data) = take(size)(data)?;
            data = input;

            if flags.contains(&TaggedDataFlag::Compressed) {
                let (_, decom_data) = get_decompressed_data(value_data)?;
                multi_data.push(decom_data);
                continue;
            }

            multi_data.push(value_data.to_vec());
        }
    }
    Ok((data, multi_data))
}

/// Clear column data so when we go to the next row there is no leftover data from previous row
pub(crate) fn clear_column_data(column_info: &mut [ColumnInfo]) {
    for entry in column_info.iter_mut() {
        entry.column_data.clear();
    }
}

/// Nom the fixed column data. Columns that are fixed have static sizes (ex: GUID is 16 bytes)
fn nom_fixed_column<'a>(
    column_type: &ColumnType,
    data: &'a [u8],
    column_space_usage: i32,
) -> nom::IResult<&'a [u8], Vec<u8>> {
    let (input, column_data) = match column_type {
        ColumnType::Nil => {
            warn!("[ese] Invalid column type 'NIL'");
            (data, data)
        }
        ColumnType::Bit | ColumnType::UnsignedByte => take(size_of::<u8>())(data)?,
        ColumnType::Short | ColumnType::UnsignedShort => take(size_of::<u16>())(data)?,
        ColumnType::Float32 => take(size_of::<f32>())(data)?,
        ColumnType::Float64 => take(size_of::<f64>())(data)?,
        ColumnType::DateTime | ColumnType::LongLong | ColumnType::Currency => {
            take(size_of::<u64>())(data)?
        }
        ColumnType::UnsignedLong | ColumnType::Long => take(size_of::<u32>())(data)?,
        ColumnType::Guid => take(size_of::<u128>())(data)?,
        ColumnType::Binary | ColumnType::LongBinary | ColumnType::Text | ColumnType::LongText => {
            take(column_space_usage as u32)(data)?
        }
        _ => {
            error!("[ese] Invalid fixed column type {column_type:?}");
            take(column_space_usage as u32)(data)?
        }
    };
    Ok((input, column_data.to_vec()))
}

/// Get the column type. Determines what kind of data is stored in the column
pub(crate) fn get_column_type(column: i32) -> ColumnType {
    match column {
        0 => ColumnType::Nil,
        1 => ColumnType::Bit,
        2 => ColumnType::UnsignedByte,
        3 => ColumnType::Short,
        4 => ColumnType::Long,
        5 => ColumnType::Currency,
        6 => ColumnType::Float32,
        7 => ColumnType::Float64,
        8 => ColumnType::DateTime,
        9 => ColumnType::Binary,
        10 => ColumnType::Text,
        11 => ColumnType::LongBinary,
        12 => ColumnType::LongText,
        13 => ColumnType::SuperLong,
        14 => ColumnType::UnsignedLong,
        15 => ColumnType::LongLong,
        16 => ColumnType::Guid,
        17 => ColumnType::UnsignedShort,
        _ => ColumnType::Unknown,
    }
}

/// Get flags associated with the column
pub(crate) fn get_column_flags(flags: i32) -> Vec<ColumnFlags> {
    let not_null = 0x1;
    let version = 0x2;
    let increment = 0x4;
    let multi = 0x8;
    let default = 0x10;
    let escrow = 0x20;
    let finalize = 0x40;
    let user_define = 0x80;
    let template = 0x100;
    let delete_zero = 0x200;
    let primary = 0x800;
    let compressed = 0x1000;
    let encrypted = 0x2000;
    let versioned = 0x10000;
    let deleted = 0x20000;
    let version_add = 0x40000;

    let mut flags_data = Vec::new();
    if (flags & not_null) == not_null {
        flags_data.push(ColumnFlags::NotNull);
    }
    if (flags & version) == version {
        flags_data.push(ColumnFlags::Version);
    }
    if (flags & increment) == increment {
        flags_data.push(ColumnFlags::AutoIncrement);
    }
    if (flags & multi) == multi {
        flags_data.push(ColumnFlags::MultiValued);
    }
    if (flags & default) == default {
        flags_data.push(ColumnFlags::Default);
    }
    if (flags & escrow) == escrow {
        flags_data.push(ColumnFlags::EscrowUpdate);
    }
    if (flags & finalize) == finalize {
        flags_data.push(ColumnFlags::Finalize);
    }
    if (flags & user_define) == user_define {
        flags_data.push(ColumnFlags::UserDefinedDefault);
    }
    if (flags & template) == template {
        flags_data.push(ColumnFlags::TemplateColumnESE98);
    }
    if (flags & delete_zero) == delete_zero {
        flags_data.push(ColumnFlags::DeleteOnZero);
    }
    if (flags & primary) == primary {
        flags_data.push(ColumnFlags::PrimaryIndexPlaceholder);
    }
    if (flags & compressed) == compressed {
        flags_data.push(ColumnFlags::Compressed);
    }
    if (flags & encrypted) == encrypted {
        flags_data.push(ColumnFlags::Encrypted);
    }
    if (flags & versioned) == versioned {
        flags_data.push(ColumnFlags::Versioned);
    }
    if (flags & deleted) == deleted {
        flags_data.push(ColumnFlags::Deleted);
    }
    if (flags & version_add) == version_add {
        flags_data.push(ColumnFlags::VersionedAdd);
    }
    flags_data
}

#[cfg(test)]
mod tests {
    use super::ColumnInfo;
    use crate::artifacts::os::windows::ese::{
        catalog::TaggedDataFlag,
        pages::leaf::{LeafType, PageLeaf},
        tables::{
            ColumnFlags, ColumnType, clear_column_data, column_data_to_string, create_table_data,
            decompress_ese, extract_column_data_to_string, get_column_flags, get_column_type,
            get_decompressed_data, nom_fixed_column, parse_fixed_data, parse_multivalue_data,
            parse_row, parse_tagged_data, parse_variable_data,
        },
    };
    use serde_json::json;

    #[test]
    fn test_parse_fixed_data() {
        let last_fixed = 1;
        let test = [2, 0, 0, 0];
        let info = ColumnInfo {
            column_type: ColumnType::Long,
            column_name: String::new(),
            column_data: Vec::new(),
            column_id: 1,
            column_flags: Vec::new(),
            column_space_usage: 0,
            column_tagged_flags: Vec::new(),
        };
        let mut info_vec = vec![info];
        let (_, _) = parse_fixed_data(last_fixed, &test, &mut info_vec).unwrap();
        assert_eq!(info_vec[0].column_data, [2, 0, 0, 0]);
    }

    #[test]
    fn test_parse_variable_data() {
        let last_variable = 128;
        let test = [11, 0, 77, 83, 121, 115, 79, 98, 106, 101, 99, 116, 115];
        let info = ColumnInfo {
            column_type: ColumnType::Binary,
            column_name: String::new(),
            column_data: Vec::new(),
            column_id: 128,
            column_flags: Vec::new(),
            column_space_usage: 0,
            column_tagged_flags: Vec::new(),
        };
        let mut info_vec = vec![info];
        let (_, _) = parse_variable_data(last_variable, &test, &mut info_vec).unwrap();
        assert_eq!(
            info_vec[0].column_data,
            [77, 83, 121, 115, 79, 98, 106, 101, 99, 116, 115]
        );
    }

    #[test]
    fn test_parse_tagged_data() {
        let test = [5, 1, 4, 0, 1, 101, 0, 110, 0, 45, 0, 85, 0, 83, 0];
        let info = ColumnInfo {
            column_type: ColumnType::Binary,
            column_name: String::new(),
            column_data: Vec::new(),
            column_id: 261,
            column_flags: Vec::new(),
            column_space_usage: 0,
            column_tagged_flags: Vec::new(),
        };
        let mut info_vec = vec![info];
        let (_, _) = parse_tagged_data(&test, &mut info_vec).unwrap();
        assert_eq!(
            info_vec[0].column_data,
            [101, 0, 110, 0, 45, 0, 85, 0, 83, 0]
        );
        assert_eq!(info_vec[0].column_tagged_flags, [TaggedDataFlag::Variable]);
    }

    #[test]
    fn test_column_data_to_string() {
        let col_type = ColumnType::Long;
        let data = [4, 0, 0, 0];
        let flags = Vec::new();
        let tagged_flags = Vec::new();

        let (_, results) = column_data_to_string(&col_type, &data, &flags, &tagged_flags).unwrap();
        assert_eq!(results, "4");
    }

    #[test]
    fn test_create_table_data() {
        let info = vec![ColumnInfo {
            column_type: ColumnType::Long,
            column_name: String::from("IdFileLocal"),
            column_data: vec![11, 0, 0, 0],
            column_id: 1,
            column_flags: vec![ColumnFlags::AutoIncrement],
            column_space_usage: 4,
            column_tagged_flags: Vec::new(),
        }];
        let rows = vec![info];
        let name = "test";
        let results = create_table_data(&rows, name);
        let values = results.get("test").unwrap();
        assert_eq!(values[0][0].column_data, "11");
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
            leaf_data: json!({"last_fixed_data":1, "last_variable_data":127, "variable_data_offset":21, "fixed_data": vec![172,4,101,38,116,217,108,68,150,173,43,225,58,86,101,176,254], "variable_data": vec![0,1,4,0,5,12,0,0,128,0,0,0,128]}),
        };
        let mut col = [
            ColumnInfo {
                column_type: ColumnType::Guid,
                column_name: String::from("Id"),
                column_data: Vec::new(),
                column_id: 1,
                column_flags: Vec::new(),
                column_space_usage: 16,
                column_tagged_flags: Vec::new(),
            },
            ColumnInfo {
                column_type: ColumnType::LongBinary,
                column_name: String::from("Blob"),
                column_data: Vec::new(),
                column_id: 256,
                column_flags: Vec::new(),
                column_space_usage: 0,
                column_tagged_flags: Vec::new(),
            },
        ];
        let _ = parse_row(leaf, &mut col);
        assert_eq!(
            col[0].column_data,
            [
                172, 4, 101, 38, 116, 217, 108, 68, 150, 173, 43, 225, 58, 86, 101, 176
            ]
        );
        assert_eq!(col[1].column_data, [12, 0, 0, 128, 0, 0, 0, 128]);
    }

    #[test]
    fn test_clear_column_data() {
        let info = ColumnInfo {
            column_type: ColumnType::Binary,
            column_name: String::new(),
            column_data: vec![0, 0, 1],
            column_id: 15,
            column_flags: Vec::new(),
            column_space_usage: 0,
            column_tagged_flags: Vec::new(),
        };
        let mut info_vec = vec![info];

        clear_column_data(&mut info_vec);
        assert_eq!(info_vec[0].column_data.is_empty(), true);
    }

    #[test]
    fn test_get_column_type() {
        let test = 2;
        let result = get_column_type(test);
        assert_eq!(result, ColumnType::UnsignedByte);
    }

    #[test]
    fn test_nom_fixed_column() {
        let test_data = [2, 0, 0, 0];
        let data_type = ColumnType::Long;
        let (_, result) = nom_fixed_column(&data_type, &test_data, 0).unwrap();
        assert_eq!(result, [2, 0, 0, 0])
    }

    #[test]
    fn test_get_column_flags() {
        let test = 4096;
        let flags = get_column_flags(test);
        assert_eq!(flags, vec![ColumnFlags::Compressed]);
    }

    #[test]
    fn test_parse_multivalue_data() {
        let flags = vec![TaggedDataFlag::MultiValue, TaggedDataFlag::Compressed];
        let test = [
            8, 0, 19, 0, 51, 0, 121, 0, 20, 215, 180, 155, 252, 190, 207, 65, 49, 24, 0, 87, 0,
            105, 0, 110, 0, 100, 0, 111, 0, 119, 0, 115, 0, 32, 0, 49, 0, 48, 0, 32, 0, 76, 0, 84,
            0, 83, 0, 66, 0, 0, 0, 87, 0, 105, 0, 110, 0, 100, 0, 111, 0, 119, 0, 115, 0, 32, 0,
            49, 0, 48, 0, 44, 0, 32, 0, 118, 0, 101, 0, 114, 0, 115, 0, 105, 0, 111, 0, 110, 0, 32,
            0, 49, 0, 57, 0, 48, 0, 51, 0, 32, 0, 97, 0, 110, 0, 100, 0, 32, 0, 108, 0, 97, 0, 116,
            0, 101, 0, 114, 0, 0, 0, 87, 0, 105, 0, 110, 0, 100, 0, 111, 0, 119, 0, 115, 0, 32, 0,
            49, 0, 49, 0, 0, 0,
        ];
        let (_, data) = parse_multivalue_data(&test, &flags).unwrap();
        assert_eq!(
            data,
            vec![
                vec![87, 105, 110, 100, 111, 119, 115, 32, 49, 48, 0],
                vec![
                    87, 0, 105, 0, 110, 0, 100, 0, 111, 0, 119, 0, 115, 0, 32, 0, 49, 0, 48, 0, 32,
                    0, 76, 0, 84, 0, 83, 0, 66, 0, 0, 0
                ],
                vec![
                    87, 0, 105, 0, 110, 0, 100, 0, 111, 0, 119, 0, 115, 0, 32, 0, 49, 0, 48, 0, 44,
                    0, 32, 0, 118, 0, 101, 0, 114, 0, 115, 0, 105, 0, 111, 0, 110, 0, 32, 0, 49, 0,
                    57, 0, 48, 0, 51, 0, 32, 0, 97, 0, 110, 0, 100, 0, 32, 0, 108, 0, 97, 0, 116,
                    0, 101, 0, 114, 0, 0, 0
                ],
                vec![
                    87, 0, 105, 0, 110, 0, 100, 0, 111, 0, 119, 0, 115, 0, 32, 0, 49, 0, 49, 0, 0,
                    0
                ]
            ]
        );
    }

    #[test]
    fn test_get_decompressed_data_seven_bit() {
        let test = [18, 213, 121, 89, 62, 7];
        let (_, results) = get_decompressed_data(&test).unwrap();
        assert_eq!(results, [85, 115, 101, 114, 115]);
    }

    #[test]
    fn test_get_decompressed_data_string_seven_bit() {
        let test = [
            19, 247, 180, 155, 94, 150, 155, 203, 108, 182, 203, 253, 150, 211, 209, 174, 121, 217,
            94, 118, 175, 211, 238, 51, 249, 221, 158, 187, 216, 239, 113, 152, 13,
        ];
        let (_, results) = get_decompressed_data(&test).unwrap();
        assert_eq!(
            results,
            [
                119, 105, 110, 116, 101, 114, 102, 101, 108, 108, 46, 110, 111, 114, 116, 104, 46,
                115, 101, 118, 101, 110, 107, 105, 110, 103, 100, 111, 109, 115, 46, 108, 111, 99,
                97, 108
            ]
        );
    }

    #[test]
    fn test_get_decompressed_data_seven_bit_but_not_compressed() {
        let test = [67, 0, 121, 0, 103, 0, 119, 0, 105, 0, 110, 0];
        let (_, results) = get_decompressed_data(&test).unwrap();
        assert_eq!(results, [67, 0, 121, 0, 103, 0, 119, 0, 105, 0, 110, 0]);
    }

    #[test]
    fn test_get_decompressed_data_huffman() {
        let test = [
            24, 0, 8, 65, 0, 0, 0, 80, 0, 69, 0, 67, 0, 109, 0, 100, 0, 32, 0, 118, 0, 101, 0, 114,
            0, 115, 0, 105, 0, 111, 0, 110, 120, 0, 49, 0, 46, 0, 52, 24, 0, 88, 81, 4, 68, 48, 26,
            0, 13, 0, 10, 26, 0, 65, 0, 117, 0, 116, 0, 104, 24, 1, 114, 0, 58, 40, 1, 69, 56, 0,
            105, 0, 99, 72, 0, 90, 56, 0, 109, 8, 0, 41, 2, 109, 0, 97, 136, 162, 136, 173, 26, 2,
            40, 120, 2, 97, 138, 0, 25, 1, 122, 40, 0, 15, 1, 132, 64, 0, 103, 74, 0, 105, 0, 108,
            40, 3, 99, 168, 2, 109, 0, 41, 58, 3, 104, 40, 3, 116, 0, 112, 248, 1, 58, 0, 47, 106,
            221, 99, 177, 8, 0, 103, 24, 1, 185, 3, 117, 0, 98, 78, 1, 47, 238, 3, 223, 3, 47, 0,
            80, 232, 0, 219, 6, 121, 2, 25, 0, 67, 170, 1, 251, 0, 137, 7, 108, 168, 1, 110, 152,
            1, 57, 6, 45, 138, 0, 67, 88, 0, 92, 162, 43, 42, 0, 0, 87, 0, 73, 0, 78, 0, 68, 0, 79,
            72, 0, 83, 120, 0, 80, 168, 2, 101, 0, 102, 24, 0, 116, 88, 3, 104, 136, 0, 105, 2, 25,
            0, 75, 152, 0, 121, 0, 119, 184, 2, 114, 189, 42, 122, 212, 248, 1, 89, 5, 32, 24, 1,
            101, 24, 3, 112, 0, 44, 90, 0, 73, 0, 73, 1, 25, 0, 76, 56, 1, 111, 0, 107, 170, 3,
            103, 232, 0, 102, 104, 0, 114, 56, 0, 112, 40, 0, 255, 2, 242, 201, 0, 137, 8, 101,
            136, 2, 240, 253, 90, 107, 32, 72, 0, 73, 10, 39, 223, 4, 16, 39, 90, 3, 29, 0, 70, 24,
            3, 117, 8, 2, 233, 6, 50, 152, 14, 54, 56, 0, 159, 1, 38, 127, 3, 153, 1, 25, 0, 25, 1,
            111, 232, 0, 153, 0, 153, 16, 139, 5, 47, 4, 15, 17, 65, 0, 77, 0, 85, 171, 70, 85, 95,
            8, 1, 69, 168, 7, 84, 104, 0, 95, 24, 1, 65, 72, 0, 67, 0, 72, 88, 0, 217, 18, 51, 8,
            0, 57, 56, 0, 49, 184, 4, 54, 8, 0, 41, 19, 46, 120, 1, 45, 152, 5, 66, 168, 0, 49, 40,
            0, 171, 213, 174, 86, 56, 216, 0, 69, 168, 0, 112, 216, 2, 191, 6, 67, 106, 3, 97, 88,
            3, 101, 186, 6, 121, 21, 201, 11, 50, 8, 2, 25, 7, 45, 56, 0, 53, 40, 0, 50, 248, 21,
            169, 0, 51, 232, 0, 50, 88, 2, 58, 216, 0, 75, 22, 171, 138, 87, 93, 77, 152, 1, 100,
            168, 6, 185, 7, 31, 2, 255, 31, 76, 8, 4, 115, 24, 4, 32, 56, 0, 99, 8, 0, 75, 9, 111,
            2, 31, 25, 0, 69, 0, 120, 248, 1, 99, 120, 13, 116, 152, 2, 98, 138, 12, 32, 72, 2, 97,
            40, 18, 43, 21, 87, 181, 186, 215, 175, 10, 111, 33, 249, 2, 72, 72, 2, 115, 232, 12,
            73, 2, 31, 11, 249, 0, 70, 104, 8, 155, 3, 137, 15, 122, 74, 0, 40, 56, 4, 121, 120, 4,
            233, 6, 41, 202, 1, 56, 200, 22, 52, 200, 2, 52, 154, 1, 86, 200, 0, 175, 33, 32, 249,
            0, 175, 187, 245, 110, 87, 88, 0, 121, 19, 111, 184, 24, 249, 21, 15, 7, 82, 216, 6,
            105, 22, 249, 29, 73, 0, 116, 138, 1, 49, 218, 0, 127, 10, 240, 114, 218, 0, 201, 0,
            255, 9, 9, 48, 88, 0, 233, 1, 25, 0, 86, 168, 2, 108, 216, 1, 105, 9, 187, 25, 11, 27,
            170, 162, 186, 189, 137, 31, 116, 120, 0, 11, 5, 73, 1, 25, 0, 35, 216, 1, 25, 3, 78,
            232, 0, 137, 1, 89, 0, 92, 24, 2, 79, 72, 4, 85, 56, 11, 69, 0, 123, 8, 1, 49, 72, 6,
            48, 56, 2, 57, 40, 1, 49, 40, 0, 99, 94, 109, 177, 170, 56, 4, 56, 232, 7, 49, 184, 3,
            52, 136, 4, 100, 40, 0, 57, 40, 0, 123, 11, 102, 0, 125, 40, 2, 83, 104, 2, 185, 36,
            97, 104, 4, 169, 2, 68, 26, 1, 68, 28, 1, 70, 136, 0, 15, 22, 4, 25, 1, 73, 7, 49, 127,
            255, 117, 219, 216, 2, 73, 7, 57, 40, 0, 233, 6, 32, 88, 0, 73, 7, 53, 168, 1, 75, 17,
            89, 2, 105, 218, 1, 99, 216, 1, 217, 6, 249, 21, 187, 35, 201, 30, 15, 14, 25, 1, 121,
            28, 57, 0, 110, 104, 1, 45, 1, 25, 6, 233, 7, 25, 0, 95, 2, 172, 223, 1, 85, 117, 85,
            188, 27, 5, 54, 170, 1, 25, 0, 203, 9, 111, 9, 255, 43, 92, 0, 36, 216, 1, 88, 24, 22,
            69, 72, 12, 68, 234, 2, 49, 239, 2, 50, 79, 37, 244, 233, 2, 50, 239, 2, 64, 92, 24, 0,
            79, 24, 12, 84, 104, 0, 65, 88, 23, 69, 200, 0, 255, 255, 95, 85, 73, 152, 0, 84, 88,
            0, 73, 72, 27, 85, 72, 0, 73, 8, 1, 78, 58, 4, 201, 14, 63, 4, 15, 103,
        ];
        let (_, result) = get_decompressed_data(&test).unwrap();
        assert_eq!(result.len(), 2048);
    }

    #[test]
    fn test_decompress_ese() {
        let mut test = [
            65, 0, 0, 0, 80, 0, 69, 0, 67, 0, 109, 0, 100, 0, 32, 0, 118, 0, 101, 0, 114, 0, 115,
            0, 105, 0, 111, 0, 110, 120, 0, 49, 0, 46, 0, 52, 24, 0, 88, 81, 4, 68, 48, 26, 0, 13,
            0, 10, 26, 0, 65, 0, 117, 0, 116, 0, 104, 24, 1, 114, 0, 58, 40, 1, 69, 56, 0, 105, 0,
            99, 72, 0, 90, 56, 0, 109, 8, 0, 41, 2, 109, 0, 97, 136, 162, 136, 173, 26, 2, 40, 120,
            2, 97, 138, 0, 25, 1, 122, 40, 0, 15, 1, 132, 64, 0, 103, 74, 0, 105, 0, 108, 40, 3,
            99, 168, 2, 109, 0, 41, 58, 3, 104, 40, 3, 116, 0, 112, 248, 1, 58, 0, 47, 106, 221,
            99, 177, 8, 0, 103, 24, 1, 185, 3, 117, 0, 98, 78, 1, 47, 238, 3, 223, 3, 47, 0, 80,
            232, 0, 219, 6, 121, 2, 25, 0, 67, 170, 1, 251, 0, 137, 7, 108, 168, 1, 110, 152, 1,
            57, 6, 45, 138, 0, 67, 88, 0, 92, 162, 43, 42, 0, 0, 87, 0, 73, 0, 78, 0, 68, 0, 79,
            72, 0, 83, 120, 0, 80, 168, 2, 101, 0, 102, 24, 0, 116, 88, 3, 104, 136, 0, 105, 2, 25,
            0, 75, 152, 0, 121, 0, 119, 184, 2, 114, 189, 42, 122, 212, 248, 1, 89, 5, 32, 24, 1,
            101, 24, 3, 112, 0, 44, 90, 0, 73, 0, 73, 1, 25, 0, 76, 56, 1, 111, 0, 107, 170, 3,
            103, 232, 0, 102, 104, 0, 114, 56, 0, 112, 40, 0, 255, 2, 242, 201, 0, 137, 8, 101,
            136, 2, 240, 253, 90, 107, 32, 72, 0, 73, 10, 39, 223, 4, 16, 39, 90, 3, 29, 0, 70, 24,
            3, 117, 8, 2, 233, 6, 50, 152, 14, 54, 56, 0, 159, 1, 38, 127, 3, 153, 1, 25, 0, 25, 1,
            111, 232, 0, 153, 0, 153, 16, 139, 5, 47, 4, 15, 17, 65, 0, 77, 0, 85, 171, 70, 85, 95,
            8, 1, 69, 168, 7, 84, 104, 0, 95, 24, 1, 65, 72, 0, 67, 0, 72, 88, 0, 217, 18, 51, 8,
            0, 57, 56, 0, 49, 184, 4, 54, 8, 0, 41, 19, 46, 120, 1, 45, 152, 5, 66, 168, 0, 49, 40,
            0, 171, 213, 174, 86, 56, 216, 0, 69, 168, 0, 112, 216, 2, 191, 6, 67, 106, 3, 97, 88,
            3, 101, 186, 6, 121, 21, 201, 11, 50, 8, 2, 25, 7, 45, 56, 0, 53, 40, 0, 50, 248, 21,
            169, 0, 51, 232, 0, 50, 88, 2, 58, 216, 0, 75, 22, 171, 138, 87, 93, 77, 152, 1, 100,
            168, 6, 185, 7, 31, 2, 255, 31, 76, 8, 4, 115, 24, 4, 32, 56, 0, 99, 8, 0, 75, 9, 111,
            2, 31, 25, 0, 69, 0, 120, 248, 1, 99, 120, 13, 116, 152, 2, 98, 138, 12, 32, 72, 2, 97,
            40, 18, 43, 21, 87, 181, 186, 215, 175, 10, 111, 33, 249, 2, 72, 72, 2, 115, 232, 12,
            73, 2, 31, 11, 249, 0, 70, 104, 8, 155, 3, 137, 15, 122, 74, 0, 40, 56, 4, 121, 120, 4,
            233, 6, 41, 202, 1, 56, 200, 22, 52, 200, 2, 52, 154, 1, 86, 200, 0, 175, 33, 32, 249,
            0, 175, 187, 245, 110, 87, 88, 0, 121, 19, 111, 184, 24, 249, 21, 15, 7, 82, 216, 6,
            105, 22, 249, 29, 73, 0, 116, 138, 1, 49, 218, 0, 127, 10, 240, 114, 218, 0, 201, 0,
            255, 9, 9, 48, 88, 0, 233, 1, 25, 0, 86, 168, 2, 108, 216, 1, 105, 9, 187, 25, 11, 27,
            170, 162, 186, 189, 137, 31, 116, 120, 0, 11, 5, 73, 1, 25, 0, 35, 216, 1, 25, 3, 78,
            232, 0, 137, 1, 89, 0, 92, 24, 2, 79, 72, 4, 85, 56, 11, 69, 0, 123, 8, 1, 49, 72, 6,
            48, 56, 2, 57, 40, 1, 49, 40, 0, 99, 94, 109, 177, 170, 56, 4, 56, 232, 7, 49, 184, 3,
            52, 136, 4, 100, 40, 0, 57, 40, 0, 123, 11, 102, 0, 125, 40, 2, 83, 104, 2, 185, 36,
            97, 104, 4, 169, 2, 68, 26, 1, 68, 28, 1, 70, 136, 0, 15, 22, 4, 25, 1, 73, 7, 49, 127,
            255, 117, 219, 216, 2, 73, 7, 57, 40, 0, 233, 6, 32, 88, 0, 73, 7, 53, 168, 1, 75, 17,
            89, 2, 105, 218, 1, 99, 216, 1, 217, 6, 249, 21, 187, 35, 201, 30, 15, 14, 25, 1, 121,
            28, 57, 0, 110, 104, 1, 45, 1, 25, 6, 233, 7, 25, 0, 95, 2, 172, 223, 1, 85, 117, 85,
            188, 27, 5, 54, 170, 1, 25, 0, 203, 9, 111, 9, 255, 43, 92, 0, 36, 216, 1, 88, 24, 22,
            69, 72, 12, 68, 234, 2, 49, 239, 2, 50, 79, 37, 244, 233, 2, 50, 239, 2, 64, 92, 24, 0,
            79, 24, 12, 84, 104, 0, 65, 88, 23, 69, 200, 0, 255, 255, 95, 85, 73, 152, 0, 84, 88,
            0, 73, 72, 27, 85, 72, 0, 73, 8, 1, 78, 58, 4, 201, 14, 63, 4, 15, 103,
        ];
        let out = decompress_ese(&mut test, 2048);
        assert_eq!(out.len(), 2048);
    }

    #[test]
    fn test_extract_column_data_to_string_multivalue_compressed() {
        let test = [
            19, 247, 180, 155, 94, 150, 155, 203, 108, 182, 203, 253, 150, 211, 209, 174, 121, 217,
            94, 118, 175, 211, 238, 51, 249, 221, 158, 187, 216, 239, 113, 152, 13,
        ];
        let column = ColumnType::LongText;
        let flags = vec![ColumnFlags::Compressed];
        let tagged_flags = vec![
            TaggedDataFlag::Variable,
            TaggedDataFlag::LongValue,
            TaggedDataFlag::MultiValue,
        ];
        let (_, result) =
            extract_column_data_to_string(&column, &test, &flags, &tagged_flags).unwrap();
        assert_eq!(result, "winterfell.north.sevenkingdoms.local");
    }

    #[test]
    fn test_extract_column_data_to_string_multivalue_tag_compressed_tag_not_compressed() {
        let test = [
            83, 0, 69, 0, 86, 0, 69, 0, 78, 0, 75, 0, 73, 0, 78, 0, 71, 0, 68, 0, 79, 0, 77, 0, 83,
            0, 92, 0, 75, 0, 73, 0, 78, 0, 71, 0, 83, 0, 76, 0, 65, 0, 78, 0, 68, 0, 73, 0, 78, 0,
            71, 0, 36, 0,
        ];
        let column = ColumnType::LongText;
        let tagged_flags = vec![
            TaggedDataFlag::Variable,
            TaggedDataFlag::LongValue,
            TaggedDataFlag::MultiValue,
            TaggedDataFlag::Compressed,
        ];
        let (_, result) =
            extract_column_data_to_string(&column, &test, &[], &tagged_flags).unwrap();
        assert_eq!(result, "SEVENKINGDOMS\\KINGSLANDING$");
    }

    #[test]
    fn test_parse_tagged_data_extended_table() {
        let test = [
            0, 1, 24, 64, 4, 1, 29, 64, 5, 1, 129, 64, 6, 1, 137, 64, 7, 1, 192, 64, 11, 1, 247,
            64, 5, 1, 0, 0, 0, 3, 23, 138, 49, 121, 172, 91, 167, 221, 231, 57, 59, 236, 38, 167,
            221, 103, 215, 188, 108, 47, 187, 215, 105, 247, 153, 252, 110, 207, 93, 236, 247, 56,
            204, 86, 200, 219, 100, 221, 58, 237, 62, 207, 217, 97, 55, 57, 237, 62, 187, 230, 101,
            123, 217, 189, 78, 187, 207, 228, 119, 123, 238, 98, 191, 199, 97, 182, 66, 49, 30,
            183, 117, 235, 180, 251, 60, 103, 135, 221, 228, 180, 251, 236, 154, 151, 237, 101,
            247, 58, 237, 62, 147, 223, 237, 185, 139, 253, 30, 135, 217, 3, 17, 201, 249, 188, 94,
            38, 3, 1, 83, 0, 69, 0, 86, 0, 69, 0, 78, 0, 75, 0, 73, 0, 78, 0, 71, 0, 68, 0, 79, 0,
            77, 0, 83, 0, 92, 0, 75, 0, 73, 0, 78, 0, 71, 0, 83, 0, 76, 0, 65, 0, 78, 0, 68, 0, 73,
            0, 78, 0, 71, 0, 36, 0, 1, 83, 0, 69, 0, 86, 0, 69, 0, 78, 0, 75, 0, 73, 0, 78, 0, 71,
            0, 68, 0, 79, 0, 77, 0, 83, 0, 92, 0, 75, 0, 73, 0, 78, 0, 71, 0, 83, 0, 76, 0, 65, 0,
            78, 0, 68, 0, 73, 0, 78, 0, 71, 0, 36, 0, 1, 48,
        ];

        let mut cols = Vec::new();
        let req = ColumnInfo {
            column_type: ColumnType::LongText,
            column_name: String::from("$RequesterName"),
            column_data: Vec::new(),
            column_id: 262,
            column_flags: Vec::new(),
            column_space_usage: 2048,
            column_tagged_flags: vec![TaggedDataFlag::Variable],
        };
        let mut caller = req.clone();
        caller.column_name = String::from("$CallerName");
        caller.column_id = 263;
        cols.push(caller);
        cols.push(req);
        parse_tagged_data(&test, &mut cols).unwrap();
        assert_eq!(cols[1].column_data.len(), 54);
        assert_eq!(cols[0].column_data.len(), 54);
    }
}
