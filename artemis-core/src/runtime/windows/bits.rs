use crate::{
    artifacts::os::windows::bits::parser::{grab_bits, grab_bits_path},
    runtime::error::RuntimeError,
    structs::artifacts::os::windows::BitsOptions,
};
use deno_core::{error::AnyError, op};
use log::error;

#[op]
/// Expose parsing default BITS location on systemdrive to `Deno`
fn get_bits(carve: bool) -> Result<String, AnyError> {
    let options = BitsOptions {
        alt_path: None,
        carve,
    };
    let bits_results = grab_bits(&options);
    let bits = match bits_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse BITS: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let results = serde_json::to_string_pretty(&bits)?;
    Ok(results)
}

#[op]
/// Expose parsing provided BITS path to `Deno`
fn get_bits_path(path: String, carve: bool) -> Result<String, AnyError> {
    if path.is_empty() {
        error!("[runtime] Can not parse BITS path, path is empty.");
        return Err(RuntimeError::ExecuteScript.into());
    }

    let bits_results = grab_bits_path(&path, carve);
    let bits = match bits_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse BITS file: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let results = serde_json::to_string_pretty(&bits)?;
    Ok(results)
}

#[cfg(test)]
mod tests {
    use crate::{
        runtime::deno::execute_script, structs::artifacts::runtime::script::JSScript,
        utils::artemis_toml::Output,
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
            filter_name: None,
            filter_script: None,
            logging: None,
        }
    }

    #[test]
    fn test_get_bits() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfYml0cyhjYXJ2ZSkgewogICAgY29uc3QgZGF0YSA9IERlbm9bRGVuby5pbnRlcm5hbF0uY29yZS5vcHMuZ2V0X2JpdHMoY2FydmUpOwogICAgY29uc3QgYml0cyA9IEpTT04ucGFyc2UoZGF0YSk7CiAgICByZXR1cm4gYml0czsKfQpmdW5jdGlvbiBnZXRCaXRzKGNhcnZlKSB7CiAgICByZXR1cm4gZ2V0X2JpdHMoY2FydmUpOwp9CmZ1bmN0aW9uIG1haW4oKSB7CiAgICBjb25zdCBlbnRyaWVzID0gZ2V0Qml0cyh0cnVlKTsKICAgIHJldHVybiBlbnRyaWVzOwp9Cm1haW4oKTsKCg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("bits"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_bits_path() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfYml0c19wYXRoKHBhdGgsIGNhcnZlKSB7CiAgICBjb25zdCBkYXRhID0gRGVub1tEZW5vLmludGVybmFsXS5jb3JlLm9wcy5nZXRfYml0c19wYXRoKHBhdGgsIGNhcnZlKTsKICAgIGNvbnN0IGJpdHMgPSBKU09OLnBhcnNlKGRhdGEpOwogICAgcmV0dXJuIGJpdHM7Cn0KZnVuY3Rpb24gZ2V0Qml0c1BhdGgocGF0aCwgY2FydmUpIHsKICAgIHJldHVybiBnZXRfYml0c19wYXRoKHBhdGgsIGNhcnZlKTsKfQpmdW5jdGlvbiBtYWluKCkgewogICAgY29uc3QgcGF0aCA9ICJDOlxcUHJvZ3JhbURhdGFcXE1pY3Jvc29mdFxcTmV0d29ya1xcRG93bmxvYWRlclxccW1nci5kYiI7CiAgICBjb25zdCBlbnRyaWVzID0gZ2V0Qml0c1BhdGgocGF0aCwgdHJ1ZSk7CiAgICByZXR1cm4gZW50cmllczsKfQptYWluKCk7Cgo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("bits_path"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
