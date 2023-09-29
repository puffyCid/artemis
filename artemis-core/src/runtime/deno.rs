/**
 * Embeds the Deno runtime and core into Artemis
 * This allows us to execute Javascript using Rust
 */
use super::{
    error::RuntimeError,
    run::{run_async_script, run_script},
};
use crate::{
    output::formats::{json::json_format, jsonl::jsonl_format},
    structs::{artifacts::runtime::script::JSScript, toml::Output},
    utils::{encoding::base64_decode_standard, time},
};
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
pub(crate) fn raw_script(script: &str) -> Result<(), RuntimeError> {
    let args = [];
    let result = if script.contains(" async ") || script.contains(" await ") {
        run_async_script(script, &args)
    } else {
        run_script(script, &args)
    };

    match result {
        Ok(_result) => {}
        Err(err) => {
            error!(
                "[runtime] Could not execute javascript: {}",
                err.to_string()
            );
            return Err(RuntimeError::ExecuteScript);
        }
    };

    Ok(())
}

/// Base64 decode the Javascript string and execute using Deno runtime and output the returned value
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

    let result = if script.contains(" async ") || script.contains(" await ") {
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

    output_data(&script_value, script_name, output, &start_time)?;
    Ok(())
}

/// Output Javascript results based on the output options provided from the TOML file
pub(crate) fn output_data(
    serde_data: &Value,
    output_name: &str,
    output: &mut Output,
    start_time: &u64,
) -> Result<(), RuntimeError> {
    let output_status = if output.format == "json" {
        json_format(serde_data, output_name, output, start_time)
    } else if output.format == "jsonl" {
        jsonl_format(serde_data, output_name, output, start_time)
    } else {
        error!("[runtime] Unknown formatter provided: {}", output.format);
        return Err(RuntimeError::Format);
    };
    match output_status {
        Ok(_) => {}
        Err(err) => {
            error!("[runtime] Could not output data: {err:?}");
            return Err(RuntimeError::Output);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{decode_script, execute_script, filter_script, raw_script};
    use crate::{
        runtime::deno::output_data,
        structs::{artifacts::runtime::script::JSScript, toml::Output},
        utils::time,
    };

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("json"),
            compress,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        }
    }

    #[test]
    fn test_decode_script() {
        let test = "Y29uc29sZS5sb2coIkhlbGxvIGRlbm8hIik7";
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
        let test = "Y29uc29sZS5sb2coIkhlbGxvIGRlbm8hIik7";
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
        let data = serde_json::Value::String(String::from("test"));
        let status = output_data(&data, name, &mut output, &start_time).unwrap();
        assert_eq!(status, ());
    }
}
