use crate::utils::nom_helper::{nom_unsigned_four_bytes, nom_unsigned_one_byte, Endian};

pub(crate) struct B5 {
    sig: u8,
    record_entry_size: u8,
    record_value_size: u8,
    level: u8,
    entries_reference: u32,
}

pub(crate) fn parse_b5(data: &[u8]) -> nom::IResult<&[u8], B5> {
    let (input, sig) = nom_unsigned_one_byte(data, Endian::Le)?;
    let (input, record_entry_size) = nom_unsigned_one_byte(input, Endian::Le)?;
    let (input, record_value_size) = nom_unsigned_one_byte(input, Endian::Le)?;
    let (input, level) = nom_unsigned_one_byte(input, Endian::Le)?;
    let (input, entries_reference) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let table = B5 {
        sig,
        record_entry_size,
        record_value_size,
        level,
        entries_reference,
    };
    Ok((input, table))
}

#[cfg(test)]
mod tests {
    use super::parse_b5;

    #[test]
    fn test_parse_b5() {
        let test = [181, 4, 4, 0, 0, 0, 0, 0];
        let (_, result) = parse_b5(&test).unwrap();
        assert_eq!(result.sig, 181);
        assert_eq!(result.record_value_size, 4);
        assert_eq!(result.record_entry_size, 4);
        assert_eq!(result.level, 0);
        assert_eq!(result.entries_reference, 0);
    }
}
