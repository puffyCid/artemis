use crate::utils::{
    encoding::base64_encode_standard,
    nom_helper::{Endian, nom_unsigned_four_bytes},
    uuid::format_guid_le_bytes,
};
use nom::bytes::complete::take;

/// Get binary data and base64 encode it
pub(crate) fn parse_binary<'a>(data: &'a [u8], tag_value: &u16) -> nom::IResult<&'a [u8], String> {
    let (input, binary_data) = get_binary(data)?;

    let tag_app_id = 0x9011;
    let tag_exe_id = 0x9004;
    let tag_fix_id = 0x9010;
    if tag_value == &tag_app_id || tag_value == &tag_exe_id || tag_value == &tag_fix_id {
        return Ok((input, format_guid_le_bytes(binary_data)));
    }

    let binary_value = base64_encode_standard(binary_data);
    Ok((input, binary_value))
}

/// Grab the raw binary data
pub(crate) fn get_binary(data: &[u8]) -> nom::IResult<&[u8], &[u8]> {
    let (input, binary_size) = nom_unsigned_four_bytes(data, Endian::Le)?;

    let (input, binary_data) = take(binary_size)(input)?;

    Ok((input, binary_data))
}

#[cfg(test)]
mod tests {
    use super::{get_binary, parse_binary};

    #[test]
    fn test_parse_binary() {
        let test_data = [
            16, 0, 0, 0, 166, 192, 126, 161, 45, 215, 149, 47, 157, 134, 61, 18, 81, 227, 217, 212,
        ];

        let (_, result) = parse_binary(&test_data, &1).unwrap();
        assert_eq!(result, "psB+oS3XlS+dhj0SUePZ1A==")
    }

    #[test]
    fn test_get_binary() {
        let test_data = [
            16, 0, 0, 0, 166, 192, 126, 161, 45, 215, 149, 47, 157, 134, 61, 18, 81, 227, 217, 212,
        ];

        let (_, result) = get_binary(&test_data).unwrap();
        assert_eq!(
            result,
            [
                166, 192, 126, 161, 45, 215, 149, 47, 157, 134, 61, 18, 81, 227, 217, 212,
            ]
        )
    }
}
