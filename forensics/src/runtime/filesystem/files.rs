use crate::{
    accessor::access::Accessor,
    filesystem::files::hash_reader,
    runtime::{
        filesystem::accessor::return_file_handle_object,
        helper::{boolean_arg, number_arg, value_arg},
    },
    utils::strings::extract_utf8_string_lossy,
};
use boa_engine::{Context, JsError, JsResult, JsString, JsValue, js_string};
use common::files::Hashes;
use serde::Serialize;
use std::io::BufRead;

#[derive(Serialize, Debug)]
pub(crate) struct HashInfo {
    md5: String,
    sha1: String,
    sha256: String,
}

/// Hash a file from provided path based on hashing algorithms. If file is not provided empty values are returned
pub(crate) fn js_hash_file(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let value = value_arg(args, 0, context)?;
    let hashes = Hashes {
        md5: boolean_arg(args, 1)?,
        sha1: boolean_arg(args, 2)?,
        sha256: boolean_arg(args, 3)?,
    };

    let handle = return_file_handle_object(value)?;
    let mut reader = match Accessor::with_defaults().open_reader_handle(&handle) {
        Ok(results) => results,
        Err(err) => {
            let issue = format!(
                "Could not open reader for {}: {err:?}",
                handle.display_path()
            );
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let (md5_value, sha1_value, sha256_value) = hash_reader(&hashes, &mut reader);
    let info = HashInfo {
        md5: md5_value,
        sha1: sha1_value,
        sha256: sha256_value,
    };
    let data = serde_json::to_value(&info).unwrap_or_default();
    let value = JsValue::from_json(&data, context)?;

    Ok(value)
}

/// Read a text file at provided path. Currently only files smaller than 2GB can be read
pub(crate) fn js_read_text_file(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let value = value_arg(args, 0, context)?;
    let handle = return_file_handle_object(value)?;
    let bytes = match Accessor::with_defaults().read_file_handle(&handle) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Could not read text {}: {err:?}", handle.display_path());
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    Ok(JsValue::new::<JsString>(
        extract_utf8_string_lossy(&bytes).into(),
    ))
}

/// Read a specified number of lines from a text file
pub(crate) fn js_read_lines(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let value = value_arg(args, 0, context)?;
    let offset = number_arg(args, 1)? as usize;
    let limit = number_arg(args, 2)? as u64;

    let handle = return_file_handle_object(value)?;

    let reader = match Accessor::with_defaults().open_reader_handle(&handle) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!(
                "Could not open reader for {} for file lines: {err:?}",
                handle.display_path()
            );
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let mut lines = Vec::new();
    let mut count = 0;
    let mut start_reader = reader.lines().skip(offset);
    while let Some(Ok(line)) = start_reader.next() {
        if count == limit {
            break;
        }
        lines.push(line);
        count += 1;
    }

    let data = serde_json::to_value(&lines).unwrap_or_default();
    let value = JsValue::from_json(&data, context)?;

    Ok(value)
}

#[cfg(test)]
mod tests {
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use crate::utils::encoding::base64_encode_standard;
    use crate::{
        output::manager::OutputManager, runtime::run::execute_script,
        structs::artifacts::runtime::script::JSScript,
    };
    use std::fs;
    use std::path::PathBuf;

    fn output_options(name: &str, directory: &str, compress: bool) -> OutputManager {
        let config = OutputConfig {
            name: name.to_string(),
            directory: PathBuf::from(directory),
            format: OutputFormat::Jsonl,
            compress,
            endpoint_id: String::from("abcd"),
            destination: OutputDestination::Local,
            ..Default::default()
        };
        OutputManager::new(config).unwrap()
    }

    #[test]
    fn test_js_hash_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/runtime/filesystem/hash.js");
        let bytes = fs::read(test_location).unwrap();
        let test = base64_encode_standard(&bytes);

        let mut output = output_options("runtime_test", "./tmp", false);
        let script = JSScript {
            name: String::from("hash_files"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_js_read_lines() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/runtime/filesystem/lines.js");
        let bytes = fs::read(test_location).unwrap();
        let test = base64_encode_standard(&bytes);

        let mut output = output_options("runtime_test", "./tmp", false);
        let script = JSScript {
            name: String::from("text_files"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_js_read_text_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/runtime/filesystem/text.js");
        let bytes = fs::read(test_location).unwrap();
        let test = base64_encode_standard(&bytes);

        let mut output = output_options("runtime_test", "./tmp", false);
        let script = JSScript {
            name: String::from("text_files"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
