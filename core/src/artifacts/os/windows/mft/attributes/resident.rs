use crate::utils::nom_helper::{
    nom_unsigned_four_bytes, nom_unsigned_one_byte, nom_unsigned_two_bytes, Endian,
};

#[derive(Debug)]
pub(crate) struct Resident {
    pub(crate) size: u32,
    _offset: u16,
    _indexed_flag: u8,
}

impl Resident {
    /// Parse Resident MFT metadata
    pub(crate) fn parse_resident(data: &[u8]) -> nom::IResult<&[u8], Resident> {
        let (input, size) = nom_unsigned_four_bytes(data, Endian::Le)?;
        let (input, offset) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, indexed_flag) = nom_unsigned_one_byte(input, Endian::Le)?;
        let (input, _padding) = nom_unsigned_one_byte(input, Endian::Le)?;

        let resident = Resident {
            size,
            _offset: offset,
            _indexed_flag: indexed_flag,
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
        assert_eq!(result._offset, 2826);
    }
}
