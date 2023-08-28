use super::formats::parse_formats;
use crate::utils::nom_helper::{nom_unsigned_four_bytes, Endian};
use crate::utils::uuid::format_guid_le_bytes;
use nom::bytes::complete::{take, take_until};
use nom::number::complete::le_u32;
use nom::Needed;
use serde_json::Value;
use std::collections::HashMap;
use std::mem::size_of;

/// Parse the property store, only getting the first GUID right now
pub(crate) fn parse_property_store(
    data: &[u8],
) -> nom::IResult<&[u8], Vec<HashMap<String, Value>>> {
    let mut remaining_data = data;
    let end = [0, 0, 0, 0];

    let mut stores = Vec::new();
    while !remaining_data.is_empty() && !remaining_data.starts_with(&end) {
        let version_sig = [49, 83, 80, 83];
        let (input, size_data) = take_until(version_sig.as_slice())(remaining_data)?;

        let property_size = 4;
        if size_data.len() != property_size {
            break;
        }

        let (_, data_size) = le_u32(size_data)?;
        if data_size < property_size as u32 {
            return Err(nom::Err::Incomplete(Needed::Unknown));
        }
        let (remaining, input) = take(data_size as usize - property_size)(input)?;

        remaining_data = remaining;

        let (input, _version) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, guid_data) = take(size_of::<u128>())(input)?;

        let (_, store) = parse_formats(input, &format_guid_le_bytes(guid_data))?;
        if store.is_empty() {
            continue;
        }
        stores.push(store);
    }

    Ok((remaining_data, stores))
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::propertystore::store::parse_property_store,
        filesystem::files::read_file,
    };
    use std::path::PathBuf;

    #[test]
    fn test_parse_property_store() {
        let test_data = [
            231, 2, 0, 0, 49, 83, 80, 83, 5, 213, 205, 213, 156, 46, 27, 16, 147, 151, 8, 0, 43,
            44, 249, 174, 39, 2, 0, 0, 18, 0, 0, 0, 0, 65, 0, 117, 0, 116, 0, 111, 0, 76, 0, 105,
            0, 115, 0, 116, 0, 0, 0, 66, 0, 0, 0, 30, 0, 0, 0, 112, 0, 114, 0, 111, 0, 112, 0, 52,
            0, 50, 0, 57, 0, 52, 0, 57, 0, 54, 0, 55, 0, 50, 0, 57, 0, 53, 0, 0, 0, 0, 0, 221, 1,
            0, 0, 174, 165, 78, 56, 225, 173, 138, 78, 138, 155, 123, 234, 120, 255, 241, 233, 6,
            0, 0, 128, 0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 128, 1, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0,
            32, 0, 0, 0, 0, 0, 0, 0, 223, 0, 20, 0, 31, 80, 224, 79, 208, 32, 234, 58, 105, 16,
            162, 216, 8, 0, 43, 48, 48, 157, 25, 0, 47, 67, 58, 92, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 86, 0, 49, 0, 0, 0, 0, 0, 183, 80, 11, 162, 16, 0, 87, 105,
            110, 100, 111, 119, 115, 0, 64, 0, 9, 0, 4, 0, 239, 190, 115, 78, 172, 36, 183, 80, 11,
            162, 46, 0, 0, 0, 87, 146, 1, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            244, 161, 252, 0, 87, 0, 105, 0, 110, 0, 100, 0, 111, 0, 119, 0, 115, 0, 0, 0, 22, 0,
            90, 0, 49, 0, 0, 0, 0, 0, 203, 80, 102, 10, 16, 0, 83, 121, 115, 116, 101, 109, 51, 50,
            0, 0, 66, 0, 9, 0, 4, 0, 239, 190, 115, 78, 172, 36, 203, 80, 102, 10, 46, 0, 0, 0,
            147, 155, 1, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 133, 88, 172, 0,
            83, 0, 121, 0, 115, 0, 116, 0, 101, 0, 109, 0, 51, 0, 50, 0, 0, 0, 24, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 128, 1, 0,
            0, 0, 4, 0, 105, 0, 116, 0, 101, 0, 109, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 30, 26,
            222, 127, 49, 139, 165, 73, 147, 184, 107, 225, 76, 250, 73, 67, 255, 255, 255, 255,
            255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 0, 1, 0, 0, 0, 26, 0, 83, 0, 101, 0,
            97, 0, 114, 0, 99, 0, 104, 0, 32, 0, 82, 0, 101, 0, 115, 0, 117, 0, 108, 0, 116, 0,
            115, 0, 32, 0, 105, 0, 110, 0, 32, 0, 83, 0, 121, 0, 115, 0, 116, 0, 101, 0, 109, 0,
            51, 0, 50, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 57, 0, 0, 0, 36, 0, 0, 0, 0, 65, 0, 117, 0, 116, 0, 111, 0, 108,
            0, 105, 0, 115, 0, 116, 0, 67, 0, 97, 0, 99, 0, 104, 0, 101, 0, 84, 0, 105, 0, 109, 0,
            101, 0, 0, 0, 20, 0, 0, 0, 149, 78, 49, 203, 24, 0, 0, 0, 107, 0, 0, 0, 34, 0, 0, 0, 0,
            65, 0, 117, 0, 116, 0, 111, 0, 108, 0, 105, 0, 115, 0, 116, 0, 67, 0, 97, 0, 99, 0,
            104, 0, 101, 0, 75, 0, 101, 0, 121, 0, 0, 0, 31, 0, 0, 0, 28, 0, 0, 0, 83, 0, 101, 0,
            97, 0, 114, 0, 99, 0, 104, 0, 32, 0, 82, 0, 101, 0, 115, 0, 117, 0, 108, 0, 116, 0,
            115, 0, 32, 0, 105, 0, 110, 0, 32, 0, 83, 0, 121, 0, 115, 0, 116, 0, 101, 0, 109, 0,
            51, 0, 50, 0, 48, 0, 0, 0, 0, 0, 0, 0,
        ];

        let (_, result) = parse_property_store(&test_data).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].get("AutoCacheKey").unwrap(),
            "Search Results in System320"
        );
    }

    #[test]
    fn test_parse_formats() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location
            .push("tests/test_data/windows/propertystores/win11/multiplepropertystores.raw");

        let results = read_file(&test_location.display().to_string()).unwrap();
        let (_, prop_results) = parse_property_store(&results).unwrap();
        assert_eq!(prop_results.len(), 5);
    }
}
