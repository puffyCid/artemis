use crate::{
    artifacts::os::windows::ole::types::parse_types,
    utils::{
        nom_helper::{
            nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_one_byte,
            nom_unsigned_two_bytes, Endian,
        },
        strings::extract_utf16_string,
        time::filetime_to_unixepoch,
    },
};
use log::error;
use nom::bytes::complete::{take, take_until};
use serde_json::{Number, Value};
use std::collections::HashMap;

/// Parse a `Property Store` stream
pub(crate) fn parse_string(data: &[u8]) -> nom::IResult<&[u8], HashMap<String, Value>> {
    let (input, size) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let adjust_size = 4;

    // Check to make sure size is large enough
    if size < adjust_size {
        return Ok((input, HashMap::new()));
    }

    // Size includes the size itself
    let (input, property_data) = take(size - adjust_size)(input)?;
    let (property_input, name_size) = nom_unsigned_four_bytes(property_data, Endian::Le)?;
    let (property_input, _reserved) = nom_unsigned_one_byte(property_input, Endian::Le)?;

    let (property_input, name_data) = take(name_size)(property_input)?;
    let (property_input, prop_type) = nom_unsigned_two_bytes(property_input, Endian::Le)?;
    let (property_input, _padding) = nom_unsigned_two_bytes(property_input, Endian::Le)?;

    let name = extract_utf16_string(name_data);
    let mut values = HashMap::new();

    let (_, items) = parse_types(property_input, &prop_type, &mut values, String::new())?;
    let serde_result = serde_json::to_value(items);
    let _ = match serde_result {
        Ok(results) => values.insert(name, results),
        Err(err) => {
            error!("[propertystore] Failed to serialize property store shellitems: {err:?}");
            Option::None
        }
    };

    let time_results = scan_cache_time(input);
    let _ = match time_results {
        Ok((_, result)) => values.insert(
            String::from("AutoCacheTime"),
            Value::Number(Number::from(result)),
        ),
        Err(_err) => Option::None,
    };

    let key_results = scan_cache_key(input);
    let _ = match key_results {
        Ok((_, result)) => values.insert(String::from("AutoCacheKey"), Value::String(result)),
        Err(_err) => Option::None,
    };

    Ok((input, values))
}

/// Scan `Property Store` bytes for cache time
fn scan_cache_time(data: &[u8]) -> nom::IResult<&[u8], i64> {
    // UTF16 string: AutoCacheTime
    let cache_time = [
        65, 0, 117, 0, 116, 0, 111, 0, 108, 0, 105, 0, 115, 0, 116, 0, 67, 0, 97, 0, 99, 0, 104, 0,
        101, 0, 84, 0, 105, 0, 109, 0, 101, 0, 0, 0,
    ];
    // Scan bytes for string
    let (input, _) = take_until(cache_time.as_slice())(data)?;
    let (input, _) = take(cache_time.len())(input)?;
    let (input, _unknown) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, filetime) = nom_unsigned_eight_bytes(input, Endian::Le)?;

    Ok((input, filetime_to_unixepoch(&filetime)))
}

/// Scan `Property Store` bytes for cache key
fn scan_cache_key(data: &[u8]) -> nom::IResult<&[u8], String> {
    // UTF16 string: AutoCacheKey
    let cache_key = [
        65, 0, 117, 0, 116, 0, 111, 0, 108, 0, 105, 0, 115, 0, 116, 0, 67, 0, 97, 0, 99, 0, 104, 0,
        101, 0, 75, 0, 101, 0, 121, 0, 0, 0,
    ];

    // Scan bytes for string
    let (input, _) = take_until(cache_key.as_slice())(data)?;
    let (input, _) = take(cache_key.len())(input)?;
    let (input, _entry_type) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, string_size) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let utf_adjust = 2;
    let (input, string_data) = take(string_size * utf_adjust)(input)?;

    Ok((input, extract_utf16_string(string_data)))
}

