use crate::artifacts::os::systeminfo::info::SystemInfo;
use deno_core::{error::AnyError, op};

#[op]
/// Expose pulling systeminfo to `Deno`
fn get_systeminfo() -> Result<String, AnyError> {
    let info = SystemInfo::get_info();
    let results = serde_json::to_string(&info)?;
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
    fn test_get_systeminfo() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3dpbmRvd3Mvc3lzdGVtaW5mby50cwpmdW5jdGlvbiBnZXRfc3lzdGVtaW5mb193aW4oKSB7CiAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X3N5c3RlbWluZm8oKTsKICBjb25zdCBpbmZvID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gaW5mbzsKfQoKLy8gLi4vLi4vYXJ0ZW1pcy1hcGkvbW9kLnRzCmZ1bmN0aW9uIGdldFN5c3RlbUluZm9XaW4oKSB7CiAgcmV0dXJuIGdldF9zeXN0ZW1pbmZvX3dpbigpOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgaW5mbyA9IGdldFN5c3RlbUluZm9XaW4oKTsKICByZXR1cm4gaW5mbzsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("systeminfo"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
