use crate::artifacts::os::macos::spotlight::store::property::parse_variable_size;
use serde_json::{json, Value};

pub(crate) fn extract_byte<'a>(data: &'a [u8]) -> nom::IResult<&'a [u8], Value> {
    let (input, value) = parse_variable_size(data)?;

    Ok((input, json!(value)))
}

pub(crate) fn extract_bool<'a>(data: &'a [u8]) -> nom::IResult<&'a [u8], Value> {
    let (input, value) = parse_variable_size(data)?;

    let bool = if value != 0 { true } else { false };

    Ok((input, json!(bool)))
}
