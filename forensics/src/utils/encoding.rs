use super::{
    nom_helper::{Endian, nom_unsigned_two_bytes},
    strings::{extract_utf8_string, extract_utf16_string},
};
use crate::{filesystem::files::read_file, utils::error::ArtemisError};
use base64::{DecodeError, Engine, engine::general_purpose};
use log::error;
use std::collections::HashMap;
use sunlight::light::{ProtoTag, extract_protobuf};

/// Base64 encode data using the STANDARD engine (alphabet along with "+" and "/")
pub(crate) fn base64_encode_standard(data: &[u8]) -> String {
    general_purpose::STANDARD.encode(data)
}

/// Base64 encoded data using the URL engine
pub(crate) fn base64_encode_url(data: &[u8]) -> String {
    general_purpose::URL_SAFE.encode(data)
}

/// Base64 decode data use the STANDARD engine (alphabet along with "+" and "/")
pub(crate) fn base64_decode_standard(data: &str) -> Result<Vec<u8>, DecodeError> {
    general_purpose::STANDARD.decode(data)
}

/// Read a XML file. This function will check for UTF16 encoding via Byte Order Mark (BOM)
pub(crate) fn read_xml(path: &str) -> Result<String, ArtemisError> {
    let bytes_result = read_file(path);
    let bytes = match bytes_result {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Could not read XML file at {path}: {err:?}");
            return Err(ArtemisError::ReadXml);
        }
    };

    let utf_check = nom_unsigned_two_bytes(&bytes, Endian::Be);
    let (data, utf_status) = match utf_check {
        Ok(result) => result,
        Err(_err) => {
            error!("[forensics] Could not determine UTF encoding for XML {path}");
            return Err(ArtemisError::UtfType);
        }
    };

    let utf16_le = 0xfffe;
    let utf16_be = 0xfeff;

    let xml_string = if utf_status == utf16_be || utf_status == utf16_le {
        extract_utf16_string(data)
    } else {
        extract_utf8_string(&bytes)
    };

    Ok(xml_string)
}

/// Extract Protobuf data from bytes
pub(crate) fn parse_protobuf(data: &[u8]) -> Result<HashMap<usize, ProtoTag>, ArtemisError> {
    let result = match extract_protobuf(data) {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Could not parse protobuf: {err:?}");
            return Err(ArtemisError::Protobuf);
        }
    };

    Ok(result)
}

#[cfg(test)]
mod tests {
    use crate::utils::encoding::{
        base64_decode_standard, base64_encode_standard, base64_encode_url, parse_protobuf, read_xml,
    };
    use std::path::PathBuf;

    #[test]
    fn test_base64_encode_standard() {
        let test = b"Hello word!";
        let result = base64_encode_standard(test);
        assert_eq!(result, "SGVsbG8gd29yZCE=")
    }

    #[test]
    fn test_base64_encode_url() {
        let test = b"Hello word!";
        let result = base64_encode_url(test);
        assert_eq!(result, "SGVsbG8gd29yZCE=")
    }

    #[test]
    fn test_base64_decode_standard() {
        let test = "SGVsbG8gd29yZCE=";
        let result = base64_decode_standard(test).unwrap();
        assert_eq!(result, b"Hello word!")
    }

    #[test]
    fn test_read_xml() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/tasks/win10/VSIX Auto Update");

        let result = read_xml(&test_location.display().to_string()).unwrap();
        assert!(result.starts_with("<?xml version=\"1.0\" encoding=\"UTF-16\"?>"));
        assert!(result.contains("<URI>\\Microsoft\\VisualStudio\\VSIX Auto Update</URI>"));
        assert_eq!(result.len(), 1356);
    }

    #[test]
    fn test_parse_protobuf() {
        let test = [
            18, 12, 103, 119, 115, 45, 119, 105, 122, 45, 115, 101, 114, 112, 34, 21, 100, 117, 99,
            107, 100, 117, 99, 107, 103, 111, 32, 105, 115, 32, 97, 119, 101, 115, 111, 109, 101,
            50, 10, 16, 0, 24, 176, 3, 24, 214, 4, 24, 71, 50, 10, 16, 0, 24, 176, 3, 24, 214, 4,
            24, 71, 50, 10, 16, 0, 24, 176, 3, 24, 214, 4, 24, 71, 50, 10, 16, 0, 24, 176, 3, 24,
            214, 4, 24, 71, 50, 10, 16, 0, 24, 176, 3, 24, 214, 4, 24, 71, 50, 10, 16, 0, 24, 176,
            3, 24, 214, 4, 24, 71, 50, 10, 16, 0, 24, 176, 3, 24, 214, 4, 24, 71, 50, 10, 16, 0,
            24, 176, 3, 24, 214, 4, 24, 71, 72, 160, 3, 80, 0, 88, 0, 112, 1, 120, 1, 144, 1, 0,
            152, 1, 0, 160, 1, 0, 170, 1, 0, 184, 1, 3, 200, 1, 0, 152, 2, 1, 160, 2, 0, 152, 3, 0,
            136, 6, 1, 144, 6, 8, 146, 7, 1, 49, 160, 7, 0,
        ];

        let result = parse_protobuf(&test).unwrap();
        assert_eq!(result.len(), 21);
    }
}
