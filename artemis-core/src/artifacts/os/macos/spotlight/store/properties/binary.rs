use crate::utils::{
    encoding::base64_encode_standard, strings::extract_utf8_string, uuid::format_guid_be_bytes,
};
use nom::bytes::complete::take;
use serde_json::{json, Value};

pub(crate) fn extract_binary<'a>(
    data: &'a [u8],
    size: &usize,
    name: &str,
) -> nom::IResult<&'a [u8], Value> {
    let (input, string_data) = take(*size)(data)?;

    let string = if name == "kMDStoreProperties" {
        extract_utf8_string(string_data)
    } else if name == "kMDStoreUUID" {
        // Contains GUID
        format_guid_be_bytes(string_data)
    } else {
        // Otherwise we do not know what the binary data is
        base64_encode_standard(string_data)
    };

    Ok((input, json!(string)))
}
