use crate::utils::{
    encoding::base64_encode_standard, strings::extract_utf8_string, uuid::format_guid_be_bytes,
};
use nom::bytes::complete::take;
use serde_json::{Value, json};

/// Extract binary info associated with Spotlight property. This function will detect two (2) binary props and extract the data
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

#[cfg(test)]
mod tests {
    use super::extract_binary;
    use crate::filesystem::files::read_file;
    use std::path::PathBuf;

    #[test]
    fn test_extract_binary() {
        let size = 2691;
        let name = "kMDStoreProperties";

        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/spotlight/bigsur/binary.raw");
        let data = read_file(test_location.to_str().unwrap()).unwrap();

        let (_, result) = extract_binary(&data, &size, name).unwrap();
        assert_eq!(result.as_str().unwrap().len(), 2691);
    }
}
