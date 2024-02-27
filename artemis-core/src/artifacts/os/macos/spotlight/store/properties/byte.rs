use crate::artifacts::os::macos::spotlight::store::property::parse_variable_size;
use serde_json::{json, Value};

/// Extra single byte associated with Spotlight property (appears to be rarely used)
pub(crate) fn extract_byte(data: &[u8]) -> nom::IResult<&[u8], Value> {
    let (input, value) = parse_variable_size(data)?;

    Ok((input, json!(value)))
}

/// Extra single boolean associated with Spotlight property
pub(crate) fn extract_bool(data: &[u8]) -> nom::IResult<&[u8], Value> {
    let (input, value) = parse_variable_size(data)?;

    let bool = value != 0;
    Ok((input, json!(bool)))
}

#[cfg(test)]
mod tests {
    use super::{extract_bool, extract_byte};

    #[test]
    fn test_extract_byte() {
        let data = [1, 0, 0, 0];
        let (_, result) = extract_byte(&data).unwrap();

        assert_eq!(result.as_u64().unwrap(), 1);
    }

    #[test]
    fn test_extract_bool() {
        let data = [1, 0, 0, 0];
        let (_, result) = extract_bool(&data).unwrap();

        assert!(result.as_bool().unwrap());
    }
}
