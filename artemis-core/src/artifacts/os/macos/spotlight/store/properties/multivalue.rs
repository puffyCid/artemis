use crate::artifacts::os::macos::spotlight::store::property::parse_variable_size;
use serde_json::{json, Value};

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
        //panic!("multi values");
        return Ok((input, json!(multi_values)));
    }
    multi_values.push(multi_number);

    Ok((input, json!(multi_values)))
}
