use crate::utils::nom_helper::{Endian, nom_unsigned_four_bytes};
use nom::error::ErrorKind;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct SdbHeader {
    pub(crate) major_version: u32,
    pub(crate) minor_version: u32,
    sig: u32,
}

impl SdbHeader {
    pub(crate) fn parse_header(data: &[u8]) -> nom::IResult<&[u8], SdbHeader> {
        let (input, major_version) = nom_unsigned_four_bytes(data, Endian::Le)?;
        let (input, minor_version) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, sig) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let sdb_sig = 0x66626473;
        if sig != sdb_sig {
            return Err(nom::Err::Failure(nom::error::Error::new(
                data,
                ErrorKind::Fail,
            )));
        }

        let sdb_header = SdbHeader {
            major_version,
            minor_version,
            sig,
        };
        Ok((input, sdb_header))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{artifacts::os::windows::shimdb::header::SdbHeader, filesystem::files::read_file};

    #[test]
    fn test_parse_header() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/shimdb/win10/sysmain.sdb");

        let buffer = read_file(&test_location.display().to_string()).unwrap();

        let (_, header) = SdbHeader::parse_header(&buffer).unwrap();
        assert_eq!(header.major_version, 3);
        assert_eq!(header.minor_version, 0);
        assert_eq!(header.sig, 0x66626473) // sdbf
    }
}
