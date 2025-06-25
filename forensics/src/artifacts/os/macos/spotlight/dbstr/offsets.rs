use crate::{
    artifacts::os::macos::spotlight::error::SpotlightError,
    utils::nom_helper::{Endian, nom_unsigned_four_bytes},
};
use log::error;
use nom::bytes::complete::take;

/// The offsets associated with Spotlight data
pub(crate) fn get_offsets(data: &[u8], offset_entries: u32) -> Result<Vec<u32>, SpotlightError> {
    let offset_results = parse_offsets(data, offset_entries);
    let offsets = match offset_results {
        Ok((_, result)) => result,
        Err(_err) => {
            error!("[spotlight] Could not parse dbstr offsets data");
            return Err(SpotlightError::Offsets);
        }
    };

    Ok(offsets)
}

/// Parse the offsets info associated with Dbstr files
fn parse_offsets<'a>(data: &'a [u8], offset_entries: u32) -> nom::IResult<&'a [u8], Vec<u32>> {
    let offset_size = 4;
    let (input, mut offsets_data) = take(offset_entries * offset_size)(data)?;

    let mut offsets = Vec::new();

    let min_size = 4;
    while !offsets_data.is_empty() && offsets_data.len() >= min_size {
        let (input, offset) = nom_unsigned_four_bytes(offsets_data, Endian::Le)?;
        offsets.push(offset);
        offsets_data = input;
    }

    Ok((input, offsets))
}

#[cfg(test)]
mod tests {
    use super::{get_offsets, parse_offsets};
    use crate::{
        artifacts::os::macos::spotlight::dbstr::header::get_header,
        filesystem::{files::read_file, metadata::glob_paths},
    };
    use std::path::PathBuf;

    #[test]
    fn test_get_offsets() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/spotlight/bigsur/*.header");
        let headers = glob_paths(test_location.to_str().unwrap()).unwrap();
        for header in headers {
            // 3 is always empty and contains no data
            if header.full_path.contains("3") {
                continue;
            }
            let data = read_file(&header.full_path).unwrap();
            let db_header = get_header(&data).unwrap();
            let offsets = header.full_path.replace("header", "offsets");
            let offset_data = read_file(&offsets).unwrap();
            let offsets_vec = get_offsets(&offset_data, db_header.offset_entries).unwrap();
            assert!(offsets_vec.len() >= 1)
        }
    }

    #[test]
    fn test_parse_offsets() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/spotlight/bigsur/*.header");
        let headers = glob_paths(test_location.to_str().unwrap()).unwrap();
        for header in headers {
            // 3 is always empty and contains no data
            if header.full_path.contains("3") {
                continue;
            }
            let data = read_file(&header.full_path).unwrap();
            let db_header = get_header(&data).unwrap();
            let offsets = header.full_path.replace("header", "offsets");
            let offset_data = read_file(&offsets).unwrap();
            let (_, offsets_vec) = parse_offsets(&offset_data, db_header.offset_entries).unwrap();
            assert!(offsets_vec.len() >= 1)
        }
    }
}
