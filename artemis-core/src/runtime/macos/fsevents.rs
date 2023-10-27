use crate::artifacts::os::macos::fsevents::parser::grab_fsventsd_file;
use deno_core::{error::AnyError, op2};
use log::error;

#[op2]
#[string]
/// Expose parsing FsEvents to `Deno`
fn get_fsevents(#[string] path: String) -> Result<String, AnyError> {
    let fsevents_results = grab_fsventsd_file(&path);
    let fsevents = match fsevents_results {
        Ok(results) => results,
        Err(err) => {
            // A user may submit a non-fsevent file
            // Instead of cancelling the whole script, return empty result
            error!("[runtime] Failed to parse fsevents file: {err:?}");
            return Ok(String::new());
        }
    };
    let results = serde_json::to_string(&fsevents)?;
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
    fn test_get_fsevents() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbWFjb3MvZnNldmVudHMudHMKZnVuY3Rpb24gZ2V0RnNldmVudHMocGF0aCkgewogIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9mc2V2ZW50cyhwYXRoKTsKICBpZiAoZGF0YSA9PT0gIiIpIHsKICAgIHJldHVybiBudWxsOwogIH0KICBjb25zdCBmc2V2ZW50cyA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGZzZXZlbnRzOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9maWxlc3lzdGVtL2RpcmVjdG9yeS50cwphc3luYyBmdW5jdGlvbiByZWFkRGlyKHBhdGgpIHsKICBjb25zdCBkYXRhID0gSlNPTi5wYXJzZShhd2FpdCBmcy5yZWFkRGlyKHBhdGgpKTsKICByZXR1cm4gZGF0YTsKfQoKLy8gbWFpbi50cwphc3luYyBmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IGZzX2RhdGEgPSBbXTsKICBjb25zdCBmc2V2ZW50c19wYXRoID0gIi9TeXN0ZW0vVm9sdW1lcy9EYXRhLy5mc2V2ZW50c2QiOwogIGZvciAoY29uc3QgZW50cnkgb2YgYXdhaXQgcmVhZERpcihmc2V2ZW50c19wYXRoKSkgewogICAgaWYgKCFlbnRyeS5pc19maWxlKSB7CiAgICAgIGNvbnRpbnVlOwogICAgfQogICAgY29uc3QgZnNldmVudHNfZmlsZSA9IGAke2ZzZXZlbnRzX3BhdGh9LyR7ZW50cnkuZmlsZW5hbWV9YDsKICAgIGNvbnN0IGluZm8gPSBnZXRGc2V2ZW50cyhmc2V2ZW50c19maWxlKTsKICAgIGlmIChpbmZvID09PSBudWxsKSB7CiAgICAgIGNvbnRpbnVlOwogICAgfQogICAgZm9yIChjb25zdCBmc2V2ZW50X2VudHJ5IG9mIGluZm8pIHsKICAgICAgaWYgKCFmc2V2ZW50X2VudHJ5LnBhdGguaW5jbHVkZXMoIi5ycyIpKSB7CiAgICAgICAgY29udGludWU7CiAgICAgIH0KICAgICAgZnNfZGF0YS5wdXNoKGZzZXZlbnRfZW50cnkpOwogICAgfQogIH0KICByZXR1cm4gZnNfZGF0YTsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", true);
        let script = JSScript {
            name: String::from("fsevent"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
