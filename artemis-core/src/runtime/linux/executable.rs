use crate::artifacts::os::linux::executable::parser::parse_elf_file;
use deno_core::{error::AnyError, op2};

#[op2]
#[string]
/// Expose parsing elf file  to `Deno`
fn get_elf(#[string] path: String) -> Result<String, AnyError> {
    let elf_results = parse_elf_file(&path);
    let elf_data = match elf_results {
        Ok(results) => results,
        Err(_err) => {
            // Parsing elf files could fail for many reasons
            // Instead of cancelling the whole script, return empty result
            return Ok(String::new());
        }
    };
    let results = serde_json::to_string(&elf_data)?;
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
    fn test_get_elf() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbGludXgvZWxmLnRzCmZ1bmN0aW9uIGdldEVsZihwYXRoKSB7CiAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X2VsZihwYXRoKTsKICBpZiAoZGF0YSA9PT0gIiIpIHsKICAgIHJldHVybiBudWxsOwogIH0KICBjb25zdCBlbGYgPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiBlbGY7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL2ZpbGVzeXN0ZW0vZGlyZWN0b3J5LnRzCmFzeW5jIGZ1bmN0aW9uIHJlYWREaXIocGF0aCkgewogIGNvbnN0IGRhdGEgPSBKU09OLnBhcnNlKGF3YWl0IGZzLnJlYWREaXIocGF0aCkpOwogIHJldHVybiBkYXRhOwp9CgovLyBtYWluLnRzCmFzeW5jIGZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgYmluX3BhdGggPSAiL2JpbiI7CiAgY29uc3QgZWxmcyA9IFtdOwogIGZvciAoY29uc3QgZW50cnkgb2YgYXdhaXQgcmVhZERpcihiaW5fcGF0aCkpIHsKICAgIGlmICghZW50cnkuaXNfZmlsZSkgewogICAgICBjb250aW51ZTsKICAgIH0KICAgIGNvbnN0IGVsZl9wYXRoID0gYCR7YmluX3BhdGh9LyR7ZW50cnkuZmlsZW5hbWV9YDsKICAgIGNvbnN0IGluZm8gPSBnZXRFbGYoZWxmX3BhdGgpOwogICAgaWYgKGluZm8gPT09IG51bGwpIHsKICAgICAgY29udGludWU7CiAgICB9CiAgICBjb25zdCBtZXRhID0gewogICAgICBwYXRoOiBlbGZfcGF0aCwKICAgICAgZWxmOiBpbmZvCiAgICB9OwogICAgZWxmcy5wdXNoKG1ldGEpOwogIH0KICByZXR1cm4gZWxmczsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);

        let script = JSScript {
            name: String::from("elf"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
