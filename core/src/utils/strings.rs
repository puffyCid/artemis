use crate::utils::encoding::base64_encode_standard;
use log::warn;
use std::string::FromUtf16Error;

/// Get a UTF16 string from provided bytes data. Will attempt to fix malformed UTF16. Such as UTF16 missing zeros
pub(crate) fn extract_utf16_string(data: &[u8]) -> String {
    let result = bytes_to_utf16_string(data, &false);
    match result {
        Ok(result) => result,
        Err(_err) => {
            // If we fail, try again with adjustment. Just incase it works.
            let result = bytes_to_utf16_string(data, &true);
            match result {
                Ok(result) => result,
                Err(err) => {
                    warn!("[strings] Failed to get UTF16 string: {err:?}");
                    base64_encode_standard(data)
                }
            }
        }
    }
}

/// Get a UTF16 string from provided bytes data
fn bytes_to_utf16_string(data: &[u8], adjust: &bool) -> Result<String, FromUtf16Error> {
    let mut utf16_data: Vec<u16> = Vec::new();
    // Convert data to UTF16 (&[u16])
    let min_byte_size = 2;
    for wide_char in data.chunks(min_byte_size) {
        if wide_char == vec![0, 0] || wide_char.len() < min_byte_size {
            // Check for last character
            if !wide_char.is_empty() && !wide_char.contains(&0) {
                utf16_data.push(wide_char[0] as u16);
            }
            break;
        }

        // If Wide char does not contain 0, append separately. This can probably be removed now?
        if !wide_char.contains(&0) && *adjust {
            utf16_data.push(wide_char[0] as u16);
            utf16_data.push(wide_char[1] as u16);
            continue;
        }
        if wide_char[0] == 0 {
            utf16_data.push(u16::from_ne_bytes([wide_char[1], wide_char[0]]));
            continue;
        }

        utf16_data.push(u16::from_ne_bytes([wide_char[0], wide_char[1]]));
    }

    // Windows uses UTF16
    let utf16_result = String::from_utf16(&utf16_data)?;

    Ok(utf16_result)
}

/// Get UTF16 strings that have new lines
pub(crate) fn extract_multiline_utf16_string(data: &[u8]) -> String {
    let mut utf16_data: Vec<u16> = Vec::new();
    let mut result = String::new();
    // Convert data to UTF16 (&[u16])
    let min_byte_size = 2;
    for wide_char in data.chunks(min_byte_size) {
        if wide_char.len() < min_byte_size {
            break;
        }

        if wide_char == vec![0, 0] {
            if utf16_data.is_empty() {
                continue;
            }
            // Windows uses UTF16
            let utf16_result = String::from_utf16(&utf16_data);
            let value = match utf16_result {
                Ok(results) => format!("{}\n", results.trim_matches('\0')),
                Err(err) => {
                    warn!("[strings] Failed to get UTF16 multi-line string: {err:?}");

                    let max_size = 2097152;
                    let issue = if data.len() < max_size {
                        base64_encode_standard(data)
                    } else {
                        format!("Binary data size larger than 2MB, size: {}", data.len())
                    };
                    format!("Failed to get UTF16 multi-line string: {}", issue)
                }
            };
            result = format!("{result}{value}");
            utf16_data.clear();
        }

        utf16_data.push(u16::from_ne_bytes([wide_char[0], wide_char[1]]));
    }

    result.trim().to_string()
}

/// Get a UTF8 string from provided bytes data. Invalid UTF8 is base64 encoded. Use `extract_uf8_string_lossy` if replacing bytes is acceptable
pub(crate) fn extract_utf8_string(data: &[u8]) -> String {
    let utf8_result = String::from_utf8(data.to_vec());
    match utf8_result {
        Ok(result) => result.trim_end_matches('\0').to_string(),
        Err(err) => {
            warn!("[strings] Failed to get UTF8 string: {err:?}");
            let max_size = 2097152;
            let issue = if data.len() < max_size {
                base64_encode_standard(data)
            } else {
                format!("Binary data size larger than 2MB, size: {}", data.len())
            };
            format!("Failed to get UTF8 string: {}", issue)
        }
    }
}

/// Get UTF8 string from provided bytes data. Invalid UTF8 will be replaced. Use `extract_utf8_string` if you do not want bytes replaced
pub(crate) fn extract_utf8_string_lossy(data: &[u8]) -> String {
    String::from_utf8_lossy(data).to_string()
}

/// Try to detect ASCII or UTF16 byte string
pub(crate) fn extract_ascii_utf16_string(data: &[u8]) -> String {
    if data.is_ascii() && data.iter().filter(|&c| *c == 0).count() <= 1 {
        extract_utf8_string(data)
    } else {
        extract_utf16_string(data)
    }
}

