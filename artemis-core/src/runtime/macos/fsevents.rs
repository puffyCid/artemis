use crate::artifacts::os::macos::fsevents::parser::grab_fsventsd_file;
use deno_core::{error::AnyError, op};
use log::error;

#[op]
/// Expose parsing FsEvents to `Deno`
fn get_fsevents(path: String) -> Result<String, AnyError> {
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
    let results = serde_json::to_string_pretty(&fsevents)?;
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
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbWFjb3MvZnNldmVudHMudHMKZnVuY3Rpb24gZ2V0X2ZzZXZlbnRzKHBhdGgpIHsKICBjb25zdCBkYXRhID0gRGVub1tEZW5vLmludGVybmFsXS5jb3JlLm9wcy5nZXRfZnNldmVudHMocGF0aCk7CiAgaWYgKGRhdGEgPT09ICIiKSB7CiAgICByZXR1cm4gbnVsbDsKICB9CiAgY29uc3QgZnNldmVudHMgPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiBmc2V2ZW50czsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9tb2QudHMKZnVuY3Rpb24gZ2V0RnNFdmVudHMocGF0aCkgewogIHJldHVybiBnZXRfZnNldmVudHMocGF0aCk7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBmc19kYXRhID0gW107CiAgY29uc3QgZnNldmVudHNfcGF0aCA9ICIvU3lzdGVtL1ZvbHVtZXMvRGF0YS8uZnNldmVudHNkIjsKICBmb3IgKGNvbnN0IGVudHJ5IG9mIERlbm8ucmVhZERpclN5bmMoZnNldmVudHNfcGF0aCkpIHsKICAgIGlmICghZW50cnkuaXNGaWxlKSB7CiAgICAgIGNvbnRpbnVlOwogICAgfQogICAgY29uc3QgZnNldmVudHNfZmlsZSA9IGAke2ZzZXZlbnRzX3BhdGh9LyR7ZW50cnkubmFtZX1gOwogICAgY29uc3QgaW5mbyA9IGdldEZzRXZlbnRzKGZzZXZlbnRzX2ZpbGUpOwogICAgaWYgKGluZm8gPT09IG51bGwpIHsKICAgICAgY29udGludWU7CiAgICB9CiAgICBmb3IgKGNvbnN0IGZzZXZlbnRfZW50cnkgb2YgaW5mbykgewogICAgICBpZiAoIWZzZXZlbnRfZW50cnkucGF0aC5pbmNsdWRlcygiLnJzIikpIHsKICAgICAgICBjb250aW51ZTsKICAgICAgfQogICAgICBmc19kYXRhLnB1c2goZnNldmVudF9lbnRyeSk7CiAgICB9CiAgfQogIHJldHVybiBmc19kYXRhOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", true);
        let script = JSScript {
            name: String::from("fsevent"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
