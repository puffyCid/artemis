use super::{
    error::RuntimeError,
    setup::{run_async_script, run_script},
};
use crate::{
    output2::{
        manager::OutputManager,
        record::{Record, VecRecordStream},
    },
    structs::artifacts::runtime::script::JSScript,
    utils::encoding::base64_decode_standard,
};
use boa_engine::Context;
use log::error;
use serde_json::Value;
use std::str::from_utf8;

/// Execute the provided JavaScript data from the TOML input
pub(crate) fn execute_script(
    manager: &mut OutputManager,
    script: &JSScript,
) -> Result<(), RuntimeError> {
    decode_script(manager, &script.name, &script.script, &[])
}

/// Execute the provided JavaScript data from the TOML and provide a stringified serde Value as an argument to JavaScript
pub(crate) fn filter_script(
    manager: &mut OutputManager,
    args: &[String],
    filter_name: &str,
    filter_script: &str,
) -> Result<(), RuntimeError> {
    decode_script(manager, filter_name, filter_script, args)
}

/// Execute raw JavaScript code
pub(crate) fn raw_script(script: &str) -> Result<Value, RuntimeError> {
    let args = [];
    let result = if script.contains("async function ") || script.contains(" await ") {
        run_async_script(script, &args)
    } else {
        run_script(script, &args)
    };

    let status = match result {
        Ok(result) => result,
        Err(err) => {
            error!("[runtime] Could not execute javascript: {err}");
            return Err(RuntimeError::ExecuteScript);
        }
    };

    Ok(status)
}

/// Base64 decode the Javascript string and execute using Boa runtime and output the returned value
fn decode_script(
    manager: &mut OutputManager,
    script_name: &str,
    encoded_script: &str,
    args: &[String],
) -> Result<(), RuntimeError> {
    let script_result = base64_decode_standard(encoded_script);
    let script_bytes = match script_result {
        Ok(result) => result,
        Err(err) => {
            error!("[runtime] Could not base64 provided javascript script: {err:?}",);
            return Err(RuntimeError::Decode);
        }
    };

    let str_result = from_utf8(&script_bytes);
    let script = match str_result {
        Ok(result) => result,
        Err(err) => {
            error!("[runtime] Could not read javascript script as string: {err:?}");
            return Err(RuntimeError::Decode);
        }
    };

    let result = if script.contains("async function") || script.contains(" await ") {
        run_async_script(script, args)
    } else {
        run_script(script, args)
    };
    let script_value = match result {
        Ok(result) => result,
        Err(err) => {
            error!("[runtime] Could not execute javascript: {err:?}");
            return Err(RuntimeError::ExecuteScript);
        }
    };

    if script_value.is_null() {
        return Ok(());
    }

    output_data(script_value, script_name, manager)
}

/// A `BoaJS` runtime we use to filter data
pub(crate) struct JsFilterRuntime {
    /// `BoaJS` runtime context
    pub(crate) context: Context,
}
/// Create a JavaScript runtime to filter data
pub(crate) fn create_filter_runtime(script: &str) -> Result<JsFilterRuntime, RuntimeError> {
    JsFilterRuntime::new(script)
}

