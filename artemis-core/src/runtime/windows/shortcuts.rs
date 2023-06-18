use crate::{
    artifacts::os::windows::shortcuts::parser::grab_lnk_file, runtime::error::RuntimeError,
};
use deno_core::{error::AnyError, op};
use log::error;

#[op]
fn get_lnk_file(path: String) -> Result<String, AnyError> {
    let lnk_result = grab_lnk_file(&path);
    let lnk = match lnk_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse shortcut file {path}: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let results = serde_json::to_string_pretty(&lnk)?;
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
        }
    }

    #[test]
    fn test_get_lnk_file() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfbG5rX2ZpbGUocGF0aCkgewogICAgY29uc3QgZGF0YSA9IERlbm9bRGVuby5pbnRlcm5hbF0uY29yZS5vcHMuZ2V0X2xua19maWxlKHBhdGgpOwogICAgY29uc3QgbG5rID0gSlNPTi5wYXJzZShkYXRhKTsKICAgIHJldHVybiBsbms7Cn0KZnVuY3Rpb24gZ2V0TG5rRmlsZShwYXRoKSB7CiAgICByZXR1cm4gZ2V0X2xua19maWxlKHBhdGgpOwp9CmZ1bmN0aW9uIG1haW4oKSB7CiAgICBjb25zdCBkcml2ZSA9IERlbm8uZW52LmdldCgiU3lzdGVtRHJpdmUiKTsKICAgIGlmIChkcml2ZSA9PT0gdW5kZWZpbmVkKSB7CiAgICAgICAgcmV0dXJuIFtdOwogICAgfQogICAgY29uc3QgdXNlcnMgPSBgJHtkcml2ZX1cXFVzZXJzYDsKICAgIGNvbnN0IHJlY2VudF9maWxlcyA9IFtdOwogICAgZm9yIChjb25zdCBlbnRyeSBvZiBEZW5vLnJlYWREaXJTeW5jKHVzZXJzKSl7CiAgICAgICAgdHJ5IHsKICAgICAgICAgICAgY29uc3QgcGF0aCA9IGAke3VzZXJzfVxcJHtlbnRyeS5uYW1lfVxcQXBwRGF0YVxcUm9hbWluZ1xcTWljcm9zb2Z0XFxXaW5kb3dzXFxSZWNlbnRgOwogICAgICAgICAgICBmb3IgKGNvbnN0IGVudHJ5MSBvZiBEZW5vLnJlYWREaXJTeW5jKHBhdGgpKXsKICAgICAgICAgICAgICAgIGlmICghZW50cnkxLm5hbWUuZW5kc1dpdGgoImxuayIpKSB7CiAgICAgICAgICAgICAgICAgICAgY29udGludWU7CiAgICAgICAgICAgICAgICB9CiAgICAgICAgICAgICAgICBjb25zdCBsbmtfZmlsZSA9IGAke3BhdGh9XFwke2VudHJ5MS5uYW1lfWA7CiAgICAgICAgICAgICAgICBjb25zdCBsbmsgPSBnZXRMbmtGaWxlKGxua19maWxlKTsKICAgICAgICAgICAgICAgIHJlY2VudF9maWxlcy5wdXNoKGxuayk7CiAgICAgICAgICAgIH0KICAgICAgICB9IGNhdGNoIChfZXJyb3IpIHsKICAgICAgICAgICAgY29udGludWU7CiAgICAgICAgfQogICAgfQogICAgcmV0dXJuIHJlY2VudF9maWxlczsKfQptYWluKCk7Cgo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("recent_files"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
