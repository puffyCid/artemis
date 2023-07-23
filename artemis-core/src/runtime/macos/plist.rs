use crate::artifacts::os::macos::plist::property_list::parse_plist_file;
use deno_core::{error::AnyError, op};
use log::error;

#[op]
/// Expose parsing plist file  to `Deno`
fn get_plist(path: String) -> Result<String, AnyError> {
    let plist_results = parse_plist_file(&path);
    let plist = match plist_results {
        Ok(results) => results,
        Err(err) => {
            // Parsing plist files could fail for many reasons
            // Instead of cancelling the whole script, return empty result
            error!("[runtime] Failed to parse plist: {err:?}");

            return Ok(String::new());
        }
    };
    let results = serde_json::to_string_pretty(&plist)?;
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
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        }
    }

    #[test]
    fn test_get_plist() {
        // Grabs and attempts to parse all plist files under /User/*, will recurse all directories
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbWFjb3MvcGxpc3QudHMKZnVuY3Rpb24gZ2V0UGxpc3QocGF0aCkgewogIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9wbGlzdChwYXRoKTsKICBpZiAoZGF0YSA9PT0gIiIpIHsKICAgIHJldHVybiBudWxsOwogIH0KICBjb25zdCBwbGlzdF9kYXRhID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gcGxpc3RfZGF0YTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9kaXJlY3RvcnkudHMKZnVuY3Rpb24gcmVhZERpcihwYXRoKSB7CiAgY29uc3QgZGF0YSA9IGZzLnJlYWREaXIocGF0aCk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIG1haW4udHMKYXN5bmMgZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBzdGFydF9wYXRoID0gIi9Vc2VycyI7CiAgY29uc3QgcGxpc3RfZmlsZXMgPSBbXTsKICBhd2FpdCByZWN1cnNlX2RpcihwbGlzdF9maWxlcywgc3RhcnRfcGF0aCk7CiAgcmV0dXJuIHBsaXN0X2ZpbGVzOwp9CmFzeW5jIGZ1bmN0aW9uIHJlY3Vyc2VfZGlyKHBsaXN0X2ZpbGVzLCBzdGFydF9wYXRoKSB7CiAgZm9yIGF3YWl0IChjb25zdCBlbnRyeSBvZiByZWFkRGlyKHN0YXJ0X3BhdGgpKSB7CiAgICBjb25zdCBwbGlzdF9wYXRoID0gYCR7c3RhcnRfcGF0aH0vJHtlbnRyeS5maWxlbmFtZX1gOwogICAgaWYgKGVudHJ5LmlzX2ZpbGUgJiYgZW50cnkuZmlsZW5hbWUuZW5kc1dpdGgoInBsaXN0IikpIHsKICAgICAgY29uc3QgZGF0YSA9IGdldFBsaXN0KHBsaXN0X3BhdGgpOwogICAgICBpZiAoZGF0YSA9PT0gbnVsbCkgewogICAgICAgIGNvbnRpbnVlOwogICAgICB9CiAgICAgIGNvbnN0IHBsaXN0X2luZm8gPSB7CiAgICAgICAgcGxpc3RfY29udGVudDogZGF0YSwKICAgICAgICBmaWxlOiBwbGlzdF9wYXRoCiAgICAgIH07CiAgICAgIHBsaXN0X2ZpbGVzLnB1c2gocGxpc3RfaW5mbyk7CiAgICAgIGNvbnRpbnVlOwogICAgfQogICAgaWYgKGVudHJ5LmlzX2RpcmVjdG9yeSkgewogICAgICBhd2FpdCByZWN1cnNlX2RpcihwbGlzdF9maWxlcywgcGxpc3RfcGF0aCk7CiAgICB9CiAgfQp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);

        let script = JSScript {
            name: String::from("plist_files"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
