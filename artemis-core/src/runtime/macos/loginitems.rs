use crate::{
    artifacts::os::macos::loginitems::parser::grab_loginitems, runtime::error::RuntimeError,
};
use deno_core::{error::AnyError, op};
use log::error;

#[op]
/// Expose parsing LoginItems to `Deno`
fn get_loginitems() -> Result<String, AnyError> {
    let loginitems_results = grab_loginitems();
    let loginitems = match loginitems_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse loginitems: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };
    let results = serde_json::to_string_pretty(&loginitems)?;
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
    fn test_get_loginitems() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfbG9naW5pdGVtcygpIHsKICAgIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9sb2dpbml0ZW1zKCk7CiAgICBjb25zdCBpdGVtcyA9IEpTT04ucGFyc2UoZGF0YSk7CiAgICByZXR1cm4gaXRlbXM7Cn0KZnVuY3Rpb24gZ2V0TG9naW5JdGVtcygpIHsKICAgIHJldHVybiBnZXRfbG9naW5pdGVtcygpOwp9CmZ1bmN0aW9uIG1haW4oKSB7CiAgICBjb25zdCBkYXRhID0gZ2V0TG9naW5JdGVtcygpOwogICAgcmV0dXJuIGRhdGE7Cn0KbWFpbigpOwoK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("loginitems"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
