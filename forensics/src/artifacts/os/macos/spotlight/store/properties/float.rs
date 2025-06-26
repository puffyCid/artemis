use crate::artifacts::os::macos::spotlight::store::property::parse_variable_size;
use nom::{
    bytes::complete::take,
    number::complete::{le_f32, le_f64},
};
use serde_json::{Value, json};
use std::mem::size_of;

/// Extract 32-bit floats associated with Spotlight property
pub(crate) fn extract_float32(data: &[u8], prop_type: u8) -> nom::IResult<&[u8], Value> {
    let mut floats = Vec::new();

    let multiple_floats = 2;
    if (prop_type & multiple_floats) == multiple_floats {
        let (mut input, multi_floats) = parse_variable_size(data)?;
        let num_values = multi_floats / 4;
        let mut count = 0;
        while count < num_values {
            let (remaining, float_data) = take(size_of::<f32>())(input)?;
            let (_, float) = le_f32(float_data)?;
            input = remaining;
            count += 1;

            floats.push(float);
        }

        return Ok((input, json!(floats)));
    }
    let (input, float_data) = take(size_of::<f32>())(data)?;
    let (_, float) = le_f32(float_data)?;

    floats.push(float);

    Ok((input, json!(floats)))
}

/// Extract 64-bit floats associated with Spotlight property
pub(crate) fn extract_float64(data: &[u8], prop_type: u8) -> nom::IResult<&[u8], Value> {
    let mut floats = Vec::new();

    let multiple_floats = 2;
    if (prop_type & multiple_floats) == multiple_floats {
        let (mut input, multi_floats) = parse_variable_size(data)?;
        let num_values = multi_floats / 8;
        let mut count = 0;
        while count < num_values {
            let (remaining, float_data) = take(size_of::<f64>())(input)?;
            let (_, float) = le_f64(float_data)?;
            input = remaining;
            count += 1;

            floats.push(float);
        }

        return Ok((input, json!(floats)));
    }
    let (input, float_data) = take(size_of::<f64>())(data)?;
    let (_, float) = le_f64(float_data)?;

    floats.push(float);

    Ok((input, json!(floats)))
}

#[cfg(test)]
mod tests {
    use super::{extract_float32, extract_float64};

    #[test]
    fn test_extract_float32() {
        let prop_type = 64;
        let data = [1, 0, 0, 0, 0];
        let (_, result) = extract_float32(&data, prop_type).unwrap();
        assert_eq!(
            result.as_array().unwrap()[0].as_f64().unwrap(),
            1.401298464324817e-45
        );
    }

    #[test]
    fn test_extract_float64() {
        let prop_type = 64;
        let data = [1, 0, 0, 0, 0, 0, 0, 0, 0];
        let (_, result) = extract_float64(&data, prop_type).unwrap();
        assert_eq!(result.as_array().unwrap()[0].as_f64().unwrap(), 5e-324);
    }
}