/// Check if either string contains the other
pub(crate) fn strings_contains(input1: &str, input2: &str) -> bool {
    if input1.contains(input2) || input2.contains(input1) {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use crate::{
        filesystem::files::read_file,
        utils::strings::{
            extract_ascii_utf16_string, extract_multiline_utf16_string, extract_utf16_string,
            extract_utf8_string, extract_utf8_string_lossy, strings_contains,
        },
    };
    use std::path::PathBuf;

    #[test]
    fn test_extract_utf16_string() {
        let test_data = vec![
            79, 0, 83, 0, 81, 0, 85, 0, 69, 0, 82, 0, 89, 0, 68, 0, 46, 0, 69, 0, 88, 0, 69, 0, 0,
            0,
        ];
        assert_eq!(extract_utf16_string(&test_data), "OSQUERYD.EXE")
    }

    #[test]
    fn test_extract_utf16_no_zeros() {
        let test_data = vec![
            75, 111, 110, 116, 114, 97, 115, 116, 32, 35, 49, 32, 40, 101, 120, 116, 114, 97, 103,
            114, 111, 223, 41,
        ];
        assert_eq!(extract_utf16_string(&test_data), "Kontrast #1 (extragroß)")
    }

    #[test]
    fn test_extract_multiline_utf16_string() {
        let test_data = vec![
            79, 0, 83, 0, 81, 0, 85, 0, 69, 0, 82, 0, 89, 0, 68, 0, 46, 0, 69, 0, 88, 0, 69, 0, 0,
            0, 79, 0, 83, 0, 81, 0, 85, 0, 69, 0, 82, 0, 89, 0, 68, 0, 46, 0, 69, 0, 88, 0, 69, 0,
            0, 0,
        ];
        assert_eq!(
            extract_multiline_utf16_string(&test_data),
            "OSQUERYD.EXE\nOSQUERYD.EXE"
        )
    }

    #[test]
    fn test_extract_utf8_string() {
        let test_data = vec![79, 83, 81, 85, 69, 82, 89, 68, 46, 69, 88, 69, 0];
        assert_eq!(extract_utf8_string(&test_data), "OSQUERYD.EXE")
    }

    #[test]
    fn test_extract_ascii_utf16_string() {
        let test_data = vec![79, 83, 81, 85, 69, 82, 89, 68, 46, 69, 88, 69, 0];
        assert_eq!(extract_ascii_utf16_string(&test_data), "OSQUERYD.EXE")
    }

    #[test]
    fn test_strings_contains() {
        let path1 = "a very long path";
        let path2 = "long path";
        let result = strings_contains(path1, path2);
        assert_eq!(result, true);
    }

    #[test]
    fn test_strings_ascii_utf16() {
        let test = [
            87, 0, 105, 0, 110, 0, 100, 0, 111, 0, 119, 0, 115, 0, 32, 0, 49, 0, 48, 0, 32, 0, 76,
            0, 84, 0, 83, 0, 66, 0, 0,
        ];
        let data = extract_utf16_string(&test);
        assert_eq!(data, "Windows 10 LTSB");
    }

    #[test]
    fn test_extract_utf8_string_bad_utf8() {
        let test = [
            50, 39, 43, 162, 202, 31, 180, 42, 43, 166, 138, 218, 182, 42, 39, 58, 119, 140, 137,
            202, 232, 178, 135, 237, 89, 172, 145, 121, 217, 168, 157, 213, 128, 247, 205, 57,
        ];

        assert_eq!(
            extract_utf8_string(&test),
            "Failed to get UTF8 string: MicrosoftCorporationOneMicrosoftWayRedmondWA9805"
        );
    }

    #[test]
    fn test_extract_utf8_lossy() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/outlook/windows11/invalid_utf8_email.html");
        let test = read_file(test_location.to_str().unwrap()).unwrap();

        assert_eq!(extract_utf8_string_lossy(&test).len(), 11750);
    }

    #[test]
    fn test_extract_utf16_complex_string() {
        let test = [
            82, 0, 97, 0, 105, 0, 115, 0, 101, 0, 32, 0, 89, 0, 111, 0, 117, 0, 114, 0, 32, 0, 104,
            0, 97, 0, 110, 0, 100, 0, 32, 0, 105, 0, 102, 0, 32, 0, 121, 0, 111, 0, 117, 0, 32, 0,
            108, 0, 105, 0, 107, 0, 101, 0, 32, 0, 82, 0, 117, 0, 115, 0, 116, 0, 33, 0, 32, 0, 61,
            216, 75, 222, 13, 32, 66, 38, 15, 254, 32, 0, 61, 216, 75, 222, 13, 32, 64, 38, 15,
            254,
        ];

        assert_eq!(
            extract_utf16_string(&test),
            "Raise Your hand if you like Rust! 🙋‍♂️ 🙋‍♀️"
        )
    }
}
