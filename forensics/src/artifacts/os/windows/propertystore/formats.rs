use super::properties::{numeric::parse_numeric, string::parse_string};
use log::warn;
use serde_json::Value;
use std::collections::HashMap;

/// Parse support `Property Store` formats
pub(crate) fn parse_formats<'a>(
    data: &'a [u8],
    guid: &str,
    count: &mut u32,
) -> nom::IResult<&'a [u8], HashMap<String, Value>> {
    // List at https://github.com/libyal/libfwps/blob/main/documentation/Windows%20Property%20Store%20format.asciidoc
    // Additional entries at https://github.com/EricZimmerman/ExtensionBlocks/blob/master/ExtensionBlocks/Utils.cs
    let (input, result) = if guid == "d5cdd505-2e9c-101b-9397-08002b2cf9ae" {
        parse_string(data)?
    } else {
        // All other GUIDs should use numeric parsers. Per libfwps docs
        let numeric_result = parse_numeric(data, count);
        let (input, result) = match numeric_result {
            Ok(result) => result,
            Err(_err) => {
                warn!("[propertystore] Failed to parse Property GUID: {guid}");
                return Ok((data, HashMap::new()));
            }
        };
        (input, result)
    };

    Ok((input, result))
}

#[cfg(test)]
mod tests {
    use super::parse_formats;
    use crate::filesystem::files::read_file;
    use std::path::PathBuf;

    #[test]
    fn test_parse_formats() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/propertystores/win11/property.raw");

        let results = read_file(&test_location.display().to_string()).unwrap();
        let (_, prop_result) =
            parse_formats(&results, "d5cdd505-2e9c-101b-9397-08002b2cf9ae", &mut 0).unwrap();
        assert_eq!(prop_result.len(), 4);

        assert_eq!(
            prop_result.get("AutoCacheKey").unwrap().as_str().unwrap(),
            "Search Results in System320"
        )
    }
}
