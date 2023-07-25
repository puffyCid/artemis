use crate::artifacts::os::linux::journals::parser::grab_journal_file;
use deno_core::{error::AnyError, op};

#[op]
/// Expose parsing journal file  to `Deno`
fn get_journal(path: String) -> Result<String, AnyError> {
    let elf_results = grab_journal_file(&path);
    let elf_data = match elf_results {
        Ok(results) => results,
        Err(_err) => {
            // Parsing Journal files could fail
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
    fn test_get_journal() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvdW5peC9zdWRvbG9ncy50cwpmdW5jdGlvbiBnZXRNYWNvc1N1ZG9Mb2dzKCkgewogIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9zdWRvbG9ncygpOwogIGNvbnN0IGxvZ19kYXRhID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gbG9nX2RhdGE7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBkYXRhID0gZ2V0TWFjb3NTdWRvTG9ncygpOwogIHJldHVybiBkYXRhOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);

        let script = JSScript {
            name: String::from("journal"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
