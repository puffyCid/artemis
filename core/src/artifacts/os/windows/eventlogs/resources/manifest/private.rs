use crate::utils::nom_helper::{nom_unsigned_four_bytes, Endian};

/// Parse private data. Format is undocumented. Seem to just contain another Channel string
pub(crate) fn parse_private(data: &[u8]) -> nom::IResult<&[u8], ()> {
    let (input, sig) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (input, size) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let (input, size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let adjust_size = 8;

    // Size includes sig and size itself
    if adjust_size > size {
        // Should not happen
        return Ok((&[], ()));
    }

    return Ok((input, ()));
}

#[cfg(test)]
mod tests {
    use super::parse_private;

    #[test]
    fn test_parse_private() {
        let test = [
            80, 82, 86, 65, 76, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 16, 24, 48, 0, 0, 77, 0, 105, 0, 99,
            0, 114, 0, 111, 0, 115, 0, 111, 0, 102, 0, 116, 0, 45, 0, 87, 0, 105, 0, 110, 0, 100,
            0, 111, 0, 119, 0, 115, 0, 45, 0, 78, 0, 118, 0, 109, 0, 101, 0, 68, 0, 105, 0, 115, 0,
            107, 0, 0, 0, 0, 0,
        ];

        let _ = parse_private(&test).unwrap();
    }
}
