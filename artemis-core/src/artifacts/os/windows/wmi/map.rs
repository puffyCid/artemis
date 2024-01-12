use crate::utils::nom_helper::{nom_unsigned_four_bytes, Endian};

#[derive(Debug)]
pub(crate) struct MapInfo {
    pub(crate) seq_number: u32,
    number_pages: u32,
    pub(crate) mappings: Vec<u32>,
    pub(crate) seq_number2: u32,
    number_pages2: u32,
    pub(crate) mappings2: Vec<u32>,
}

/**
 * Parse the mapping data. Need to parse all mapping files. The file with the greatest sequence number is the active mapping file
 */
pub(crate) fn parse_map(data: &[u8]) -> nom::IResult<&[u8], MapInfo> {
    // First section applies to objects.data
    let (input, (seq_number, number_pages)) = parse_header(data)?;
    let (input, mappings) = parse_mapping(input)?;
    let (input, _table) = parse_table(input)?;

    // Seconcd section applies to index.btr
    let (input, (seq_number2, number_pages2)) = parse_header(input)?;
    let (input, mappings2) = parse_mapping(input)?;
    let (input, _table2) = parse_table(input)?;

    let info = MapInfo {
        seq_number,
        number_pages,
        mappings,
        seq_number2,
        number_pages2,
        mappings2,
    };
    Ok((input, info))
}

fn parse_header(data: &[u8]) -> nom::IResult<&[u8], (u32, u32)> {
    let (input, _sig) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (input, seq_number) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, _unknown) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, _unknown2) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, number_pages) = nom_unsigned_four_bytes(input, Endian::Le)?;

    Ok((input, (seq_number, number_pages)))
}

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

    //page_numbers.sort();

    Ok((map_input, page_numbers))
}

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
    use crate::{artifacts::os::windows::wmi::map::parse_map, filesystem::files::read_file};
    use std::path::PathBuf;

    #[test]
    fn test_parse_map() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/wmi/MAPPING1.MAP");

        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let (_, results) = parse_map(&data).unwrap();
        println!("{results:?}");
    }

    #[test]
    fn test_parse_map_live() {
        let data = read_file("C:\\Windows\\System32\\wbem\\Repository\\MAPPING3.MAP").unwrap();
        let (_, results) = parse_map(&data).unwrap();
        println!("{results:?}");
    }
}
