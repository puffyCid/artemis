use crate::artifacts::os::windows::ese::parser::grab_ese_tables;
use deno_core::{error::AnyError, op2};

#[op2]
#[string]
pub(crate) fn get_table(
    #[string] path: String,
    #[serde] table: Vec<String>,
) -> Result<String, AnyError> {
    let ese = grab_ese_tables(&path, &table)?;

    let results = serde_json::to_string(&ese)?;
    Ok(results)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
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
    fn test_get_table() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3dpbmRvd3MvZXNlLnRzCmZ1bmN0aW9uIHBhcnNlVGFibGUocGF0aCwgdGFibGVzKSB7CiAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X3RhYmxlKHBhdGgsIHRhYmxlcyk7CiAgaWYgKGRhdGEgaW5zdGFuY2VvZiBFcnJvcikgewogICAgcmV0dXJuIGRhdGE7CiAgfQogIGNvbnN0IHJlc3VsdHMgPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiByZXN1bHRzOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgZGIgPSAiQzpcXFdpbmRvd3NcXHNlY3VyaXR5XFxkYXRhYmFzZVxcc2VjZWRpdC5zZGIiOwogIGNvbnN0IHRhYmxlcyA9IFsiU21UYmxTbXAiXTsKICBjb25zdCBkYXRhID0gcGFyc2VUYWJsZShkYiwgdGFibGVzKTsKICBjb25zb2xlLmxvZyhgRVNFICR7dGFibGVzWzBdfSBsZW46ICR7ZGF0YVsiU21UYmxTbXAiXS5sZW5ndGh9YCk7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("recent_files"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
