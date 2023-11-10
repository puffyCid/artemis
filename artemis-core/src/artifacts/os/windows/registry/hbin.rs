use super::parser::Params;
use crate::{
    artifacts::os::windows::registry::{
        cell::{get_cell_type, is_allocated, CellType},
        keys::nk::NameKey,
    },
    utils::nom_helper::{nom_unsigned_eight_bytes, nom_unsigned_four_bytes, Endian},
};
use common::windows::RegistryEntry;
use nom::{bytes::complete::take, Needed};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct HiveBin {
    signature: u32,
    offset: u32,
    pub(crate) size: u32,
    reserved: u64,
    timestamp: u64,
    spare: u32,
}

impl HiveBin {
    /// Parse the hive bin (hbin) header to determine size of hbin (should be multiple of 4096 bytes)
    pub(crate) fn parse_hive_bin_header(data: &[u8]) -> nom::IResult<&[u8], HiveBin> {
        let (input, signature) = nom_unsigned_four_bytes(data, Endian::Le)?;
        let (input, offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, size) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, reserved) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, timestamp) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, spare) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let hbin = HiveBin {
            signature,
            offset,
            size,
            reserved,
            timestamp,
            spare,
        };

        Ok((input, hbin))
    }

    /// Start parsing the Registry from the ROOT key
    pub(crate) fn parse_hive_cells<'a>(
        reg_data: &'a [u8],
        hive_data: &'a [u8],
        params: &mut Params,
        minor_version: u32,
    ) -> nom::IResult<&'a [u8], Vec<RegistryEntry>> {
        let skip_header: usize = 32;
        // We already parsed the header data. We can skip it
        let (input, _) = take(skip_header)(hive_data)?;
        let mut all_cells_data = input;

        while !all_cells_data.is_empty() {
            // Get the size of the list and check if its allocated (negative numbers = allocated, postive number = unallocated)
            let (input, (allocated, size)) = is_allocated(all_cells_data)?;

            // Size includes the size itself. We nommed that away
            let adjust_cell_size = 4;
            if size < adjust_cell_size {
                return Err(nom::Err::Incomplete(Needed::Unknown));
            }

            if !allocated {
                // Size is a non-negative number so it should be ok to convert to unsigned
                let (input, _) = take(size - adjust_cell_size)(input)?;
                all_cells_data = input;
                continue;
            }

            let (input, cell_data) = take(size - adjust_cell_size)(input)?;
            all_cells_data = input;

            let (cell_data, cell) = get_cell_type(cell_data)?;

            // Name key cells are the only cells we want, they contain all the info needed to parse the Registry
            if cell != CellType::Nk {
                continue;
            }
            // We only need the ROOT key
            NameKey::parse_name_key(reg_data, cell_data, params, minor_version)?;
            break;
        }

        let mut reg_list: Vec<RegistryEntry> = Vec::new();
        reg_list.append(&mut params.registry_list);

        Ok((reg_data, reg_list))
    }
}

#[cfg(test)]
mod tests {
    use super::HiveBin;
    use crate::{artifacts::os::windows::registry::parser::Params, filesystem::files::read_file};
    use regex::Regex;
    use std::{collections::HashMap, path::PathBuf};

    #[test]
    fn test_parse_hive_bin_header() {
        let test_data = [
            104, 98, 105, 110, 0, 0, 0, 0, 0, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0,
        ];

        let (_, result) = HiveBin::parse_hive_bin_header(&test_data).unwrap();

        assert_eq!(result.signature, 0x6e696268); // hbin
        assert_eq!(result.offset, 0);
        assert_eq!(result.size, 4096);
        assert_eq!(result.reserved, 0);
        assert_eq!(result.timestamp, 0);
        assert_eq!(result.spare, 0);
    }

    #[test]
    fn test_parse_hive_cells() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/registry/win10/hbins.raw");

        let buffer = read_file(&test_location.display().to_string()).unwrap();

        let (_, result) = HiveBin::parse_hive_bin_header(&buffer).unwrap();

        assert_eq!(result.signature, 0x6e696268); // hbin
        assert_eq!(result.offset, 0);
        assert_eq!(result.size, 4096);
        assert_eq!(result.reserved, 0);
        assert_eq!(result.timestamp, 0);
        assert_eq!(result.spare, 0);

        let mut params = Params {
            start_path: String::from("ROOT"),
            path_regex: Regex::new("").unwrap(),
            registry_list: Vec::new(),
            key_tracker: Vec::new(),
            offset_tracker: HashMap::new(),
            filter: false,
        };

        let (_, result) = HiveBin::parse_hive_cells(&buffer, &buffer, &mut params, 4).unwrap();
        assert_eq!(result.len(), 666)
    }
}
