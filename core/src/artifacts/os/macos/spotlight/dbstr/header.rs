use crate::{
    artifacts::os::macos::spotlight::error::SpotlightError,
    utils::nom_helper::{nom_unsigned_eight_bytes, nom_unsigned_four_bytes, Endian},
};
use log::error;

pub(crate) struct DbHeader {
    _sig: u64,
    _unknown: u32,
    _unknown2: u32,
    _unknown3: u32,
    _data_size: u32,
    _bucket_entries: u32,
    pub(crate) offset_entries: u32,
}

/// Get Dbstr header info
pub(crate) fn get_header(data: &[u8]) -> Result<DbHeader, SpotlightError> {
    let header_results = parse_header(data);
    let header = match header_results {
        Ok((_, results)) => results,
        Err(_err) => {
            error!("[spotlight] Could not parse dbstr header data");
            return Err(SpotlightError::Header);
        }
    };

    Ok(header)
}

/// Parse the header info associated with Dbstr files
fn parse_header(data: &[u8]) -> nom::IResult<&[u8], DbHeader> {
    let (input, sig) = nom_unsigned_eight_bytes(data, Endian::Le)?;
    let (input, unknown) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, unknown2) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, unknown3) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, data_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, bucket_entries) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, offset_entries) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let header = DbHeader {
        _sig: sig,
        _unknown: unknown,
        _unknown2: unknown2,
        _unknown3: unknown3,
        _data_size: data_size,
        _bucket_entries: bucket_entries,
        offset_entries,
    };

    Ok((input, header))
}

#[cfg(test)]
mod tests {
    use super::{get_header, parse_header};
    use crate::filesystem::{files::read_file, metadata::glob_paths};
    use std::path::PathBuf;

    #[test]
    fn test_get_header() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/spotlight/bigsur/*.header");
        let headers = glob_paths(test_location.to_str().unwrap()).unwrap();
        for header in headers {
            let data = read_file(&header.full_path).unwrap();
            let db_header = get_header(&data).unwrap();
            assert!(db_header.offset_entries >= 1);
        }
    }

    #[test]
    fn test_parse_header() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/spotlight/bigsur/*.header");
        let headers = glob_paths(test_location.to_str().unwrap()).unwrap();
        for header in headers {
            let data = read_file(&header.full_path).unwrap();
            let (_, db_header) = parse_header(&data).unwrap();
            assert!(db_header.offset_entries >= 1);
        }
    }
}
