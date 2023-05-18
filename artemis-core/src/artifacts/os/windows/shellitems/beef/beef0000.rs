use crate::utils::nom_helper::{nom_unsigned_two_bytes, Endian};
use crate::utils::uuid::format_guid_le_bytes;
use nom::bytes::complete::take;
use std::mem::size_of;

/// Parse a `0xbeef0000` block. May contain a GUID
pub(crate) fn parse_beef(data: &[u8]) -> nom::IResult<&[u8], String> {
    let (input, size) = nom_unsigned_two_bytes(data, Endian::Le)?;
    let (input, _extension) = take(size_of::<u16>())(input)?;
    let (input, _signature) = take(size_of::<u32>())(input)?;

    let guid_size = 42;
    if size != guid_size {
        return Ok((input, String::from("Unknown beef00000 string")));
    }

    let (input, guid_data) = take(size_of::<u128>())(input)?;

    Ok((input, format_guid_le_bytes(guid_data)))
}

#[cfg(test)]
mod tests {
    use super::parse_beef;

    #[test]
    fn test_parse_beef() {
        let test_data = [
            42, 0, 0, 0, 0, 0, 239, 190, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 32, 0,
        ];
        let (_, result) = parse_beef(&test_data).unwrap();

        assert_eq!(result, "20000000-0000-0000-0000-000000000000")
    }
}
