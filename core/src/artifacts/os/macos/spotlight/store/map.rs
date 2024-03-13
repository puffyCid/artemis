use crate::utils::nom_helper::{nom_unsigned_eight_bytes, nom_unsigned_four_bytes, Endian};

/// Parse store map
pub(crate) fn parse_map(data: &[u8]) -> nom::IResult<&[u8], Vec<u32>> {
    let (input, _sig) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (input, _page_size) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let (input, map_count) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, _unknown) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (mut input, _unknown2) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let mut count = 0;
    let mut block_numbers = Vec::new();
    while count < map_count {
        let (remaining, _unknown) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (remaining, block) = nom_unsigned_four_bytes(remaining, Endian::Le)?;
        let (remaining, _unknown2) = nom_unsigned_four_bytes(remaining, Endian::Le)?;
        block_numbers.push(block);

        input = remaining;
        count += 1;
    }

    Ok((input, block_numbers))
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::macos::spotlight::store::map::parse_map, filesystem::files::read_file,
    };
    use std::path::PathBuf;

    #[test]
    fn test_parse_map() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/spotlight/bigsur/map.raw");

        let data = read_file(test_location.to_str().unwrap()).unwrap();

        let (_, results) = parse_map(&data).unwrap();
        assert_eq!(results.len(), 714);
    }
}
