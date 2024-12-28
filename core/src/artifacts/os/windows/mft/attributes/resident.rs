use crate::utils::nom_helper::{
    nom_unsigned_four_bytes, nom_unsigned_one_byte, nom_unsigned_two_bytes, Endian,
};

pub(crate) struct Resident {
    size: u32,
    offset: u16,
    indexed_flag: u8,
}

impl Resident {
    pub(crate) fn parse_resident(data: &[u8]) -> nom::IResult<&[u8], Resident> {
        let (input, size) = nom_unsigned_four_bytes(data, Endian::Le)?;
        let (input, offset) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, indexed_flag) = nom_unsigned_one_byte(input, Endian::Le)?;
        let (input, _padding) = nom_unsigned_one_byte(input, Endian::Le)?;

        let resident = Resident {
            size,
            offset,
            indexed_flag,
        };

        Ok((input, resident))
    }
}

#[cfg(test)]
mod tests {
    use super::Resident;

    #[test]
    fn test_parse_resident() {
        let test = [1, 0, 0, 0, 10, 11, 1, 0];
        let (_, result) = Resident::parse_resident(&test).unwrap();
        assert_eq!(result.offset, 2826);
    }
}
