use crate::{artifacts::os::macos::emond::parser::grab_emond, runtime::error::RuntimeError};
use deno_core::{error::AnyError, op2};
use log::error;

#[op2]
#[string]
/// Expose parsing Emond to `Deno`
pub(crate) fn get_emond() -> Result<String, AnyError> {
    let emond_results = grab_emond();
    let emond = match emond_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse emond: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };
    let results = serde_json::to_string(&emond)?;
    Ok(results)
}

#[cfg(test)]
mod tests {
    use crate::{
        runtime::deno::execute_script, structs::artifacts::runtime::script::JSScript,
        structs::toml::Output,
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
    fn test_get_emond() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfZW1vbmQoKSB7CiAgICBjb25zdCBkYXRhID0gRGVuby5jb3JlLm9wcy5nZXRfZW1vbmQoKTsKICAgIGNvbnN0IGVtb25kID0gSlNPTi5wYXJzZShkYXRhKTsKICAgIHJldHVybiBlbW9uZDsKfQpmdW5jdGlvbiBnZXRFbW9uZCgpIHsKICAgIHJldHVybiBnZXRfZW1vbmQoKTsKfQpmdW5jdGlvbiBtYWluKCkgewogICAgY29uc3QgZGF0YSA9IGdldEVtb25kKCk7CiAgICByZXR1cm4gZGF0YTsKfQptYWluKCk7Cgo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("emond"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
