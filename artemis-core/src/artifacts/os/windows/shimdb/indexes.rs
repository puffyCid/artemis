use nom::bytes::complete::take;

use crate::utils::nom_helper::{nom_unsigned_four_bytes, Endian};

/// Get the indexes data associated with a sdb file
pub(crate) fn get_indexes_data(data: &[u8]) -> nom::IResult<&[u8], Vec<u8>> {
    let (_, indexes_size) = nom_unsigned_four_bytes(data, Endian::Le)?;

    // Nom the whole list including the indexes size
    let indexes_list_size = 4;
    let (input, indexes_data) = take(indexes_size as usize + indexes_list_size)(data)?;
    Ok((input, indexes_data.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::get_indexes_data;
    use crate::filesystem::files::read_file;
    use std::path::PathBuf;

    #[test]
    fn test_parse_shimdb() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/shimdb/win10/indexes.raw");

        let buffer = read_file(&test_location.display().to_string()).unwrap();

        let (_, indexes) = get_indexes_data(&buffer).unwrap();
        assert_eq!(indexes.len(), 153618)
    }
}
