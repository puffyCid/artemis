use crate::artifacts::os::macos::spotlight::store::property::parse_variable_size;
use serde_json::{Value, json};

/// Extract multivalue data associated with Spotlight property
pub(crate) fn extract_multivalue<'a>(
    data: &'a [u8],
    prop_type: &u8,
) -> nom::IResult<&'a [u8], Value> {
    let mut multi_values = Vec::new();
    let (mut input, multi_number) = parse_variable_size(data)?;

    let multiple_values = 2;
    if (prop_type & multiple_values) == multiple_values {
        let num_values = multi_number >> 3;
        let mut count = 0;
        while count < num_values {
            let (remaining, value) = parse_variable_size(input)?;
            input = remaining;
            count += 1;

            multi_values.push(value);
        }
        return Ok((input, json!(multi_values)));
    }
    multi_values.push(multi_number);

    Ok((input, json!(multi_values)))
}

#[cfg(test)]
mod tests {
    use super::extract_multivalue;

    #[test]
    fn test_extract_multivalue() {
        let data = [
            0, 2, 5, 98, 111, 111, 116, 0, 18, 7, 2, 0, 0, 0, 192, 203, 201, 195, 65, 2, 20, 1, 0,
            1, 79, 3, 0, 0, 0, 192, 203, 201, 195, 65, 1, 122, 182, 108, 247, 224, 201, 195, 65, 1,
            168, 158, 69, 247, 224, 201, 195, 65, 1, 0, 1, 0, 3, 0, 1, 0, 1, 122, 182, 108, 247,
            224, 201, 195, 65, 2, 0, 1, 0, 0, 0, 192, 203, 201, 195, 65, 2, 169, 158, 69, 247, 224,
            201, 195, 65, 2, 169, 158, 69, 247, 224, 201, 195, 65, 1, 122, 182, 108, 247, 224, 201,
            195, 65, 1, 0, 0, 0, 192, 203, 201, 195, 65, 4, 7, 98, 111, 111, 116, 22, 2, 0, 1, 7,
            98, 111, 111, 116, 22, 2, 0,
        ];
        let prop_type = 76;

        let (_, result) = extract_multivalue(&data, &prop_type).unwrap();
        assert_eq!(result.as_array().unwrap()[0].as_u64().unwrap(), 0);
    }
}
