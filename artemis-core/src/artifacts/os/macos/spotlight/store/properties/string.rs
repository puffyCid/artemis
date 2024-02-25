use crate::utils::strings::extract_utf8_string;
use nom::bytes::complete::take;
use serde_json::{json, Value};

pub(crate) fn extract_string<'a>(data: &'a [u8], size: &usize) -> nom::IResult<&'a [u8], Value> {
    let (input, string_data) = take(*size)(data)?;
    let string = extract_utf8_string(string_data);

    Ok((input, json!(string)))
}
