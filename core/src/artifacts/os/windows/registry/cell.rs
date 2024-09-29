use crate::utils::nom_helper::{nom_signed_four_bytes, Endian};

use super::{
    keys::{nk::NameKey, vk::ValueKey},
    lists::{lf::Leaf, lh::HashLeaf, li::LeafItem, ri::RefItem},
    parser::Params,
};
use common::windows::KeyValue;
use log::{error, warn};
use nom::{
    bytes::complete::take, combinator::peek, error::ErrorKind, number::complete::le_u16, Needed,
};
use std::mem::size_of;

#[derive(Debug, PartialEq)]
pub(crate) enum CellType {
    Nk,
    Vk,
    Sk,
    Lf,
    Lh,
    Li,
    Ri,
    Db,
    Unknown,
}

/// Check for a cell type from provided bytes
pub(crate) fn get_cell_type(data: &[u8]) -> nom::IResult<&[u8], CellType> {
    // Take a peek at the cell data to determine the type, but do not nom the data
    let (cell_data, cell_type_data) = peek(take(size_of::<u16>()))(data)?;
    let (_, cell_type) = le_u16(cell_type_data)?;

    let cell = match cell_type {
        0x6b6e => CellType::Nk,
        0x686c => CellType::Lh,
        0x6b73 => CellType::Sk,
        0x6b76 => CellType::Vk,
        0x6972 => CellType::Ri,
        0x666c => CellType::Lf,
        0x6264 => CellType::Db,
        0x696c => CellType::Li,
        _ => {
            error!("[registry] Unknown cell: {cell_type}");
            CellType::Unknown
        }
    };

    Ok((cell_data, cell))
}

/// Iterate through the Registry data based on provided offset
pub(crate) fn walk_registry<'a>(
    reg_data: &'a [u8],
    offset: u32,
    params: &mut Params,
    minor_version: u32,
) -> nom::IResult<&'a [u8], ()> {
    if let Some(_value) = params.offset_tracker.get(&offset) {
        error!("[registry] Detected duplicate Registry offset: {offset}. This triggers infinite loops, stopping parsing and exiting early.");
        return Err(nom::Err::Failure(nom::error::Error::new(
            reg_data,
            ErrorKind::Fail,
        )));
    }
    params.offset_tracker.insert(offset, offset);
    let (list_data, _) = take(offset)(reg_data)?;
    // Get the size of the list and check if its allocated (negative numbers = allocated, postive number = unallocated)
    let (list_data, (allocated, size)) = is_allocated(list_data)?;
    if !allocated {
        return Ok((reg_data, ()));
    }
    // Size includes the size itself. We nommed that away
    let adjust_cell_size = 4;
    if size < adjust_cell_size {
        return Err(nom::Err::Incomplete(Needed::Unknown));
    }
    // Grab all data associated with the list based on list size
    let (_, list_data) = take(size - adjust_cell_size)(list_data)?;

    let (list_data, cell_type) = get_cell_type(list_data)?;

    if cell_type == CellType::Lh {
        HashLeaf::parse_hash_leaf(reg_data, list_data, params, minor_version)?;
    } else if cell_type == CellType::Nk {
        NameKey::parse_name_key(reg_data, list_data, params, minor_version)?;
    } else if cell_type == CellType::Lf {
        Leaf::parse_leaf(reg_data, list_data, params, minor_version)?;
    } else if cell_type == CellType::Li {
        LeafItem::parse_leaf_item(reg_data, list_data, params, minor_version)?;
    } else if cell_type == CellType::Ri {
        RefItem::parse_reference_item(reg_data, list_data, params, minor_version)?;
    } else {
        error!("[registry] Got unknown cell type: {cell_type:?}.");
        return Err(nom::Err::Failure(nom::error::Error::new(
            reg_data,
            ErrorKind::Fail,
        )));
    }
    params.offset_tracker.remove_entry(&offset);

    Ok((reg_data, ()))
}

