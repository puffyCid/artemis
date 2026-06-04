use super::{
    error::RuntimeError,
    setup::{run_async_script, run_script},
};
use crate::{
    output2::{
        manager::OutputManager,
        record::{Record, SingleRecordStream, VecRecordStream},
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
    decode_script(manager, script, &[])
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
    options: &JSScript,
    args: &[String],
) -> Result<(), RuntimeError> {
    let script_result = base64_decode_standard(&options.script);
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

    output_data(script_value, options, manager)
}

/// A `BoaJS` runtime we use to filter data
pub(crate) struct JsFilterRuntime {
    /// `BoaJS` runtime context
    pub(crate) context: Context,
}
/// Create a JavaScript runtime to filter data. Used by the `OutputManager` to filter data when writing results
pub(crate) fn create_filter_runtime(script: &str) -> Result<JsFilterRuntime, RuntimeError> {
    JsFilterRuntime::new(script)
}

/// Output our script data based the configured `OutputManager`
pub(crate) fn output_data(
    entries: Value,
    options: &JSScript,
    manager: &mut OutputManager,
) -> Result<(), RuntimeError> {
    let records = match Record::from_value(entries) {
        Ok(result) => result,
        Err(err) => {
            error!("[runtime] Could not create record from data: {err:?}");
            return Err(RuntimeError::Output);
        }
    };

    match records {
        Record::Array(record_array) => {
            if let Err(err) = manager.write_artifact(
                &options.name,
                options,
                &mut VecRecordStream::new(record_array),
            ) {
                println!("[runtime] Could not write record from data: {err:?}");
                return Err(RuntimeError::Output);
            }
        }
        // All other values are treat as a `SingleRecordStream`
        _ => {
            if let Err(err) = manager.write_artifact(
                &options.name,
                options,
                &mut SingleRecordStream::new(records),
            ) {
                error!("[runtime] Could not write record from data: {err:?}");
                return Err(RuntimeError::Output);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{decode_script, execute_script, raw_script};
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use crate::{
        output2::manager::OutputManager,
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
        let mut output = output_options("runtime_test", "./tmp", false, OutputFormat::Json);

        let script = JSScript {
            name: String::from("hello world"),
            script: String::from("Y29uc29sZS5sb2coIkhlbGxvIGJvYSEiKTs="),
        };
        decode_script(&mut output, &script, &[]).unwrap();
    }

    #[test]
    fn test_raw_script() {
        let test = r#"console.log(2+2);"#;
        raw_script(&test).unwrap();
    }

    #[test]
    fn test_execute_script() {
        let mut output = output_options("runtime_test", "./tmp", false, OutputFormat::Jsonl);
        let script = JSScript {
            name: String::from("hello world"),
            script: String::from("Y29uc29sZS5sb2coIkhlbGxvIGJvYSEiKTs="),
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
        let script = JSScript {
            name: String::from("hello world"),
            script: from_utf8(&buffer).unwrap().to_string(),
        };
        decode_script(&mut output, &script, &[]).unwrap();
    }

    #[test]
    fn test_output_data() {
        let mut output = output_options("output_test", "./tmp", false, OutputFormat::Jsonl);

        let data = json!([{"test":"test"},{"test":"test"}]);
        let status = output_data(
            data,
            &JSScript {
                name: String::from("test"),
                script: String::new(),
            },
            &mut output,
        )
        .unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_output_data_mix_array() {
        let mut output = output_options("output_test", "./tmp", false, OutputFormat::Jsonl);

        let data = json!([{"test":"test"},123, false]);
        let status = output_data(
            data,
            &JSScript {
                name: String::from("test"),
                script: String::new(),
            },
            &mut output,
        )
        .unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_output_data_json_format_mix_array() {
        let mut output = output_options("output_test", "./tmp", false, OutputFormat::Json);

        let data = json!([{"test":"test"},123, false]);
        let status = output_data(
            data,
            &JSScript {
                name: String::from("test"),
                script: String::new(),
            },
            &mut output,
        )
        .unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_output_data_text_format_mix_array() {
        let mut output = output_options("output_test", "./tmp", false, OutputFormat::Text);

        let data = json!([{"test":"test"},123, false]);
        let err = output_data(
            data,
            &JSScript {
                name: String::from("test"),
                script: String::new(),
            },
            &mut output,
        )
        .unwrap_err();
        assert!(matches!(err, RuntimeError::Output));
    }

    #[test]
    fn test_output_data_text_format() {
        let mut output = output_options("output_test", "./tmp", false, OutputFormat::Text);

        let data = json!(["test", "hello world!", 123, true]);
        let status = output_data(
            data,
            &JSScript {
                name: String::from("test"),
                script: String::new(),
            },
            &mut output,
        )
        .unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_output_data_text_format_string() {
        let mut output = output_options("output_test", "./tmp", false, OutputFormat::Text);

        let data = json!("a very simple string of text");
        let status = output_data(
            data,
            &JSScript {
                name: String::from("test"),
                script: String::new(),
            },
            &mut output,
        )
        .unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_output_data_json_format_string() {
        let mut output = output_options("output_test", "./tmp", false, OutputFormat::Json);

        let data = json!("a very simple string of text");
        let status = output_data(
            data,
            &JSScript {
                name: String::from("test"),
                script: String::new(),
            },
            &mut output,
        )
        .unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_output_data_csv_format_mix_array() {
        let mut output = output_options("output_test", "./tmp", false, OutputFormat::Csv);

        let data = json!([{"test":"test"},123, false]);
        let err = output_data(
            data,
            &JSScript {
                name: String::from("test"),
                script: String::new(),
            },
            &mut output,
        )
        .unwrap_err();
        assert!(matches!(err, RuntimeError::Output));
    }
}
