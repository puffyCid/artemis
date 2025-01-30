use crate::utils::{
    encoding::base64_encode_standard,
    nom_helper::{nom_unsigned_four_bytes, nom_unsigned_two_bytes, Endian},
};
use nom::bytes::complete::take;
use serde::Serialize;

// Could parse even further based on https://github.com/libyal/libfsntfs/blob/main/documentation/New%20Technologies%20File%20System%20(NTFS).asciidoc#10-the-reparse-point\
#[derive(Debug, Serialize)]
pub(crate) struct ReparsePoint {
    tag: u32,
    size: u16,
    data: String,
}

impl ReparsePoint {
    /// Parse reparse attribute
    pub(crate) fn parse_reparse(data: &[u8]) -> nom::IResult<&[u8], ReparsePoint> {
        let (input, tag) = nom_unsigned_four_bytes(data, Endian::Le)?;
        let (input, size) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, _reserved) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, reparse_data) = take(size)(input)?;

        let point = ReparsePoint {
            tag,
            size,
            data: base64_encode_standard(reparse_data),
        };

        Ok((input, point))
    }
}

#[cfg(test)]
mod tests {
    use super::ReparsePoint;

    #[test]
    fn test_parse_reparse() {
        let test = [
            23, 0, 0, 128, 16, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0,
        ];
        let (_, result) = ReparsePoint::parse_reparse(&test).unwrap();
        assert_eq!(result.data, "AQAAAAIAAAABAAAAAgAAAA==");
        assert_eq!(result.size, 16);
        assert_eq!(result.tag, 2147483671);
    }
}
