use crate::utils::strings::extract_utf8_string;
use nom::bytes::complete::take;
use serde_json::{Value, json};

/// Extract string associated with Spotlight property
pub(crate) fn extract_string(data: &[u8], size: usize) -> nom::IResult<&[u8], Value> {
    let (input, string_data) = take(size)(data)?;
    let string = extract_utf8_string(string_data);

    Ok((input, json!(string)))
}

#[cfg(test)]
mod tests {
    use super::extract_string;

    #[test]
    fn test_extract_string() {
        let data = [79, 83, 81, 85, 69, 82, 89, 68, 46, 69, 88, 69, 0];
        let (_, result) = extract_string(&data, 13).unwrap();
        assert_eq!(result.as_str().unwrap(), "OSQUERYD.EXE");
    }
}
