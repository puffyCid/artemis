use crate::{artifacts::os::macos::fsevents::parser::grab_fseventsd, runtime::error::RuntimeError};
use deno_core::{error::AnyError, op};
use log::error;

#[op]
/// Expose parsing FsEvents to `Deno`
fn get_fsevents() -> Result<String, AnyError> {
    let fsevents_results = grab_fseventsd();
    let fsevents = match fsevents_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse fsevents: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };
    let results = serde_json::to_string_pretty(&fsevents)?;
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
            format: String::from("jsonl"),
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
    fn test_get_fsevents() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfZnNldmVudHMoKSB7CiAgICBjb25zdCBkYXRhID0gRGVub1tEZW5vLmludGVybmFsXS5jb3JlLm9wcy5nZXRfZnNldmVudHMoKTsKICAgIGlmIChkYXRhID09PSAiIikgewogICAgICAgIHJldHVybiBbXTsKICAgIH0KICAgIGNvbnN0IGZzZXZlbnRzID0gSlNPTi5wYXJzZShkYXRhKTsKICAgIHJldHVybiBmc2V2ZW50czsKfQpmdW5jdGlvbiBnZXRGc0V2ZW50cygpIHsKICAgIHJldHVybiBnZXRfZnNldmVudHMoKTsKfQpmdW5jdGlvbiBtYWluKCkgewogICAgY29uc3QgZGF0YSA9IGdldEZzRXZlbnRzKCk7CiAgICBjb25zdCByc19kYXRhID0gW107CiAgICBmb3IgKGNvbnN0IGVudHJ5IG9mIGRhdGEpewogICAgICAgIGlmIChlbnRyeS5wYXRoLmluY2x1ZGVzKCIucnMiKSkgewogICAgICAgICAgICByc19kYXRhLnB1c2goZW50cnkpOwogICAgICAgIH0KICAgIH0KICAgIHJldHVybiByc19kYXRhOwp9Cm1haW4oKTsKCg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("fsevent"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
