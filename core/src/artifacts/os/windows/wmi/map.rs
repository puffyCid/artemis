use crate::utils::nom_helper::{Endian, nom_unsigned_four_bytes};

#[derive(Debug)]
pub(crate) struct MapInfo {
    _seq_number: u32,
    _number_pages: u32,
    pub(crate) mappings: Vec<u32>,
    pub(crate) seq_number2: u32,
    _number_pages2: u32,
    _mappings2: Vec<u32>,
}

/**
 * Parse the mapping data. Need to parse all mapping files. The file with the greatest sequence number is the active mapping file
 */
pub(crate) fn parse_map(data: &[u8]) -> nom::IResult<&[u8], MapInfo> {
    // First section applies to objects.data
    let (input, (seq_number, number_pages)) = parse_header(data)?;
    let (input, mappings) = parse_mapping(input)?;
    let (input, _table) = parse_table(input)?;

    // Second section applies to index.btr
    let (input, (seq_number2, number_pages2)) = parse_header(input)?;
    let (input, mappings2) = parse_mapping(input)?;
    let (input, _table2) = parse_table(input)?;

    let info = MapInfo {
        _seq_number: seq_number,
        _number_pages: number_pages,
        mappings,
        seq_number2,
        _number_pages2: number_pages2,
        _mappings2: mappings2,
    };
    Ok((input, info))
}

/// Parse header info for map data
fn parse_header(data: &[u8]) -> nom::IResult<&[u8], (u32, u32)> {
    let (input, _sig) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (input, seq_number) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, _unknown) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, _unknown2) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, number_pages) = nom_unsigned_four_bytes(input, Endian::Le)?;

    Ok((input, (seq_number, number_pages)))
}

/// Parse map data and get page numbers
fn parse_mapping(data: &[u8]) -> nom::IResult<&[u8], Vec<u32>> {
    let (mut map_input, num_entries) = nom_unsigned_four_bytes(data, Endian::Le)?;

    let mut entries = 0;
    let mut page_numbers = Vec::new();
    while entries < num_entries {
        let (input, number) = nom_unsigned_four_bytes(map_input, Endian::Le)?;
        page_numbers.push(number);

        let (input, _checksum) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, _frees_pace) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, _used_space) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, _unknown) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, _unknown2) = nom_unsigned_four_bytes(input, Endian::Le)?;
        map_input = input;
        entries += 1;
    }

    Ok((map_input, page_numbers))
}

/// Get table info from map data
fn parse_table(data: &[u8]) -> nom::IResult<&[u8], Vec<u32>> {
    let (mut table_input, num_entries) = nom_unsigned_four_bytes(data, Endian::Le)?;

    let mut entries = 0;
    let mut table_numbers = Vec::new();
    while entries < num_entries {
        let (input, number) = nom_unsigned_four_bytes(table_input, Endian::Le)?;
        table_numbers.push(number);

        table_input = input;
        entries += 1;
    }

    let (input, _footer) = nom_unsigned_four_bytes(table_input, Endian::Le)?;
    Ok((input, table_numbers))
}

#[cfg(test)]
mod tets {
    use super::{parse_header, parse_mapping};
    use crate::{
        artifacts::os::windows::wmi::map::{parse_map, parse_table},
        filesystem::files::read_file,
    };
    use std::path::PathBuf;

    #[test]
    fn test_parse_map() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/wmi/MAPPING1.MAP");

        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let (_, results) = parse_map(&data).unwrap();
        assert_eq!(results.mappings.len(), 3293);
    }

    #[test]
    fn test_parse_header() {
        let test = vec![0, 0, 0, 0, 1, 2, 3, 4, 1, 2, 3, 4, 4, 0, 0, 0, 1, 0, 0, 0];
        let (_, (seq, pages)) = parse_header(&test).unwrap();
        assert_eq!(seq, 67305985);
        assert_eq!(pages, 1);
    }

    #[test]
    fn test_parse_mapping() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/wmi/MAPPING1.MAP");

        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let (input, (_, _)) = parse_header(&data).unwrap();
        let (_, mappings) = parse_mapping(input).unwrap();
        assert_eq!(mappings.len(), 3293);
    }

    #[test]
    fn test_parse_table() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/wmi/MAPPING1.MAP");

        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let (input, (_, _)) = parse_header(&data).unwrap();
        let (input, _) = parse_mapping(input).unwrap();
        let (_, table) = parse_table(input).unwrap();

        assert_eq!(table.len(), 238);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_parse_map_live() {
        let data = read_file("C:\\Windows\\System32\\wbem\\Repository\\MAPPING3.MAP").unwrap();
        let (_, results) = parse_map(&data).unwrap();
        assert!(results.mappings.len() > 10);
    }
}
