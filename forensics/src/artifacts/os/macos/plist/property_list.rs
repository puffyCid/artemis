use super::error::PlistError;
use log::error;
use plist::{Dictionary, Value};

/// Parse a `plist` from given path and return a Value (any `plist` value)
pub(crate) fn parse_plist_file(path: &str) -> Result<Value, PlistError> {
    let plist_result = plist::from_file(path);
    match plist_result {
        Ok(result) => Ok(result),
        Err(err) => {
            error!("[plist] Could not read plist file {path}: {err:?}");
            Err(PlistError::File)
        }
    }
}

/// Parse a `plist` from given path and return a dictionary `plist`.
/// Use only if you are certain the `plist` file is a Dictionary format.
/// Otherwise  use `parse_plist_file` which will handle any `plist` format
pub(crate) fn parse_plist_file_dict(path: &str) -> Result<Dictionary, PlistError> {
    let plist_result = plist::from_file(path);
    match plist_result {
        Ok(result) => Ok(result),
        Err(err) => {
            error!("[plist] Could not read plist file {path}: {err:?}");
            Err(PlistError::File)
        }
    }
}

/// Parse a `plist` from bytes and return a Value (any `plist` value)
pub(crate) fn parse_plist_data(data: &[u8]) -> Result<Value, PlistError> {
    let plist_result = plist::from_bytes(data);
    match plist_result {
        Ok(result) => Ok(result),
        Err(err) => {
            error!("[plist] Could not parse plist data: {err:?}");
            Err(PlistError::File)
        }
    }
}

/// Return a `plist` value as dictionary
pub(crate) fn get_dictionary(plist_value: &Value) -> Result<Dictionary, PlistError> {
    let result = plist_value.as_dictionary();
    match result {
        Some(data) => Ok(data.clone()),
        None => Err(PlistError::Dictionary),
    }
}

/// Return a `plist` value as bytes
pub(crate) fn get_data(plist_value: &Value) -> Result<Vec<u8>, PlistError> {
    let result = plist_value.as_data();
    match result {
        Some(data) => Ok(data.to_vec()),
        None => Err(PlistError::Data),
    }
}

/// Return a `plist` bytes as base64 string
pub(crate) fn get_boolean(plist_value: &Value) -> Result<bool, PlistError> {
    let result = plist_value.as_boolean();
    match result {
        Some(data) => Ok(data),
        None => Err(PlistError::Bool),
    }
}

/// Return a `plist` value as dictionary
pub(crate) fn get_string(plist_value: &Value) -> Result<String, PlistError> {
    let result = plist_value.as_string();
    match result {
        Some(data) => Ok(data.to_string()),
        None => Err(PlistError::String),
    }
}

/// Return a `plist` value as signed int
pub(crate) fn get_signed_int(plist_value: &Value) -> Result<i64, PlistError> {
    let result = plist_value.as_signed_integer();
    match result {
        Some(data) => Ok(data),
        None => Err(PlistError::SignedInt),
    }
}

/// Return a `plist value` as Vec<Value>
pub(crate) fn get_array(plist_value: &Value) -> Result<Vec<Value>, PlistError> {
    let result = plist_value.as_array();
    match result {
        Some(data) => Ok(data.clone()),
        None => Err(PlistError::Array),
    }
}

pub(crate) fn get_float(plist_value: &Value) -> Result<f64, PlistError> {
    let result = plist_value.as_real();
    match result {
        Some(data) => Ok(data),
        None => Err(PlistError::Float),
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::macos::plist::property_list::{
            get_array, get_boolean, get_data, get_dictionary, get_float, get_signed_int,
            get_string, parse_plist_data, parse_plist_file,
        },
        filesystem::files::read_file,
    };
    use plist::{Integer, Value};
    use std::path::PathBuf;

    #[test]
    fn test_get_array_value() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/users/nobody.plist");

        let downloads = parse_plist_file(&test_location.display().to_string()).unwrap();
        let mut shell = Vec::new();
        for (key, value) in downloads.as_dictionary().unwrap() {
            if key != "shell" {
                continue;
            }

            match value {
                Value::Array(_) => {
                    shell = get_array(&value).unwrap();
                }
                _ => {}
            }
        }
        assert_eq!(shell.len(), 1)
    }

    #[test]
    fn test_get_boolean() {
        let test = Value::Boolean(true);
        let results = get_boolean(&test).unwrap();

        assert_eq!(results, true);
    }

    #[test]
    fn test_get_data() {
        let test = Value::Data(vec![1, 0, 1]);
        let results = get_data(&test).unwrap();

        assert_eq!(results, [1, 0, 1]);
    }

    #[test]
    fn test_get_dictionary() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/loginitems/backgrounditems_sierra.btm");

        let login_items = parse_plist_file(&test_location.display().to_string()).unwrap();

        for (key, value) in login_items.as_dictionary().unwrap() {
            if key.as_str() != "$objects" {
                continue;
            }
            match value {
                Value::Array(value_array) => {
                    for data in value_array {
                        match data {
                            Value::Dictionary(_) => {
                                let dict = get_dictionary(&data).unwrap();
                                assert_eq!(dict.len(), 3);
                                break;
                            }
                            _ => continue,
                        }
                    }
                }
                _ => {}
            }
        }
    }

    #[test]
    fn test_get_string() {
        let test: Value = Value::String(String::from("test"));
        let results = get_string(&test).unwrap();
        assert_eq!(results, "test");
    }

    #[test]
    fn test_get_signed_int() {
        let test: Value = Value::Integer(Integer::from(2));
        let results = get_signed_int(&test).unwrap();
        assert_eq!(results, 2);
    }

    #[test]
    fn test_parse_plist_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/loginitems/backgrounditems_sierra.btm");
        let result = parse_plist_file(&test_location.display().to_string()).unwrap();
        assert_eq!(result.as_dictionary().unwrap().len(), 4);
    }

    #[test]
    fn test_get_float() {
        let test = Value::Real(10.0);
        let results = get_float(&test).unwrap();

        assert_eq!(results, 10.0);
    }
    #[test]
    fn test_parse_plist_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/loginitems/backgrounditems_sierra.btm");
        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let result = parse_plist_data(&buffer).unwrap();
        assert_eq!(result.as_dictionary().unwrap().len(), 4);
    }
}