/// Walkthrough the values list associated with a Name key
pub(crate) fn walk_values<'a>(
    reg_data: &'a [u8],
    offset: u32,
    number_values: &u32,
    minor_version: u32,
) -> nom::IResult<&'a [u8], Vec<KeyValue>> {
    // Go to the value list offset
    let (list_data, _) = take(offset)(reg_data)?;

    // Get the size of the list and check if its allocated (negative numbers = allocated, postive number = unallocated)
    let (list_data, (allocated, size)) = is_allocated(list_data)?;
    if !allocated {
        return Ok((reg_data, Vec::new()));
    }

    // Size includes the size itself. We nommed that away
    let adjust_cell_size = 4;
    if size < adjust_cell_size {
        return Err(nom::Err::Incomplete(Needed::Unknown));
    }
    // Grab all data associated with the list based on list size
    let (_, mut list_data) = take(size - adjust_cell_size)(list_data)?;

    let mut value_count = 0;
    let mut key_values: Vec<KeyValue> = Vec::new();
    // Go through each value offset in the list
    while &value_count < number_values && !list_data.is_empty() {
        // Get the value key offset
        let (input, vk_offset) = nom_signed_four_bytes(list_data, Endian::Le)?;
        list_data = input;

        let empty_offset = 0;
        let unallocated = -1;
        if vk_offset == empty_offset || vk_offset == unallocated {
            value_count += 1;
            continue;
        }
        // Go to the value key offset
        let (vk_data, _) = take(vk_offset as u32)(reg_data)?;

        // Get the size of the valeu key and check if its allocated (negative numbers = allocated, postive number = unallocated)
        let (vk_data, (allocated, size)) = is_allocated(vk_data)?;
        if !allocated {
            value_count += 1;
            continue;
        }

        // Size includes the size itself. We nommed that away
        let adjust_cell_size = 4;
        if size < adjust_cell_size {
            return Err(nom::Err::Incomplete(Needed::Unknown));
        }
        let (_, vk_data) = take(size - adjust_cell_size)(vk_data)?;
        // Check for the value key signature (vk)
        let (vk_data, cell_type) = get_cell_type(vk_data)?;
        if cell_type != CellType::Vk {
            warn!("[registry] Got non Vk cell type while iterating value list: {cell_type:?}");
            value_count += 1;
            continue;
        }
        // Parse the Value key data
        let (_, value_key) = ValueKey::parse_value_key(reg_data, vk_data, minor_version)?;
        let value = KeyValue {
            value: value_key.value_name,
            data: value_key.data,
            data_type: value_key.data_type,
        };
        key_values.push(value);
        value_count += 1;
    }

    Ok((reg_data, key_values))
}

/// Check if a cell is allocated. Negative number = allocated, postive number = unallocated
pub(crate) fn is_allocated(data: &[u8]) -> nom::IResult<&[u8], (bool, u32)> {
    let (list_data, mut list_size) = nom_signed_four_bytes(data, Endian::Le)?;
    let cell_allocated = 0;

    // If the size is a postive number then the cell is unallocated (deleted)
    // We currently do not parse deleted cells
    if list_size >= cell_allocated {
        return Ok((list_data, (false, 0)));
    }
    // Allocated cells have a negative cell size
    let size_inverse = -1;
    list_size *= size_inverse;
    Ok((list_data, (true, list_size as u32)))
}

#[cfg(test)]
mod tests {
    use super::{get_cell_type, is_allocated, CellType};
    use crate::{
        artifacts::os::windows::registry::{
            cell::{walk_registry, walk_values},
            hbin::HiveBin,
            parser::Params,
        },
        filesystem::files::read_file,
    };
    use regex::Regex;
    use std::{collections::HashMap, path::PathBuf};

    #[test]
    fn test_get_cell_type() {
        let mut test_data = [0x6e, 0x6b];
        let (_, cell) = get_cell_type(&test_data).unwrap();
        assert_eq!(cell, CellType::Nk);

        test_data = [0x6c, 0x68];
        let (_, cell) = get_cell_type(&test_data).unwrap();
        assert_eq!(cell, CellType::Lh);

        test_data = [0x73, 0x6b];
        let (_, cell) = get_cell_type(&test_data).unwrap();
        assert_eq!(cell, CellType::Sk);

        test_data = [0x76, 0x6b];
        let (_, cell) = get_cell_type(&test_data).unwrap();
        assert_eq!(cell, CellType::Vk);

        test_data = [0x72, 0x69];
        let (_, cell) = get_cell_type(&test_data).unwrap();
        assert_eq!(cell, CellType::Ri);

        test_data = [0x6c, 0x66];
        let (_, cell) = get_cell_type(&test_data).unwrap();
        assert_eq!(cell, CellType::Lf);

        test_data = [0x64, 0x62];
        let (_, cell) = get_cell_type(&test_data).unwrap();
        assert_eq!(cell, CellType::Db);

        test_data = [0x6c, 0x69];
        let (_, cell) = get_cell_type(&test_data).unwrap();
        assert_eq!(cell, CellType::Li);

        test_data = [0x66, 0x6a];
        let (_, cell) = get_cell_type(&test_data).unwrap();
        assert_eq!(cell, CellType::Unknown);
    }

    #[test]
    fn test_walk_registry() {
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
            registry_path: String::from("test\\test"),
        };
        let (_, result) = walk_registry(&buffer, 216, &mut params, 4).unwrap();

        assert_eq!(result, ())
    }

    #[test]
    fn test_is_allocated() {
        let test_data = [12, 12, 12, 12];
        let (_, (allocated, size)) = is_allocated(&test_data).unwrap();
        assert_eq!(allocated, false);
        assert_eq!(size, 0);
    }

    #[test]
    fn test_walk_values() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/registry/win10/hbins.raw");

        let buffer = read_file(&test_location.display().to_string()).unwrap();

        let (_, result) = HiveBin::parse_hive_bin_header(&buffer).unwrap();

        assert_eq!(result.size, 4096);
        let (_, result) = walk_values(&buffer, 752, &1, 4).unwrap();

        assert_eq!(result.len(), 1)
    }
}
