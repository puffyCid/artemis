/**
 * When parsing binary formats often we parse X bytes and convert bytes to a number
 * With nom we can do that in two steps, ex:  
 *   `take X bytes`  
 *   `le_uX` to number
 *
 * These functions help reduce the repetitiveness of converting bytes to a number
 */
use nom::{
    bytes::complete::take,
    number::complete::{
        be_i16, be_i32, be_i64, be_u8, be_u16, be_u32, be_u64, be_u128, le_i16, le_i32, le_i64,
        le_u8, le_u16, le_u32, le_u64, le_u128,
    },
};
use std::mem::size_of;

pub(crate) enum Endian {
    /**Little Endian */
    Le,
    /**Big Endian */
    Be,
}

/**
 * Nom four (4) bytes to u32
 * Need to specify Endianess
 */
pub(crate) fn nom_unsigned_four_bytes(data: &[u8], endian: Endian) -> nom::IResult<&[u8], u32> {
    let (input, value_data) = take(size_of::<u32>())(data)?;

    let (_, value) = match endian {
        Endian::Le => le_u32(value_data)?,
        Endian::Be => be_u32(value_data)?,
    };

    Ok((input, value))
}

/**
 * Nom eight (8) bytes to u64
 * Need to specify Endianess
 */
pub(crate) fn nom_unsigned_eight_bytes(data: &[u8], endian: Endian) -> nom::IResult<&[u8], u64> {
    let (input, value_data) = take(size_of::<u64>())(data)?;

    let (_, value) = match endian {
        Endian::Le => le_u64(value_data)?,
        Endian::Be => be_u64(value_data)?,
    };
    Ok((input, value))
}

/**
 * Nom two (2) bytes to u16
 * Need to specify Endianess
 */
pub(crate) fn nom_unsigned_two_bytes(data: &[u8], endian: Endian) -> nom::IResult<&[u8], u16> {
    let (input, value_data) = take(size_of::<u16>())(data)?;

    let (_, value) = match endian {
        Endian::Le => le_u16(value_data)?,
        Endian::Be => be_u16(value_data)?,
    };
    Ok((input, value))
}

/**
 * Nom one (1) bytes to u8
 * Need to specify Endianess
 */
pub(crate) fn nom_unsigned_one_byte(data: &[u8], endian: Endian) -> nom::IResult<&[u8], u8> {
    let (input, value_data) = take(size_of::<u8>())(data)?;

    let (_, value) = match endian {
        Endian::Le => le_u8(value_data)?,
        Endian::Be => be_u8(value_data)?,
    };
    Ok((input, value))
}

/**
 * Nom sixteen (16) bytes to u128
 * Need to specify Endianess
 */
pub(crate) fn nom_unsigned_sixteen_bytes(data: &[u8], endian: Endian) -> nom::IResult<&[u8], u128> {
    let (input, value_data) = take(size_of::<u128>())(data)?;

    let (_, value) = match endian {
        Endian::Le => le_u128(value_data)?,
        Endian::Be => be_u128(value_data)?,
    };
    Ok((input, value))
}

/**
 * Nom four (4) bytes to i32
 * Need to specify Endianess
 */
pub(crate) fn nom_signed_four_bytes(data: &[u8], endian: Endian) -> nom::IResult<&[u8], i32> {
    let (input, value_data) = take(size_of::<u32>())(data)?;

    let (_, value) = match endian {
        Endian::Le => le_i32(value_data)?,
        Endian::Be => be_i32(value_data)?,
    };

    Ok((input, value))
}

/**
 * Nom eight (8) bytes to i64
 * Need to specify Endianess
 */
pub(crate) fn nom_signed_eight_bytes(data: &[u8], endian: Endian) -> nom::IResult<&[u8], i64> {
    let (input, value_data) = take(size_of::<u64>())(data)?;

    let (_, value) = match endian {
        Endian::Le => le_i64(value_data)?,
        Endian::Be => be_i64(value_data)?,
    };
    Ok((input, value))
}

/**
 * Nom two (2) bytes to i16
 * Need to specify Endianess
 */
pub(crate) fn nom_signed_two_bytes(data: &[u8], endian: Endian) -> nom::IResult<&[u8], i16> {
    let (input, value_data) = take(size_of::<u16>())(data)?;

    let (_, value) = match endian {
        Endian::Le => le_i16(value_data)?,
        Endian::Be => be_i16(value_data)?,
    };
    Ok((input, value))
}

/**
 * Nom an arbitrary amount of data and return the bytes remaining and bytes nom'd
 */
pub(crate) fn nom_data(data: &[u8], count: u64) -> nom::IResult<&[u8], &[u8]> {
    let (input, value) = take(count)(data)?;

    Ok((input, value))
}

#[cfg(test)]
mod tests {
    use crate::utils::nom_helper::{
        Endian, nom_data, nom_signed_eight_bytes, nom_signed_four_bytes, nom_signed_two_bytes,
        nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_one_byte,
        nom_unsigned_sixteen_bytes, nom_unsigned_two_bytes,
    };

    #[test]
    fn test_nom_signed_two_bytes() {
        let test = [2, 0];
        let (_, results) = nom_signed_two_bytes(&test, Endian::Le).unwrap();
        assert_eq!(results, 2);
    }

    #[test]
    fn test_nom_signed_eight_bytes() {
        let test = [2, 0, 0, 0, 0, 0, 0, 0];
        let (_, results) = nom_signed_eight_bytes(&test, Endian::Le).unwrap();
        assert_eq!(results, 2);
    }

    #[test]
    fn test_nom_signed_four_bytes() {
        let test = [2, 0, 0, 0];
        let (_, results) = nom_signed_four_bytes(&test, Endian::Le).unwrap();
        assert_eq!(results, 2);
    }

    #[test]
    fn test_nom_unsigned_sixteen_bytes() {
        let test = [2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let (_, results) = nom_unsigned_sixteen_bytes(&test, Endian::Le).unwrap();
        assert_eq!(results, 2);
    }

    #[test]
    fn test_nom_data() {
        let test = [2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let (_, results) = nom_data(&test, 3).unwrap();
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_nom_unsigned_four_bytes() {
        let test = [0, 0, 0, 2];
        let (_, results) = nom_unsigned_four_bytes(&test, Endian::Be).unwrap();
        assert_eq!(results, 2);
    }

    #[test]
    fn test_nom_unsigned_eight_bytes() {
        let test = [0, 0, 0, 0, 0, 0, 0, 2];
        let (_, results) = nom_unsigned_eight_bytes(&test, Endian::Be).unwrap();
        assert_eq!(results, 2);
    }

    #[test]
    fn test_nom_unsigned_one_byte() {
        let test = [2];
        let (_, results) = nom_unsigned_one_byte(&test, Endian::Be).unwrap();
        assert_eq!(results, 2);
    }

    #[test]
    fn test_nom_unsigned_two_bytes() {
        let test = [0, 2];
        let (_, results) = nom_unsigned_two_bytes(&test, Endian::Be).unwrap();
        assert_eq!(results, 2);
    }
}
