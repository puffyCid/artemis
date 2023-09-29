use crate::{artifacts::os::windows::srum::parser::grab_srum_path, runtime::error::RuntimeError};
use deno_core::{error::AnyError, op};
use log::error;

#[op]
/// Expose parsing a single SRUM table to `Deno`
fn get_srum(path: String, table: String) -> Result<String, AnyError> {
    if path.is_empty() {
        error!("[runtime] Empty path to SRUM file");
        return Err(RuntimeError::ExecuteScript.into());
    } else if table.is_empty() {
        error!("[runtime] Empty SRUM table to dump");
        return Err(RuntimeError::ExecuteScript.into());
    }
    let srum_results = grab_srum_path(&path, &table);
    let srum = match srum_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse SRUM: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let results = serde_json::to_string(&srum)?;
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
    fn test_get_srum() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfc3J1bV9hcHBsaWNhdGlvbl9pbmZvKHBhdGgpIHsKICAgIGNvbnN0IG5hbWUgPSAie0QxMENBMkZFLTZGQ0YtNEY2RC04NDhFLUIyRTk5MjY2RkE4OX0iOwogICAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X3NydW0ocGF0aCwgbmFtZSk7CiAgICBjb25zdCBzcnVtID0gSlNPTi5wYXJzZShkYXRhKTsKICAgIHJldHVybiBzcnVtOwp9CmZ1bmN0aW9uIGdldFNydW1BcHBsaWNhdGlvbkluZm8ocGF0aCkgewogICAgcmV0dXJuIGdldF9zcnVtX2FwcGxpY2F0aW9uX2luZm8ocGF0aCk7Cn0KZnVuY3Rpb24gbWFpbigpIHsKICAgIGNvbnN0IHBhdGggPSAiQzpcXFdpbmRvd3NcXFN5c3RlbTMyXFxzcnVcXFNSVURCLmRhdCI7CiAgICBjb25zdCBlbnRyaWVzID0gZ2V0U3J1bUFwcGxpY2F0aW9uSW5mbyhwYXRoKTsKICAgIHJldHVybiBlbnRyaWVzOwp9Cm1haW4oKTsKCg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("srum"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
