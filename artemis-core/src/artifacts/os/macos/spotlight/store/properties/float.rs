use crate::artifacts::os::macos::spotlight::store::property::parse_variable_size;
use nom::{
    bytes::complete::take,
    number::complete::{le_f32, le_f64},
};
use serde_json::{json, Value};
use std::mem::size_of;

pub(crate) fn extract_float32<'a>(data: &'a [u8], prop_type: &u8) -> nom::IResult<&'a [u8], Value> {
    let mut floats = Vec::new();

    let multiple_floats = 2;
    if (prop_type & multiple_floats) == multiple_floats {
        let (mut input, multi_floats) = parse_variable_size(data)?;
        let num_values = multi_floats / 4;
        let mut count = 0;
        while count < num_values {
            let (remaining, float_data) = take(size_of::<f32>())(data)?;
            let (_, float) = le_f32(float_data)?;
            input = remaining;
            count += 1;

            floats.push(float);
        }
        panic!("multi floats32");
        return Ok((input, json!(floats)));
    }
    let (input, float_data) = take(size_of::<f32>())(data)?;
    let (_, float) = le_f32(float_data)?;

    floats.push(float);

    Ok((input, json!(floats)))
}

pub(crate) fn extract_float64<'a>(data: &'a [u8], prop_type: &u8) -> nom::IResult<&'a [u8], Value> {
    let mut floats = Vec::new();

    let multiple_floats = 2;
    if (prop_type & multiple_floats) == multiple_floats {
        let (mut input, multi_floats) = parse_variable_size(data)?;
        let num_values = multi_floats / 8;
        let mut count = 0;
        while count < num_values {
            let (remaining, float_data) = take(size_of::<f64>())(data)?;
            let (_, float) = le_f64(float_data)?;
            input = remaining;
            count += 1;

            floats.push(float);
        }
        panic!("multi floats64");
        return Ok((input, json!(floats)));
    }
    let (input, float_data) = take(size_of::<f64>())(data)?;
    let (_, float) = le_f64(float_data)?;

    floats.push(float);

    Ok((input, json!(floats)))
}