#[cfg(test)]
mod tests {
    use super::{parse_string, scan_cache_key, scan_cache_time};

    #[test]
    fn test_parse_string() {
        let test_data = [
            39, 2, 0, 0, 18, 0, 0, 0, 0, 65, 0, 117, 0, 116, 0, 111, 0, 76, 0, 105, 0, 115, 0, 116,
            0, 0, 0, 66, 0, 0, 0, 30, 0, 0, 0, 112, 0, 114, 0, 111, 0, 112, 0, 52, 0, 50, 0, 57, 0,
            52, 0, 57, 0, 54, 0, 55, 0, 50, 0, 57, 0, 53, 0, 0, 0, 0, 0, 221, 1, 0, 0, 174, 165,
            78, 56, 225, 173, 138, 78, 138, 155, 123, 234, 120, 255, 241, 233, 6, 0, 0, 128, 0, 0,
            0, 0, 1, 0, 0, 0, 2, 0, 0, 128, 1, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 32, 0, 0, 0, 0, 0,
            0, 0, 223, 0, 20, 0, 31, 80, 224, 79, 208, 32, 234, 58, 105, 16, 162, 216, 8, 0, 43,
            48, 48, 157, 25, 0, 47, 67, 58, 92, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 86, 0, 49, 0, 0, 0, 0, 0, 146, 82, 103, 139, 16, 0, 87, 105, 110, 100, 111, 119,
            115, 0, 64, 0, 9, 0, 4, 0, 239, 190, 135, 79, 119, 72, 171, 82, 196, 34, 46, 0, 0, 0,
            204, 110, 1, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 211, 130, 49, 0,
            87, 0, 105, 0, 110, 0, 100, 0, 111, 0, 119, 0, 115, 0, 0, 0, 22, 0, 90, 0, 49, 0, 0, 0,
            0, 0, 171, 82, 240, 34, 16, 32, 80, 114, 101, 102, 101, 116, 99, 104, 0, 0, 66, 0, 9,
            0, 4, 0, 239, 190, 36, 81, 114, 25, 171, 82, 242, 34, 46, 0, 0, 0, 246, 84, 0, 0, 0, 0,
            17, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 98, 34, 97, 0, 80, 0, 114, 0, 101, 0,
            102, 0, 101, 0, 116, 0, 99, 0, 104, 0, 0, 0, 24, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 128, 1, 0, 0, 0, 4, 0, 105, 0, 116,
            0, 101, 0, 109, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 30, 26, 222, 127, 49, 139, 165, 73,
            147, 184, 107, 225, 76, 250, 73, 67, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            255, 255, 0, 0, 0, 0, 1, 0, 0, 0, 26, 0, 83, 0, 101, 0, 97, 0, 114, 0, 99, 0, 104, 0,
            32, 0, 82, 0, 101, 0, 115, 0, 117, 0, 108, 0, 116, 0, 115, 0, 32, 0, 105, 0, 110, 0,
            32, 0, 80, 0, 114, 0, 101, 0, 102, 0, 101, 0, 116, 0, 99, 0, 104, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 57, 0, 0, 0,
            36, 0, 0, 0, 0, 65, 0, 117, 0, 116, 0, 111, 0, 108, 0, 105, 0, 115, 0, 116, 0, 67, 0,
            97, 0, 99, 0, 104, 0, 101, 0, 84, 0, 105, 0, 109, 0, 101, 0, 0, 0, 20, 0, 0, 0, 210,
            129, 225, 55, 158, 5, 0, 0, 107, 0, 0, 0, 34, 0, 0, 0, 0, 65, 0, 117, 0, 116, 0, 111,
            0, 108, 0, 105, 0, 115, 0, 116, 0, 67, 0, 97, 0, 99, 0, 104, 0, 101, 0, 75, 0, 101, 0,
            121, 0, 0, 0, 31, 0, 0, 0, 28, 0, 0, 0, 83, 0, 101, 0, 97, 0, 114, 0, 99, 0, 104, 0,
            32, 0, 82, 0, 101, 0, 115, 0, 117, 0, 108, 0, 116, 0, 115, 0, 32, 0, 105, 0, 110, 0,
            32, 0, 80, 0, 114, 0, 101, 0, 102, 0, 101, 0, 116, 0, 99, 0, 104, 0, 48, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 116, 26, 89, 94, 150, 223, 211, 72, 141, 103, 23, 51, 188, 238,
            40, 186, 103, 27, 115, 4, 51, 217, 10, 69, 144, 230, 74, 205, 46, 148, 8, 254, 42, 0,
            0, 0, 19, 0, 239, 190, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 31, 3, 42, 0, 0, 0, 25, 0, 239, 190, 30, 26, 222, 127,
            49, 139, 165, 73, 147, 184, 107, 225, 76, 250, 73, 67, 111, 115, 190, 189, 245, 52, 41,
            72, 171, 232, 181, 80, 230, 81, 70, 196, 31, 3, 0, 0,
        ];
        let (_, result) = parse_string(&test_data).unwrap();
        assert_eq!(result.get("AutoList").unwrap().as_array().unwrap().len(), 4);
    }

