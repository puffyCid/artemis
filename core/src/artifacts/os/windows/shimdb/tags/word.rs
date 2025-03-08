use crate::utils::nom_helper::{Endian, nom_unsigned_two_bytes};

/// Pars WORD value and return as string
pub(crate) fn parse_word(data: &[u8]) -> nom::IResult<&[u8], String> {
    let (input, value) = nom_unsigned_two_bytes(data, Endian::Le)?;
    Ok((input, format!("{value}")))
}

#[cfg(test)]
mod tests {
    use super::parse_word;

    #[test]
    fn test_parse_word() {
        let test_data = [5, 0];
        let (_, result) = parse_word(&test_data).unwrap();
        assert_eq!(result, "5");
    }
}
