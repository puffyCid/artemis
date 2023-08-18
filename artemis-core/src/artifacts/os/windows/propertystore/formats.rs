use super::properties::{numeric::parse_numeric, string::parse_string};
use log::error;
use nom::error::ErrorKind;
use serde_json::Value;
use std::collections::HashMap;

/// Parse support PropertyStore formats
pub(crate) fn parse_formats<'a>(
    data: &'a [u8],
    guid: &str,
) -> nom::IResult<&'a [u8], HashMap<String, Value>> {
    // List at https://github.com/libyal/libfwps/blob/main/documentation/Windows%20Property%20Store%20format.asciidoc
    // Additional entries at https://github.com/EricZimmerman/ExtensionBlocks/blob/master/ExtensionBlocks/Utils.cs
    let (input, result) = match guid {
        "000214a1-0000-0000-c000-000000000046" => parse_numeric(data)?,
        "01a3057a-74d6-4e80-bea7-dc4c212ce50a" => parse_numeric(data)?,
        "0ded77b3-c614-456c-ae5b-285b38d7b01b" => parse_numeric(data)?,
        "28636aa6-953d-11d2-b5d6-00c04fd918d0" => parse_numeric(data)?,
        "446d16b1-8dad-4870-a748-402ea43d788c" => parse_numeric(data)?,
        "46588ae2-4cbc-4338-bbfc-139326986dce" => parse_numeric(data)?,
        "4d545058-4fce-4578-95c8-8698a9bc0f49" => parse_numeric(data)?,
        "56a3372e-ce9c-11d2-9f0e-006097c686f6" => parse_numeric(data)?,
        "6444048f-4c8b-11d1-8b70-080036b11a03" => parse_numeric(data)?,
        "64440490-4c8b-11d1-8b70-080036b11a03" => parse_numeric(data)?,
        "64440491-4c8b-11d1-8b70-080036b11a03" => parse_numeric(data)?,
        "64440492-4c8b-11d1-8b70-080036b11a03" => parse_numeric(data)?,
        "841e4f90-ff59-4d16-8947-e81bbffab36d" => parse_numeric(data)?,
        "86d40b4d-9069-443c-819a-2a54090dccec" => parse_numeric(data)?,
        "8f052d93-abca-4fc5-a5ac-b01df4dbe598" => parse_numeric(data)?,
        "9f4c2855-9f79-4b39-a8d0-e1d42de1d5f3" => parse_numeric(data)?,
        "b725f130-47ef-101a-a5f1-02608c9eebac" => parse_numeric(data)?,
        "d5cdd502-2e9c-101b-9397-08002b2cf9ae" => parse_numeric(data)?,
        "d5cdd505-2e9c-101b-9397-08002b2cf9ae" => parse_string(data)?,
        "ef6b490d-5cd8-437a-affc-da8b60ee4a3c" => parse_numeric(data)?,
        "f29f85e0-4ff9-1068-ab91-08002b27b3d9" => parse_numeric(data)?,
        "fb8d2d7b-90d1-4e34-bf60-6eac09922bbf" => parse_numeric(data)?,
        "49691c90-7e17-101a-a91c-08002b2ecda9" => parse_numeric(data)?,
        "0ae54373-43be-4fad-85e4-69dc8633986e" => parse_numeric(data)?,
        _ => {
            panic!("[propertystore] Unknown Property format: {guid}");
            return Err(nom::Err::Failure(nom::error::Error::new(
                data,
                ErrorKind::Fail,
            )));
        }
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
            parse_formats(&results, "d5cdd505-2e9c-101b-9397-08002b2cf9ae").unwrap();
        assert_eq!(prop_result.len(), 4);

        assert_eq!(
            prop_result.get("AutoCacheKey").unwrap().as_str().unwrap(),
            "Search Results in System320"
        )
    }
}