pub(crate) fn output_data(
    entries: Value,
    script_name: &str,
    manager: &mut OutputManager,
) -> Result<(), RuntimeError> {
    let records = match Record::from_value(entries) {
        Ok(result) => result,
        Err(err) => {
            error!("[runtime] Could not create record from data: {err:?}");
            return Err(RuntimeError::Output);
        }
    };
    if let Err(err) =
        manager.write_artifact(script_name, &"", &mut VecRecordStream::new(vec![records]))
    {
        error!("[runtime] Could not write record from data: {err:?}");
        return Err(RuntimeError::Output);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{decode_script, execute_script, filter_script, raw_script};
    use crate::{
        output2::{
            config::{OutputConfig, OutputDestination, OutputFormat},
            manager::OutputManager,
        },
        runtime::{error::RuntimeError, run::output_data},
        structs::artifacts::runtime::script::JSScript,
    };
    use serde_json::json;
    use std::path::PathBuf;

    fn output_options(
        name: &str,
        directory: &str,
        compress: bool,
        format: OutputFormat,
    ) -> OutputManager {
        let config = OutputConfig {
            name: name.to_string(),
            directory: PathBuf::from(directory),
            format,
            compress,
            endpoint_id: String::from("abcd"),
            destination: OutputDestination::Local,
            ..Default::default()
        };
        OutputManager::new(config).unwrap()
    }

    #[test]
    fn test_decode_script() {
        let test = "Y29uc29sZS5sb2coIkhlbGxvIGJvYSEiKTs=";
        let mut output = output_options("runtime_test", "./tmp", false, OutputFormat::Json);

        decode_script(&mut output, "hello world", test, &[]).unwrap();
    }

    #[test]
    fn test_raw_script() {
        let test = r#"console.log(2+2);"#;
        raw_script(&test).unwrap();
    }

    #[test]
    fn test_execute_script() {
        let test = "Y29uc29sZS5sb2coIkhlbGxvIGJvYSEiKTs=";
        let mut output = output_options("runtime_test", "./tmp", false, OutputFormat::Jsonl);
        let script = JSScript {
            name: String::from("hello world"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_advanced_decode_script() {
        use crate::filesystem::files::read_file;
        use std::{path::PathBuf, str::from_utf8};

        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/deno_scripts/read_homebrew.txt");
        let buffer = read_file(&test_location.display().to_string()).unwrap();

        let mut output = output_options("runtime_test", "./tmp", false, OutputFormat::Jsonl);

        decode_script(
            &mut output,
            "homebrew_packages",
            from_utf8(&buffer).unwrap(),
            &[],
        )
        .unwrap();
    }

    #[test]
    fn test_filter_script() {
        let mut output = output_options("split_string", "./tmp", false, OutputFormat::Text);
        filter_script(&mut output, &vec![String::from("helloRust")], "test", "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBtYWluKCkgewogICAgY29uc3QgYXJncyA9IFNUQVRJQ19BUkdTOwogICAgaWYgKGFyZ3MubGVuZ3RoID09PSAwKSB7CiAgICAgICAgcmV0dXJuIFtdOwogICAgfQogICAgY29uc3QgdGVzdCA9IGFyZ3NbMF07CiAgICBjb25zdCB2YWx1ZXMgPSB0ZXN0LnNwbGl0KCJoZWxsbyIpCgogICAgcmV0dXJuIHZhbHVlczsKfQptYWluKCk7Cgo=").unwrap();
    }

    #[test]
    fn test_output_data() {
        let mut output = output_options("output_test", "./tmp", false, OutputFormat::Jsonl);

        let name = "test";
        let data = json!([{"test":"test"},{"test":"test"}]);
        let status = output_data(data, name, &mut output).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_output_data_mix_array() {
        let mut output = output_options("output_test", "./tmp", false, OutputFormat::Jsonl);

        let name = "test";
        let data = json!([{"test":"test"},123, false]);
        let status = output_data(data, name, &mut output).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_output_data_json_format_mix_array() {
        let mut output = output_options("output_test", "./tmp", false, OutputFormat::Json);

        let name = "test";
        let data = json!([{"test":"test"},123, false]);
        let status = output_data(data, name, &mut output).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_output_data_text_format_mix_array() {
        let mut output = output_options("output_test", "./tmp", false, OutputFormat::Text);

        let name = "test";
        let data = json!([{"test":"test"},123, false]);
        let status = output_data(data, name, &mut output).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_output_data_csv_format_mix_array() {
        let mut output = output_options("output_test", "./tmp", false, OutputFormat::Csv);

        let name = "test";
        let data = json!([{"test":"test"},123, false]);
        let err = output_data(data, name, &mut output).unwrap_err();
        assert!(matches!(err, RuntimeError::Output));
    }
}
