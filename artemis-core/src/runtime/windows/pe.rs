use crate::artifacts::os::windows::pe::parser::parse_pe_file;
use deno_core::{error::AnyError, op};

#[op]
/// Expose parsing pe file  to `Deno`
fn get_pe(path: String) -> Result<String, AnyError> {
    let pe_results = parse_pe_file(&path);
    let pe = match pe_results {
        Ok(results) => results,
        Err(_err) => {
            // Parsing pe files could fail for many reasons
            // Instead of cancelling the whole script, return empty result
            return Ok(String::new());
        }
    };
    let results = serde_json::to_string(&pe)?;
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
    fn test_get_pe() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfcGUocGF0aCkgewogICAgY29uc3QgZGF0YSA9IERlbm9bRGVuby5pbnRlcm5hbF0uY29yZS5vcHMuZ2V0X3BlKHBhdGgpOwogICAgaWYgKGRhdGEgPT09ICIiKSB7CiAgICAgICAgcmV0dXJuIG51bGw7CiAgICB9CiAgICBjb25zdCBwZSA9IEpTT04ucGFyc2UoZGF0YSk7CiAgICByZXR1cm4gcGU7Cn0KZnVuY3Rpb24gZ2V0UGUocGF0aCkgewogICAgcmV0dXJuIGdldF9wZShwYXRoKTsKfQpmdW5jdGlvbiBtYWluKCkgewogICAgY29uc3QgZHJpdmUgPSBEZW5vLmVudi5nZXQoIlN5c3RlbURyaXZlIik7CiAgICBpZiAoZHJpdmUgPT09IHVuZGVmaW5lZCkgewogICAgICAgIHJldHVybiBbXTsKICAgIH0KICAgIGNvbnN0IHBhdGggPSBgJHtkcml2ZX1cXFdpbmRvd3NcXFN5c3RlbTMyYDsKICAgIGNvbnN0IHBlcyA9IFtdOwogICAgZm9yIChjb25zdCBlbnRyeSBvZiBEZW5vLnJlYWREaXJTeW5jKHBhdGgpKXsKICAgICAgICBpZiAoIWVudHJ5LmlzRmlsZSkgewogICAgICAgICAgICBjb250aW51ZTsKICAgICAgICB9CiAgICAgICAgY29uc3QgcGVfcGF0aCA9IGAke3BhdGh9XFwke2VudHJ5Lm5hbWV9YDsKICAgICAgICBjb25zdCBpbmZvID0gZ2V0UGUocGVfcGF0aCk7CiAgICAgICAgaWYgKGluZm8gPT09IG51bGwpIHsKICAgICAgICAgICAgY29udGludWU7CiAgICAgICAgfQogICAgICAgIGNvbnN0IG1ldGEgPSB7CiAgICAgICAgICAgIHBhdGg6IHBlX3BhdGgsCiAgICAgICAgICAgIHBlOiBpbmZvCiAgICAgICAgfTsKICAgICAgICBwZXMucHVzaChtZXRhKTsKICAgIH0KICAgIHJldHVybiBwZXM7Cn0KbWFpbigpOwoK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);

        let script = JSScript {
            name: String::from("system32_pe"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
