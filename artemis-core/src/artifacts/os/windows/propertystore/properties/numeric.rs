use crate::{
    artifacts::os::windows::ole::types::parse_types,
    utils::nom_helper::{
        nom_unsigned_four_bytes, nom_unsigned_one_byte, nom_unsigned_two_bytes, Endian,
    },
};
use nom::bytes::complete::take;
use serde_json::Value;
use std::collections::HashMap;

/// Parse numeric propertystore type
pub(crate) fn parse_numeric(data: &[u8]) -> nom::IResult<&[u8], HashMap<String, Value>> {
    let (input, size) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let empty = 0;
    let adjust_size = 4;

    // Sometimes the property store is empty (has size zero (0)). Seen in Jumplists
    if size == empty || size < adjust_size {
        return Ok((input, HashMap::new()));
    }
    // Size includes size itself
    let (input, prop_data) = take(size - adjust_size)(input)?;
    let (prop_data, _entry_type) = nom_unsigned_four_bytes(prop_data, Endian::Le)?;
    let (prop_data, _padding) = nom_unsigned_one_byte(prop_data, Endian::Le)?;
    let (prop_data, prop_type) = nom_unsigned_two_bytes(prop_data, Endian::Le)?;
    let (prop_data, _padding) = nom_unsigned_two_bytes(prop_data, Endian::Le)?;

    let (_, (value, _)) = parse_types(prop_data, &prop_type)?;
    Ok((input, value))
}

#[cfg(test)]
mod tests {
    use super::parse_numeric;

    #[test]
    fn test_parse_numeric() {
        let test_data = [
            29, 0, 0, 0, 104, 0, 0, 0, 0, 72, 0, 0, 0, 129, 48, 105, 195, 194, 204, 140, 77, 128,
            223, 108, 13, 216, 242, 103, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let (_, value) = parse_numeric(&test_data).unwrap();
        assert_eq!(
            value.get("value").unwrap(),
            "c3693081-ccc2-4d8c-80df-6c0dd8f26709"
        );
    }
}
