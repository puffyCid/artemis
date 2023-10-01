use crate::{runtime::deno::output_data, structs::toml::Output};
use deno_core::{error::AnyError, op};
use log::error;
use serde_json::Value;

#[op]
fn output_results(
    data: String,
    output_name: String,
    output_format: String,
) -> Result<bool, AnyError> {
    let sucess = true;
    let failure = false;

    let serde_result = serde_json::from_str(&data);
    let serde_data: Value = match serde_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed serialize script data: {err:?}");
            return Ok(failure);
        }
    };

    let output_result = serde_json::from_str(&output_format);
    let mut output: Output = match output_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed deserialize output format data: {err:?}");
            return Ok(failure);
        }
    };

    let empty_start = 0;
    if output_data(&serde_data, &output_name, &mut output, &empty_start).is_err() {
        error!("[runtime] Failed could not output script data");
        return Ok(failure);
    }

    Ok(sucess)
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
    fn test_output_results() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy9wcm9jZXNzZXMudHMKZnVuY3Rpb24gZ2V0X3dpbl9wcm9jZXNzZXMobWQ1LCBzaGExLCBzaGEyNTYsIHBlX2luZm8pIHsKICBjb25zdCBoYXNoZXMgPSB7CiAgICBtZDUsCiAgICBzaGExLAogICAgc2hhMjU2CiAgfTsKICBjb25zdCBkYXRhID0gRGVuby5jb3JlLm9wcy5nZXRfcHJvY2Vzc2VzKAogICAgSlNPTi5zdHJpbmdpZnkoaGFzaGVzKSwKICAgIHBlX2luZm8KICApOwogIGNvbnN0IHByb2NfYXJyYXkgPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiBwcm9jX2FycmF5Owp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9zeXN0ZW0vb3V0cHV0LnRzCmZ1bmN0aW9uIG91dHB1dFJlc3VsdHMoZGF0YSwgZGF0YV9uYW1lLCBvdXRwdXQpIHsKICBjb25zdCBvdXRwdXRfc3RyaW5nID0gSlNPTi5zdHJpbmdpZnkob3V0cHV0KTsKICBjb25zdCBzdGF0dXMgPSBEZW5vLmNvcmUub3BzLm91dHB1dF9yZXN1bHRzKAogICAgZGF0YSwKICAgIGRhdGFfbmFtZSwKICAgIG91dHB1dF9zdHJpbmcKICApOwogIHJldHVybiBzdGF0dXM7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvbW9kLnRzCmZ1bmN0aW9uIGdldFdpblByb2Nlc3NlcyhtZDUsIHNoYTEsIHNoYTI1NiwgcGVfaW5mbykgewogIHJldHVybiBnZXRfd2luX3Byb2Nlc3NlcyhtZDUsIHNoYTEsIHNoYTI1NiwgcGVfaW5mbyk7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBtZDUgPSB0cnVlOwogIGNvbnN0IHNoYTEgPSBmYWxzZTsKICBjb25zdCBzaGEyNTYgPSBmYWxzZTsKICBjb25zdCBwZV9pbmZvID0gdHJ1ZTsKICBjb25zdCBwcm9jX2xpc3QgPSBnZXRXaW5Qcm9jZXNzZXMobWQ1LCBzaGExLCBzaGEyNTYsIHBlX2luZm8pOwogIGZvciAoY29uc3QgZW50cnkgb2YgcHJvY19saXN0KSB7CiAgICBpZiAoZW50cnkubmFtZS5pbmNsdWRlcygiYXJ0ZW1pcyIpKSB7CiAgICAgIGNvbnN0IG91dCA9IHsKICAgICAgICBuYW1lOiAiYXJ0ZW1pc19wcm9jIiwKICAgICAgICBkaXJlY3Rvcnk6ICIuL3RtcCIsCiAgICAgICAgZm9ybWF0OiAianNvbiIgLyogSlNPTiAqLywKICAgICAgICBjb21wcmVzczogZmFsc2UsCiAgICAgICAgZW5kcG9pbnRfaWQ6ICJhbnl0aGluZy1pLXdhbnQiLAogICAgICAgIGNvbGxlY3Rpb25faWQ6IDEsCiAgICAgICAgb3V0cHV0OiAibG9jYWwiIC8qIExPQ0FMICovCiAgICAgIH07CiAgICAgIGNvbnN0IHN0YXR1cyA9IG91dHB1dFJlc3VsdHMoSlNPTi5zdHJpbmdpZnkoZW50cnkpLCAiYXJ0ZW1pc19pbmZvIiwgb3V0KTsKICAgICAgaWYgKCFzdGF0dXMpIHsKICAgICAgICBjb25zb2xlLmxvZygiQ291bGQgbm90IG91dHB1dCB0byBsb2NhbCBkaXJlY3RvcnkiKTsKICAgICAgfQogICAgfQogIH0KfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", true);
        let script = JSScript {
            name: String::from("output_results"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_output_results_jsonl_compress() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy9wcm9jZXNzZXMudHMKZnVuY3Rpb24gZ2V0X3dpbl9wcm9jZXNzZXMobWQ1LCBzaGExLCBzaGEyNTYsIHBlX2luZm8pIHsKICBjb25zdCBoYXNoZXMgPSB7CiAgICBtZDUsCiAgICBzaGExLAogICAgc2hhMjU2CiAgfTsKICBjb25zdCBkYXRhID0gRGVuby5jb3JlLm9wcy5nZXRfcHJvY2Vzc2VzKAogICAgSlNPTi5zdHJpbmdpZnkoaGFzaGVzKSwKICAgIHBlX2luZm8KICApOwogIGNvbnN0IHByb2NfYXJyYXkgPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiBwcm9jX2FycmF5Owp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9zeXN0ZW0vb3V0cHV0LnRzCmZ1bmN0aW9uIG91dHB1dFJlc3VsdHMoZGF0YSwgZGF0YV9uYW1lLCBvdXRwdXQpIHsKICBjb25zdCBvdXRwdXRfc3RyaW5nID0gSlNPTi5zdHJpbmdpZnkob3V0cHV0KTsKICBjb25zdCBzdGF0dXMgPSBEZW5vLmNvcmUub3BzLm91dHB1dF9yZXN1bHRzKAogICAgZGF0YSwKICAgIGRhdGFfbmFtZSwKICAgIG91dHB1dF9zdHJpbmcKICApOwogIHJldHVybiBzdGF0dXM7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvbW9kLnRzCmZ1bmN0aW9uIGdldFdpblByb2Nlc3NlcyhtZDUsIHNoYTEsIHNoYTI1NiwgcGVfaW5mbykgewogIHJldHVybiBnZXRfd2luX3Byb2Nlc3NlcyhtZDUsIHNoYTEsIHNoYTI1NiwgcGVfaW5mbyk7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBtZDUgPSBmYWxzZTsKICBjb25zdCBzaGExID0gZmFsc2U7CiAgY29uc3Qgc2hhMjU2ID0gZmFsc2U7CiAgY29uc3QgcGVfaW5mbyA9IGZhbHNlOwogIGNvbnN0IHByb2NfbGlzdCA9IGdldFdpblByb2Nlc3NlcyhtZDUsIHNoYTEsIHNoYTI1NiwgcGVfaW5mbyk7CiAgZm9yIChjb25zdCBlbnRyeSBvZiBwcm9jX2xpc3QpIHsKICAgIGlmIChlbnRyeS5uYW1lLmluY2x1ZGVzKCJhcnRlbWlzIikpIHsKICAgICAgY29uc3Qgb3V0ID0gewogICAgICAgIG5hbWU6ICJhcnRlbWlzX3Byb2MiLAogICAgICAgIGRpcmVjdG9yeTogIi4vdG1wIiwKICAgICAgICBmb3JtYXQ6ICJqc29ubCIgLyogSlNPTiAqLywKICAgICAgICBjb21wcmVzczogdHJ1ZSwKICAgICAgICBlbmRwb2ludF9pZDogImFueXRoaW5nLWktd2FudCIsCiAgICAgICAgY29sbGVjdGlvbl9pZDogMSwKICAgICAgICBvdXRwdXQ6ICJsb2NhbCIgLyogTE9DQUwgKi8KICAgICAgfTsKICAgICAgY29uc3Qgc3RhdHVzID0gb3V0cHV0UmVzdWx0cyhKU09OLnN0cmluZ2lmeShlbnRyeSksICJhcnRlbWlzX2luZm8iLCBvdXQpOwogICAgICBpZiAoIXN0YXR1cykgewogICAgICAgIGNvbnNvbGUubG9nKCJDb3VsZCBub3Qgb3V0cHV0IHRvIGxvY2FsIGRpcmVjdG9yeSIpOwogICAgICB9CiAgICB9CiAgfQp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", true);
        let script = JSScript {
            name: String::from("output_results"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
