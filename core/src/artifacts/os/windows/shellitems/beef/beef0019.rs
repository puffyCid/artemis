use crate::utils::uuid::format_guid_le_bytes;
use nom::bytes::complete::take;
use std::mem::size_of;

/// Parse a `0xbeef0019` block. Contains a GUID
pub(crate) fn parse_beef(data: &[u8]) -> nom::IResult<&[u8], String> {
    let (input, _size_data) = take(size_of::<u16>())(data)?;
    let (input, _extension) = take(size_of::<u16>())(input)?;
    let (input, _signature) = take(size_of::<u32>())(input)?;

    let (input, folder_guid) = take(size_of::<u128>())(input)?;
    let (input, _unknown_guid) = take(size_of::<u128>())(input)?;
    let (input, _extension_offset) = take(size_of::<u16>())(input)?;

    Ok((input, format_guid_le_bytes(folder_guid)))
}

#[cfg(test)]
mod tests {
    use super::parse_beef;

    #[test]
    fn test_parse_beef() {
        let test_data = [
            42, 0, 0, 0, 25, 0, 239, 190, 126, 71, 179, 251, 228, 201, 59, 75, 162, 186, 211, 245,
            211, 205, 70, 249, 130, 7, 186, 130, 122, 91, 105, 69, 181, 215, 236, 131, 8, 95, 8,
            204, 32, 0,
        ];
        let (_, result) = parse_beef(&test_data).unwrap();

        assert_eq!(result, "fbb3477e-c9e4-4b3b-a2ba-d3f5d3cd46f9")
    }
}
