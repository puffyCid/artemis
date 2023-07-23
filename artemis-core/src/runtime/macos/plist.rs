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
    let results = serde_json::to_string(&plist)?;
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
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbWFjb3MvcGxpc3QudHMKZnVuY3Rpb24gZ2V0UGxpc3QocGF0aCkgewogIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9wbGlzdChwYXRoKTsKICBpZiAoZGF0YSA9PT0gIiIpIHsKICAgIHJldHVybiBudWxsOwogIH0KICBjb25zdCBwbGlzdF9kYXRhID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gcGxpc3RfZGF0YTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvc3lzdGVtL291dHB1dC50cwpmdW5jdGlvbiBvdXRwdXRSZXN1bHRzKGRhdGEsIGRhdGFfbmFtZSwgb3V0cHV0KSB7CiAgY29uc3Qgb3V0cHV0X3N0cmluZyA9IEpTT04uc3RyaW5naWZ5KG91dHB1dCk7CiAgY29uc3Qgc3RhdHVzID0gRGVuby5jb3JlLm9wcy5vdXRwdXRfcmVzdWx0cygKICAgIGRhdGEsCiAgICBkYXRhX25hbWUsCiAgICBvdXRwdXRfc3RyaW5nCiAgKTsKICByZXR1cm4gc3RhdHVzOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9maWxlc3lzdGVtL2RpcmVjdG9yeS50cwpmdW5jdGlvbiByZWFkRGlyKHBhdGgpIHsKICBjb25zdCBkYXRhID0gZnMucmVhZERpcihwYXRoKTsKICByZXR1cm4gZGF0YTsKfQoKLy8gbWFpbi50cwphc3luYyBmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHN0YXJ0X3BhdGggPSAiL1VzZXJzIjsKICBjb25zdCBwbGlzdF9maWxlcyA9IFtdOwogIGF3YWl0IHJlY3Vyc2VfZGlyKHBsaXN0X2ZpbGVzLCBzdGFydF9wYXRoKTsKICByZXR1cm4gcGxpc3RfZmlsZXM7Cn0KYXN5bmMgZnVuY3Rpb24gcmVjdXJzZV9kaXIocGxpc3RfZmlsZXMsIHN0YXJ0X3BhdGgpIHsKICBpZiAocGxpc3RfZmlsZXMubGVuZ3RoID4gMjApIHsKICAgIGNvbnN0IG91dCA9IHsKICAgICAgbmFtZTogImFydGVtaXNfcGxpc3QiLAogICAgICBkaXJlY3Rvcnk6ICIuL3RtcCIsCiAgICAgIGZvcm1hdDogImpzb24iIC8qIEpTT04gKi8sCiAgICAgIGNvbXByZXNzOiBmYWxzZSwKICAgICAgZW5kcG9pbnRfaWQ6ICJhbnl0aGluZy1pLXdhbnQiLAogICAgICBjb2xsZWN0aW9uX2lkOiAxLAogICAgICBvdXRwdXQ6ICJsb2NhbCIgLyogTE9DQUwgKi8KICAgIH07CiAgICBjb25zdCBzdGF0dXMgPSBvdXRwdXRSZXN1bHRzKEpTT04uc3RyaW5naWZ5KHBsaXN0X2ZpbGVzKSwgImFydGVtaXNfaW5mbyIsIG91dCk7CiAgICBpZiAoIXN0YXR1cykgewogICAgICBjb25zb2xlLmxvZygiQ291bGQgbm90IG91dHB1dCB0byBsb2NhbCBkaXJlY3RvcnkiKTsKICAgIH0KICAgIHBsaXN0X2ZpbGVzID0gW107CiAgfQogIGZvciBhd2FpdCAoY29uc3QgZW50cnkgb2YgcmVhZERpcihzdGFydF9wYXRoKSkgewogICAgY29uc3QgcGxpc3RfcGF0aCA9IGAke3N0YXJ0X3BhdGh9LyR7ZW50cnkuZmlsZW5hbWV9YDsKICAgIGlmIChlbnRyeS5pc19maWxlICYmIGVudHJ5LmZpbGVuYW1lLmVuZHNXaXRoKCJwbGlzdCIpKSB7CiAgICAgIGNvbnN0IGRhdGEgPSBnZXRQbGlzdChwbGlzdF9wYXRoKTsKICAgICAgaWYgKGRhdGEgPT09IG51bGwpIHsKICAgICAgICBjb250aW51ZTsKICAgICAgfQogICAgICBjb25zdCBwbGlzdF9pbmZvID0gewogICAgICAgIHBsaXN0X2NvbnRlbnQ6IGRhdGEsCiAgICAgICAgZmlsZTogcGxpc3RfcGF0aAogICAgICB9OwogICAgICBwbGlzdF9maWxlcy5wdXNoKHBsaXN0X2luZm8pOwogICAgICBjb250aW51ZTsKICAgIH0KICAgIGlmIChlbnRyeS5pc19kaXJlY3RvcnkpIHsKICAgICAgYXdhaXQgcmVjdXJzZV9kaXIocGxpc3RfZmlsZXMsIHBsaXN0X3BhdGgpOwogICAgfQogIH0KfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);

        let script = JSScript {
            name: String::from("plist_files"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
