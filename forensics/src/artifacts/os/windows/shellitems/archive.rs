use crate::utils::{
    nom_helper::{
        Endian, nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_one_byte,
        nom_unsigned_two_bytes,
    },
    strings::extract_utf16_string,
    time::{filetime_to_unixepoch, unixepoch_to_iso},
};
use nom::bytes::complete::take;

/// Parse new Archive `ShellItem` format added in Windows 11
pub(crate) fn parse_archive(data: &[u8]) -> nom::IResult<&[u8], (String, String)> {
    let (input, _unknown) = nom_unsigned_one_byte(data, Endian::Le)?;
    let (input, _unknown2) = nom_unsigned_one_byte(input, Endian::Le)?;

    let (_input, offset) = nom_unsigned_one_byte(input, Endian::Le)?;
    let adjust = 2;
    let (input, _) = take(offset - adjust)(data)?;

    let (input, time_bytes) = nom_unsigned_eight_bytes(input, Endian::Le)?;
    let (input, _unknown3) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let (input, size) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, size2) = nom_unsigned_two_bytes(input, Endian::Le)?;

    let (remaining, string_bytes) = take((size + size2) * adjust as u16)(input)?;

    // Modified timestamp of target folder
    let modified = unixepoch_to_iso(&filetime_to_unixepoch(time_bytes));
    let path = extract_utf16_string(string_bytes);

    Ok((remaining, (modified, path)))
}

#[cfg(test)]
mod tests {
    use super::parse_archive;

    #[test]
    fn test_parse_archive_7z() {
        let test = [
            208, 57, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 137, 84, 47, 22, 141, 133, 218, 1, 0, 0,
            0, 0, 32, 0, 0, 0, 100, 0, 101, 0, 102, 0, 101, 0, 110, 0, 100, 0, 101, 0, 114, 0, 45,
            0, 100, 0, 97, 0, 116, 0, 97, 0, 98, 0, 97, 0, 115, 0, 101, 0, 45, 0, 101, 0, 120, 0,
            116, 0, 114, 0, 97, 0, 99, 0, 116, 0, 45, 0, 109, 0, 97, 0, 115, 0, 116, 0, 101, 0,
            114, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let (_, (modified, path)) = parse_archive(&test).unwrap();
        assert_eq!(modified, "2024-04-03T06:06:36.000Z");
        assert_eq!(path, "defender-database-extract-master");
    }

    #[test]
    fn test_parse_archive() {
        let test = [
            102, 59, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 187, 103, 70, 181, 141, 133, 218, 1, 1,
            0, 0, 0, 32, 0, 33, 0, 100, 0, 101, 0, 102, 0, 101, 0, 110, 0, 100, 0, 101, 0, 114, 0,
            45, 0, 100, 0, 97, 0, 116, 0, 97, 0, 98, 0, 97, 0, 115, 0, 101, 0, 45, 0, 101, 0, 120,
            0, 116, 0, 114, 0, 97, 0, 99, 0, 116, 0, 45, 0, 109, 0, 97, 0, 115, 0, 116, 0, 101, 0,
            114, 0, 92, 0, 100, 0, 101, 0, 102, 0, 101, 0, 110, 0, 100, 0, 101, 0, 114, 0, 45, 0,
            100, 0, 97, 0, 116, 0, 97, 0, 98, 0, 97, 0, 115, 0, 101, 0, 45, 0, 101, 0, 120, 0, 116,
            0, 114, 0, 97, 0, 99, 0, 116, 0, 45, 0, 109, 0, 97, 0, 115, 0, 116, 0, 101, 0, 114, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let (_, (modified, path)) = parse_archive(&test).unwrap();
        assert_eq!(modified, "2024-04-03T06:11:03.000Z");
        assert_eq!(
            path,
            "defender-database-extract-master\\defender-database-extract-master"
        );
    }

    #[test]
    fn test_parse_archive_tar() {
        let test = [
            37, 35, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 247, 39, 205, 36, 212, 217, 1, 0, 0,
            0, 0, 12, 0, 0, 0, 103, 0, 111, 0, 45, 0, 101, 0, 115, 0, 101, 0, 45, 0, 48, 0, 46, 0,
            50, 0, 46, 0, 48, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let (_, (modified, path)) = parse_archive(&test).unwrap();
        assert_eq!(modified, "2023-08-21T11:44:11.000Z");
        assert_eq!(path, "go-ese-0.2.0");
    }
}
