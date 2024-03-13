use nom::bytes::complete::take;
use std::mem::size_of;

/// Parse a `0xbeef0013` block. This extension is undocumented
pub(crate) fn parse_beef(data: &[u8]) -> nom::IResult<&[u8], String> {
    let (input, _size_data) = take(size_of::<u16>())(data)?;
    let (input, _extension) = take(size_of::<u16>())(input)?;
    let (input, _signature) = take(size_of::<u32>())(input)?;

    let (input, _unknown_flags) = take(size_of::<u32>())(input)?;
    let unknown_size: u8 = 24;
    let (input, _unknown_data) = take(unknown_size)(input)?;
    let (input, _unknown) = take(size_of::<u32>())(input)?;
    let (input, _extension_offset) = take(size_of::<u16>())(input)?;

    Ok((input, String::new()))
}

#[cfg(test)]
mod tests {
    use super::parse_beef;

    #[test]
    fn test_parse_beef() {
        let test_data = [
            42, 0, 0, 0, 19, 0, 239, 190, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 139, 2, 0, 0,
        ];
        let (_, result) = parse_beef(&test_data).unwrap();

        assert_eq!(result, "")
    }
}