    #[test]
    fn test_scan_cache_key() {
        let test = [
            0, 0, 65, 0, 117, 0, 116, 0, 111, 0, 108, 0, 105, 0, 115, 0, 116, 0, 67, 0, 97, 0, 99,
            0, 104, 0, 101, 0, 84, 0, 105, 0, 109, 0, 101, 0, 0, 0, 20, 0, 0, 0, 210, 129, 225, 55,
            158, 5, 0, 0, 107, 0, 0, 0, 34, 0, 0, 0, 0, 65, 0, 117, 0, 116, 0, 111, 0, 108, 0, 105,
            0, 115, 0, 116, 0, 67, 0, 97, 0, 99, 0, 104, 0, 101, 0, 75, 0, 101, 0, 121, 0, 0, 0,
            31, 0, 0, 0, 28, 0, 0, 0, 83, 0, 101, 0, 97, 0, 114, 0, 99, 0, 104, 0, 32, 0, 82, 0,
            101, 0, 115, 0, 117, 0, 108, 0, 116, 0, 115, 0, 32, 0, 105, 0, 110, 0, 32, 0, 80, 0,
            114, 0, 101, 0, 102, 0, 101, 0, 116, 0, 99, 0, 104, 0, 48, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 116, 26, 89, 94, 150, 223, 211, 72, 141, 103, 23, 51, 188, 238, 40, 186,
            103, 27, 115, 4, 51, 217, 10, 69, 144, 230, 74, 205, 46, 148, 8, 254, 42, 0, 0, 0, 19,
            0, 239, 190, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 1, 0, 0, 0, 31, 3, 42, 0, 0, 0, 25, 0, 239, 190, 30, 26, 222, 127, 49, 139,
            165, 73, 147, 184, 107, 225, 76, 250, 73,
        ];

        let (_, result) = scan_cache_key(&test).unwrap();
        assert_eq!(result, "Search Results in Prefetch0")
    }

    #[test]
    fn test_scan_cache_time() {
        let test = [
            0, 65, 0, 117, 0, 116, 0, 111, 0, 108, 0, 105, 0, 115, 0, 116, 0, 67, 0, 97, 0, 99, 0,
            104, 0, 101, 0, 84, 0, 105, 0, 109, 0, 101, 0, 0, 0, 20, 0, 0, 0, 210, 129, 225, 55,
            158, 5, 0, 0, 107, 0, 0, 0, 34, 0, 0, 0, 0, 65, 0, 117, 0, 116, 0, 111, 0, 108, 0, 105,
            0, 115, 0,
        ];

        let (_, result) = scan_cache_time(&test).unwrap();
        assert_eq!(result, -11643855890);
    }
}
