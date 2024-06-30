use super::{
    nom_helper::{nom_unsigned_two_bytes, Endian},
    strings::{extract_utf16_string, extract_utf8_string},
};
use crate::{filesystem::files::read_file, utils::error::ArtemisError};
use base64::{engine::general_purpose, DecodeError, Engine};
use log::error;

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
            error!("[artemis-core] Could not read XML file at {path}: {err:?}");
            return Err(ArtemisError::ReadXml);
        }
    };

    let utf_check = nom_unsigned_two_bytes(&bytes, Endian::Be);
    let (data, utf_status) = match utf_check {
        Ok(result) => result,
        Err(_err) => {
            error!("[artemis-core] Could not determine UTF encoding for XML {path}");
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

#[cfg(test)]
mod tests {
    use crate::utils::encoding::{
        base64_decode_standard, base64_encode_standard, base64_encode_url, read_xml,
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
}
