use crate::utils::nom_helper::{Endian, nom_unsigned_four_bytes};

/// Grab the dword value but return it as a string
pub(crate) fn parse_dword(data: &[u8]) -> nom::IResult<&[u8], String> {
    let (input, value) = get_dword(data)?;
    Ok((input, format!("{value}")))
}

/// Grab the dword value from the tag
pub(crate) fn get_dword(data: &[u8]) -> nom::IResult<&[u8], u32> {
    let (input, value) = nom_unsigned_four_bytes(data, Endian::Le)?;
    Ok((input, value))
}

#[cfg(test)]
mod tests {
    use super::{get_dword, parse_dword};

    #[test]
    fn test_get_dword() {
        let test_data = [5, 0, 0, 0];
        let (_, result) = get_dword(&test_data).unwrap();
        assert_eq!(result, 5);
    }

    #[test]
    fn test_parse_dword() {
        let test_data = [5, 0, 0, 0];
        let (_, result) = parse_dword(&test_data).unwrap();
        assert_eq!(result, "5");
    }
}
