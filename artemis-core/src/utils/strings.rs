use crate::utils::encoding::base64_encode_standard;
use log::warn;

#[cfg(target_os = "windows")]
/// Get a UTF16 string from provided bytes data
pub(crate) fn extract_utf16_string(data: &[u8]) -> String {
    let mut utf16_data: Vec<u16> = Vec::new();
    // Convert data to UTF16 (&[u16])
    let min_byte_size = 2;
    for wide_char in data.chunks(min_byte_size) {
        if wide_char == vec![0, 0] || wide_char.len() < min_byte_size {
            break;
        }

        utf16_data.push(u16::from_ne_bytes([wide_char[0], wide_char[1]]));
    }

    // Windows uses UTF16
    let utf16_result = String::from_utf16(&utf16_data);
    let result = match utf16_result {
        Ok(results) => results.trim_end_matches('\0').to_string(),
        Err(err) => {
            warn!("[strings] Failed to get UTF16 string: {err:?}");

            let max_size = 2097152;
            let issue = if data.len() < max_size {
                base64_encode_standard(data)
            } else {
                format!("Binary data size larger than 2MB, size: {}", data.len())
            };
            format!("Failed to get UTF16: {}", issue)
        }
    };
    result
}

#[cfg(target_os = "windows")]
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

/// Get a UTF8 string from provided bytes data
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

#[cfg(target_os = "windows")]
/// Detect ASCII or UTF16 byte string
pub(crate) fn extract_ascii_utf16_string(data: &[u8]) -> String {
    if data.is_ascii() && data.iter().filter(|&c| *c == 0).count() <= 1 {
        extract_utf8_string(data)
    } else {
        extract_utf16_string(data)
    }
}

#[cfg(target_os = "windows")]
/// Check if either string contains the other
pub(crate) fn strings_contains(input1: &str, input2: &str) -> bool {
    if input1.contains(input2) || input2.contains(input1) {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::extract_utf8_string;

    #[test]
    #[cfg(target_os = "windows")]
    fn test_extract_utf16_string() {
        use crate::utils::strings::extract_utf16_string;

        let test_data = vec![
            79, 0, 83, 0, 81, 0, 85, 0, 69, 0, 82, 0, 89, 0, 68, 0, 46, 0, 69, 0, 88, 0, 69, 0, 0,
            0,
        ];
        assert_eq!(extract_utf16_string(&test_data), "OSQUERYD.EXE")
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_extract_multiline_utf16_string() {
        use crate::utils::strings::extract_multiline_utf16_string;

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
    #[cfg(target_os = "windows")]
    fn test_extract_ascii_utf16_string() {
        use crate::utils::strings::extract_ascii_utf16_string;

        let test_data = vec![79, 83, 81, 85, 69, 82, 89, 68, 46, 69, 88, 69, 0];
        assert_eq!(extract_ascii_utf16_string(&test_data), "OSQUERYD.EXE")
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_strings_contains() {
        use crate::utils::strings::strings_contains;

        let path1 = "a very long path";
        let path2 = "long path";
        let result = strings_contains(path1, path2);
        assert_eq!(result, true);
    }
}
