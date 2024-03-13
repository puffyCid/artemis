use crate::utils::nom_helper::{nom_unsigned_four_bytes, Endian};
use nom::bytes::complete::take;

/// Get the stringtable data associated with a sdb file
pub(crate) fn get_stringtable_data(data: &[u8]) -> nom::IResult<&[u8], Vec<u8>> {
    let (input, list_size) = nom_unsigned_four_bytes(data, Endian::Le)?;

    let (input, list_data) = take(list_size)(input)?;
    Ok((input, list_data.to_vec()))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::filesystem::files::read_file;

    use super::get_stringtable_data;

    #[test]
    fn test_get_stringtable_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/shimdb/win10/stringtable.raw");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let (_, result) = get_stringtable_data(&buffer).unwrap();
        assert_eq!(result.len(), 1687580)
    }
}
