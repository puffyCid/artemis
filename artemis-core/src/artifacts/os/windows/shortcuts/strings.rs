use crate::utils::{
    nom_helper::{nom_unsigned_two_bytes, Endian},
    strings::{extract_utf16_string, extract_utf8_string},
};
use common::windows::DataFlags;
use nom::bytes::complete::take;

/// Extract strings from `shortcut` data
pub(crate) fn extract_string<'a>(
    data: &'a [u8],
    flags: &[DataFlags],
) -> nom::IResult<&'a [u8], String> {
    let (input, size) = nom_unsigned_two_bytes(data, Endian::Le)?;

    // Size for UTF16 chars (2 bytes)
    let adjust_size = 2;
    for flag in flags {
        if flag != &DataFlags::IsUnicode {
            continue;
        }
        let (input, string_data) = take(size * adjust_size)(input)?;
        let data_string = extract_utf16_string(string_data);
        return Ok((input, data_string));
    }
    let (input, string_data) = take(size)(input)?;
    let data_string = extract_utf8_string(string_data);
    Ok((input, data_string))
}

#[cfg(test)]
mod tests {
    use super::extract_string;
    use common::windows::DataFlags;

    #[test]
    fn test_extract_string() {
        let test = [
            41, 0, 46, 0, 46, 0, 92, 0, 46, 0, 46, 0, 92, 0, 46, 0, 46, 0, 92, 0, 46, 0, 46, 0, 92,
            0, 46, 0, 46, 0, 92, 0, 80, 0, 114, 0, 111, 0, 106, 0, 101, 0, 99, 0, 116, 0, 115, 0,
            92, 0, 82, 0, 117, 0, 115, 0, 116, 0, 92, 0, 97, 0, 114, 0, 116, 0, 101, 0, 109, 0,
            105, 0, 115, 0, 45, 0, 99, 0, 111, 0, 114, 0, 101, 0,
        ];
        let (_, result) = extract_string(&test, &[DataFlags::IsUnicode]).unwrap();
        assert_eq!(result, "..\\..\\..\\..\\..\\Projects\\Rust\\artemis-core");
    }
}
