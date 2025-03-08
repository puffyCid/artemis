use crate::utils::nom_helper::{Endian, nom_unsigned_eight_bytes};

/// Grab the qword value but return it as a string
pub(crate) fn parse_qword(data: &[u8]) -> nom::IResult<&[u8], String> {
    let (input, value) = get_qword(data)?;
    Ok((input, format!("{value}")))
}

/// Grab the qword value from the tag
pub(crate) fn get_qword(data: &[u8]) -> nom::IResult<&[u8], u64> {
    let (input, value) = nom_unsigned_eight_bytes(data, Endian::Le)?;
    Ok((input, value))
}

#[cfg(test)]
mod tests {
    use super::{get_qword, parse_qword};

    #[test]
    fn test_get_qword() {
        let test_data = [5, 0, 0, 0, 0, 0, 0, 0];
        let (_, result) = get_qword(&test_data).unwrap();
        assert_eq!(result, 5);
    }

    #[test]
    fn test_parse_qword() {
        let test_data = [5, 0, 0, 0, 0, 0, 0, 0];
        let (_, result) = parse_qword(&test_data).unwrap();
        assert_eq!(result, "5");
    }
}
