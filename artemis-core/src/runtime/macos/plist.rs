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
        let test = "ZnVuY3Rpb24gZ2V0X3BsaXN0KHBhdGgpIHsKICAgIGNvbnN0IGRhdGEgPSBEZW5vW0Rlbm8uaW50ZXJuYWxdLmNvcmUub3BzLmdldF9wbGlzdChwYXRoKTsKICAgIGlmIChkYXRhID09PSAiIikgewogICAgICAgIHJldHVybiB7CiAgICAgICAgICAgICJwbGlzdF9lcnJvciI6IGBmYWlsZWQgdG8gcGFyc2UgcGxpc3QgYXQgcGF0aDogJHtwYXRofWAKICAgICAgICB9OwogICAgfQogICAgY29uc3QgbG9nX2RhdGEgPSBKU09OLnBhcnNlKGRhdGEpOwogICAgcmV0dXJuIGxvZ19kYXRhOwp9CmZ1bmN0aW9uIGdldFBsaXN0KHBhdGgpIHsKICAgIHJldHVybiBnZXRfcGxpc3QocGF0aCk7Cn0KZnVuY3Rpb24gbWFpbigpIHsKICAgIGNvbnN0IHN0YXJ0X3BhdGggPSAiL1VzZXJzIjsKICAgIGNvbnN0IHBsaXN0X2ZpbGVzID0gW107CiAgICByZWN1cnNlX2RpcihwbGlzdF9maWxlcywgc3RhcnRfcGF0aCk7CiAgICByZXR1cm4gcGxpc3RfZmlsZXM7Cn0KZnVuY3Rpb24gcmVjdXJzZV9kaXIocGxpc3RfZmlsZXMsIHN0YXJ0X3BhdGgpIHsKICAgIGZvciAoY29uc3QgZW50cnkgb2YgRGVuby5yZWFkRGlyU3luYyhzdGFydF9wYXRoKSl7CiAgICAgICAgY29uc3QgcGxpc3RfcGF0aCA9IGAke3N0YXJ0X3BhdGh9LyR7ZW50cnkubmFtZX1gOwogICAgICAgIGlmIChlbnRyeS5pc0ZpbGUgJiYgZW50cnkubmFtZS5lbmRzV2l0aCgiLnBsaXN0IikpIHsKICAgICAgICAgICAgY29uc3QgZGF0YSA9IGdldFBsaXN0KHBsaXN0X3BhdGgpOwogICAgICAgICAgICBjb25zdCBwbGlzdF9pbmZvID0gewogICAgICAgICAgICAgICAgcGxpc3RfY29udGVudDogZGF0YSwKICAgICAgICAgICAgICAgIGZpbGU6IHBsaXN0X3BhdGgKICAgICAgICAgICAgfTsKICAgICAgICAgICAgcGxpc3RfZmlsZXMucHVzaChwbGlzdF9pbmZvKTsKICAgICAgICAgICAgY29udGludWU7CiAgICAgICAgfQogICAgICAgIGlmIChlbnRyeS5pc0RpcmVjdG9yeSkgewogICAgICAgICAgICByZWN1cnNlX2RpcihwbGlzdF9maWxlcywgcGxpc3RfcGF0aCk7CiAgICAgICAgfQogICAgfQp9Cm1haW4oKTs=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);

        let script = JSScript {
            name: String::from("plist_files"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
