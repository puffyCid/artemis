use crate::utils::nom_helper::{nom_signed_four_bytes, nom_unsigned_four_bytes, Endian};

/// Get provider info from `WEVT_TEMPLATE`
pub(crate) fn parse_provider(data: &[u8]) -> nom::IResult<&[u8], Vec<u32>> {
    let (input, _sig) = nom_unsigned_four_bytes(data, Endian::Le)?;
    // Size includes sig and size itself
    let (input, _size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    // -1 if not set
    let (input, _message_id) = nom_signed_four_bytes(input, Endian::Le)?;
    let (input, provider_count) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (mut input, _unknown_count) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let adjust_count = 1;

    let mut count = 0;
    let mut offsets = Vec::new();
    while count < provider_count - adjust_count {
        let (remaining, element_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (remaining, _unknown) = nom_unsigned_four_bytes(remaining, Endian::Le)?;

        input = remaining;
        count += 1;

        // Will need to loop through and jump to each offset
        offsets.push(element_offset);
    }

    let (input, last_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
    offsets.push(last_offset);

    Ok((input, offsets))
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::eventlogs::resources::manifest::provider::parse_provider;

    #[test]
    fn test_parse_provider() {
        let test = [
            87, 69, 86, 84, 172, 56, 0, 0, 255, 255, 255, 255, 8, 0, 0, 0, 5, 0, 0, 0, 116, 0, 0,
            0, 7, 0, 0, 0, 204, 1, 0, 0, 13, 0, 0, 0, 4, 48, 0, 0, 2, 0, 0, 0, 80, 48, 0, 0, 0, 0,
            0, 0, 120, 49, 0, 0, 1, 0, 0, 0, 220, 49, 0, 0, 3, 0, 0, 0, 20, 50, 0, 0, 4, 0, 0, 0,
            88, 53, 0, 0,
        ];

        let (_, offsets) = parse_provider(&test).unwrap();
        assert_eq!(offsets.len(), 8);
    }
}
