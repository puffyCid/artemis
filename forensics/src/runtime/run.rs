use super::{
    error::RuntimeError,
    setup::{run_async_script, run_script},
};
use crate::{
    artifacts::output::output_artifact,
    output2::{
        manager::OutputManager,
        record::{Record, VecRecordStream},
    },
    structs::{artifacts::runtime::script::JSScript, toml::Output},
    utils::{encoding::base64_decode_standard, time},
};
use boa_engine::Context;
use log::error;
use serde_json::Value;
use std::str::from_utf8;

/// Execute the provided JavaScript data from the TOML input
pub(crate) fn execute_script(output: &mut Output, script: &JSScript) -> Result<(), RuntimeError> {
    decode_script(output, &script.name, &script.script, &[])
}

/// Execute the provided JavaScript data from the TOML and provide a stringified serde Value as an argument to JavaScript
pub(crate) fn filter_script(
    output: &mut Output,
    args: &[String],
    filter_name: &str,
    filter_script: &str,
) -> Result<(), RuntimeError> {
    decode_script(output, filter_name, filter_script, args)
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
    output: &mut Output,
    script_name: &str,
    encoded_script: &str,
    args: &[String],
) -> Result<(), RuntimeError> {
    let start_time = time::time_now();

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
    let mut script_value = match result {
        Ok(result) => result,
        Err(err) => {
            error!("[runtime] Could not execute javascript: {err:?}");
            return Err(RuntimeError::ExecuteScript);
        }
    };

    if script_value.is_null() {
        return Ok(());
    }

    output_data(&mut script_value, script_name, output, start_time)?;
    Ok(())
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

/// Output Javascript results based on the output options provided from the TOML file
pub(crate) fn output_data(
    serde_data: &mut Value,
    output_name: &str,
    output: &mut Output,
    start_time: u64,
) -> Result<(), RuntimeError> {
    // We must never filter a script. Otherwise this would cause an infinite loop!
    let filter = false;
    let status = output_artifact(serde_data, output_name, output, start_time, filter);
    if let Err(result) = status {
        error!("[runtime] Could not output data: {result:?}");
        return Err(RuntimeError::Output);
    }
    Ok(())
}

pub(crate) fn output_data2(
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
    use std::path::PathBuf;

    use super::{decode_script, execute_script, filter_script, raw_script};
    use crate::{
        output2::{
            config::{OutputConfig, OutputDestination, OutputFormat},
            manager::OutputManager,
        },
        runtime::{
            error::RuntimeError,
            run::{output_data, output_data2},
        },
        structs::{artifacts::runtime::script::JSScript, toml::Output},
        utils::time,
    };
    use serde_json::json;

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("json"),
            compress,
            endpoint_id: String::from("abcd"),
            output: output.to_string(),
            ..Default::default()
        }
    }

    fn output_options2(
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
        let mut output = output_options("runtime_test", "local", "./tmp", false);

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
        let mut output = output_options("runtime_test", "local", "./tmp", false);
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

        let mut output = output_options("runtime_test", "local", "./tmp", false);

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
        let mut output = output_options("split_string", "local", "./tmp", false);
        filter_script(&mut output, &vec![String::from("helloRust")], "test", "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBtYWluKCkgewogICAgY29uc3QgYXJncyA9IFNUQVRJQ19BUkdTOwogICAgaWYgKGFyZ3MubGVuZ3RoID09PSAwKSB7CiAgICAgICAgcmV0dXJuIFtdOwogICAgfQogICAgY29uc3QgdGVzdCA9IGFyZ3NbMF07CiAgICBjb25zdCB2YWx1ZXMgPSB0ZXN0LnNwbGl0KCJoZWxsbyIpCgogICAgcmV0dXJuIHZhbHVlczsKfQptYWluKCk7Cgo=").unwrap();
    }

    #[test]
    fn test_output_data() {
        let mut output = output_options("output_test", "local", "./tmp", false);
        let start_time = time::time_now();

        let name = "test";
        let mut data = json!({"test":"test"});
        let status = output_data(&mut data, name, &mut output, start_time).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_output_data2() {
        let mut output = output_options2("output_test", "./tmp", false, OutputFormat::Jsonl);

        let name = "test";
        let data = json!([{"test":"test"},{"test":"test"}]);
        let status = output_data2(data, name, &mut output).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_output_data2_mix_array() {
        let mut output = output_options2("output_test", "./tmp", false, OutputFormat::Jsonl);

        let name = "test";
        let data = json!([{"test":"test"},123, false]);
        let status = output_data2(data, name, &mut output).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_output_data2_json_format_mix_array() {
        let mut output = output_options2("output_test", "./tmp", false, OutputFormat::Json);

        let name = "test";
        let data = json!([{"test":"test"},123, false]);
        let status = output_data2(data, name, &mut output).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_output_data2_text_format_mix_array() {
        let mut output = output_options2("output_test", "./tmp", false, OutputFormat::Text);

        let name = "test";
        let data = json!([{"test":"test"},123, false]);
        let status = output_data2(data, name, &mut output).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_output_data2_csv_format_mix_array() {
        let mut output = output_options2("output_test", "./tmp", false, OutputFormat::Csv);

        let name = "test";
        let data = json!([{"test":"test"},123, false]);
        let err = output_data2(data, name, &mut output).unwrap_err();
        assert!(matches!(err, RuntimeError::Output));
    }
}
