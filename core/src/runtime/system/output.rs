use crate::{output::formats::{json::raw_json, jsonl::raw_jsonl}, runtime::deno::output_data, structs::toml::Output};
use deno_core::{error::AnyError, op2};
use log::error;
use serde_json::Value;

#[op2(fast)]
pub(crate) fn output_results(
    #[string] data: String,
    #[string] output_name: String,
    #[string] output_format: String,
) -> Result<bool, AnyError> {
    let sucess = true;

    let serde_result = serde_json::from_str(&data);
    let serde_data: Value = match serde_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed deserialize script data: {err:?}");
            return Err(err.into());
        }
    };

    let output_result = serde_json::from_str(&output_format);
    let mut output: Output = match output_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed deserialize output format data: {err:?}");
            return Err(err.into());
        }
    };

    let empty_start = 0;
    let status = output_data(&serde_data, &output_name, &mut output, &empty_start);
    if status.is_err() {
        error!("[runtime] Failed could not output script data:");
        return Err(status.unwrap_err().into());
    }

    Ok(sucess)
}

#[op2(fast)]
pub(crate) fn raw_dump(
    #[string] data: String,
    #[string] output_name: String,
    #[string] output_format: String,
) -> Result<bool, AnyError> {
    let sucess = true;

    let serde_result = serde_json::from_str(&data);
    let serde_data: Value = match serde_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed deserialize script data: {err:?}");
            return Err(err.into());
        }
    };

    let output_result = serde_json::from_str(&output_format);
    let mut output: Output = match output_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed deserialize output format data: {err:?}");
            return Err(err.into());
        }
    };

    if output.format == "jsonl" {
       raw_jsonl(&serde_data, &output_name, &mut output)?;
    } else if output.format == "json" {
        raw_json(&serde_data, &output_name, &mut output)?;
    } else {
        return Err(AnyError::msg(format!("bad format: {}", output.format)));
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

    #[test]
    fn test_raw_dump() {
        let test = "Ly8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3V0aWxzL2Vycm9yLnRzCnZhciBFcnJvckJhc2UgPSBjbGFzcyBleHRlbmRzIEVycm9yIHsKICBjb25zdHJ1Y3RvcihuYW1lLCBtZXNzYWdlKSB7CiAgICBzdXBlcigpOwogICAgdGhpcy5uYW1lID0gbmFtZTsKICAgIHRoaXMubWVzc2FnZSA9IG1lc3NhZ2U7CiAgfQp9OwoKLy8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL2ZpbGVzeXN0ZW0vZXJyb3JzLnRzCnZhciBGaWxlRXJyb3IgPSBjbGFzcyBleHRlbmRzIEVycm9yQmFzZSB7Cn07CgovLyAuLi8uLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvZmlsZXN5c3RlbS9maWxlcy50cwpmdW5jdGlvbiBnbG9iKHBhdHRlcm4pIHsKICB0cnkgewogICAgY29uc3QgcmVzdWx0ID0gZnMuZ2xvYihwYXR0ZXJuKTsKICAgIGNvbnN0IGRhdGEgPSBKU09OLnBhcnNlKHJlc3VsdCk7CiAgICByZXR1cm4gZGF0YTsKICB9IGNhdGNoIChlcnIpIHsKICAgIHJldHVybiBuZXcgRmlsZUVycm9yKCJHTE9CIiwgYGZhaWxlZCB0byBnbG9iIHBhdHRlcm4gJHtwYXR0ZXJufSIgJHtlcnJ9YCk7CiAgfQp9CgovLyAuLi8uLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvYXBwbGljYXRpb25zL2Vycm9ycy50cwp2YXIgQXBwbGljYXRpb25FcnJvciA9IGNsYXNzIGV4dGVuZHMgRXJyb3JCYXNlIHsKfTsKCi8vIC4uLy4uL1Byb2plY3RzL2FydGVtaXMtYXBpL3NyYy9hcHBsaWNhdGlvbnMvc3FsaXRlLnRzCmZ1bmN0aW9uIHF1ZXJ5U3FsaXRlKHBhdGgsIHF1ZXJ5KSB7CiAgdHJ5IHsKICAgIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLnF1ZXJ5X3NxbGl0ZShwYXRoLCBxdWVyeSk7CiAgICBjb25zdCByZXN1bHRzID0gSlNPTi5wYXJzZShkYXRhKTsKICAgIHJldHVybiByZXN1bHRzOwogIH0gY2F0Y2ggKGVycikgewogICAgcmV0dXJuIG5ldyBBcHBsaWNhdGlvbkVycm9yKAogICAgICAiU1FMSVRFIiwKICAgICAgYGZhaWxlZCB0byBleGVjdXRlIHF1ZXJ5ICR7ZXJyfWAKICAgICk7CiAgfQp9CgovLyAuLi8uLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvdGltZS9jb252ZXJzaW9uLnRzCmZ1bmN0aW9uIGNvY29hdGltZVRvVW5peEVwb2NoKGNvY29hdGltZSkgewogIGNvbnN0IGRhdGEgPSB0aW1lLmNvY29hdGltZV90b191bml4ZXBvY2goY29jb2F0aW1lKTsKICByZXR1cm4gTnVtYmVyKGRhdGEpOwp9CgovLyAuLi8uLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvbWFjb3MvZXJyb3JzLnRzCnZhciBNYWNvc0Vycm9yID0gY2xhc3MgZXh0ZW5kcyBFcnJvckJhc2Ugewp9OwoKLy8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL21hY29zL3NxbGl0ZS9xdWFyYW50aW5lLnRzCmZ1bmN0aW9uIHF1YXJhbnRpbmVFdmVudHMoYWx0X2ZpbGUpIHsKICBsZXQgcGF0aHMgPSBbXTsKICBpZiAoYWx0X2ZpbGUgIT0gdm9pZCAwKSB7CiAgICBwYXRocyA9IFthbHRfZmlsZV07CiAgfSBlbHNlIHsKICAgIGNvbnN0IGdsb2JfcGF0aCA9ICIvVXNlcnMvKi9MaWJyYXJ5L1ByZWZlcmVuY2VzL2NvbS5hcHBsZS5MYXVuY2hTZXJ2aWNlcy5RdWFyYW50aW5lRXZlbnRzVjIiOwogICAgY29uc3QgcGF0aHNfcmVzdWx0cyA9IGdsb2IoZ2xvYl9wYXRoKTsKICAgIGlmIChwYXRoc19yZXN1bHRzIGluc3RhbmNlb2YgRmlsZUVycm9yKSB7CiAgICAgIHJldHVybiBuZXcgTWFjb3NFcnJvcigKICAgICAgICBgUVVBUkFOVElORV9FVkVOVGAsCiAgICAgICAgYGZhaWxlZCB0byBnbG9iIHBhdGg6ICR7Z2xvYl9wYXRofTogJHtwYXRoc19yZXN1bHRzfWAKICAgICAgKTsKICAgIH0KICAgIGZvciAoY29uc3QgZW50cnkgb2YgcGF0aHNfcmVzdWx0cykgewogICAgICBwYXRocy5wdXNoKGVudHJ5LmZ1bGxfcGF0aCk7CiAgICB9CiAgfQogIGNvbnN0IHF1ZXJ5ID0gInNlbGVjdCAqIGZyb20gTFNRdWFyYW50aW5lRXZlbnQiOwogIGNvbnN0IGV2ZW50cyA9IFtdOwogIGZvciAoY29uc3QgcGF0aCBvZiBwYXRocykgewogICAgY29uc3QgcmVzdWx0cyA9IHF1ZXJ5U3FsaXRlKHBhdGgsIHF1ZXJ5KTsKICAgIGlmIChyZXN1bHRzIGluc3RhbmNlb2YgQXBwbGljYXRpb25FcnJvcikgewogICAgICByZXR1cm4gbmV3IE1hY29zRXJyb3IoCiAgICAgICAgYFFVQVJBTlRJTkVfRVZFTlRgLAogICAgICAgIGBmYWlsZWQgdG8gcXVlcnkgJHtwYXRofTogJHtyZXN1bHRzfWAKICAgICAgKTsKICAgIH0KICAgIGNvbnN0IGVudHJpZXMgPSBbXTsKICAgIGZvciAoY29uc3QgdmFsdWUgb2YgcmVzdWx0cykgewogICAgICBjb25zdCBlbnRyeSA9IHsKICAgICAgICBpZDogdmFsdWVbIkxTUXVhcmFudGluZUV2ZW50SWRlbnRpZmllciJdLAogICAgICAgIHRpbWVzdGFtcDogY29jb2F0aW1lVG9Vbml4RXBvY2goCiAgICAgICAgICB2YWx1ZVsiTFNRdWFyYW50aW5lVGltZVN0YW1wIl0KICAgICAgICApLAogICAgICAgIGFnZW50X25hbWU6IHZhbHVlWyJMU1F1YXJhbnRpbmVBZ2VudE5hbWUiXSwKICAgICAgICB0eXBlOiBxdWFyYW50aW5lVHlwZSh2YWx1ZVsiTFNRdWFyYW50aW5lVHlwZU51bWJlciJdKSwKICAgICAgICBidW5kbGVfaWQ6IHR5cGVvZiB2YWx1ZVsiTFNRdWFyYW50aW5lQWdlbnRCdW5kbGVJZGVudGlmaWVyIl0gPT09ICJ1bmRlZmluZWQiIHx8IHZhbHVlWyJMU1F1YXJhbnRpbmVBZ2VudEJ1bmRsZUlkZW50aWZpZXIiXSA9PT0gbnVsbCA/ICIiIDogdmFsdWVbIkxTUXVhcmFudGluZUFnZW50QnVuZGxlSWRlbnRpZmllciJdLAogICAgICAgIHVybF9zdHJpbmc6IHR5cGVvZiB2YWx1ZVsiTFNRdWFyYW50aW5lRGF0YVVSTFN0cmluZyJdID09PSAidW5kZWZpbmVkIiB8fCB2YWx1ZVsiTFNRdWFyYW50aW5lRGF0YVVSTFN0cmluZyJdID09PSBudWxsID8gIiIgOiB2YWx1ZVsiTFNRdWFyYW50aW5lRGF0YVVSTFN0cmluZyJdLAogICAgICAgIHNlbmRlcl9hZGRyZXNzOiB0eXBlb2YgdmFsdWVbIkxTUXVhcmFudGluZVNlbmRlckFkZHJlc3MiXSA9PT0gInVuZGVmaW5lZCIgfHwgdmFsdWVbIkxTUXVhcmFudGluZVNlbmRlckFkZHJlc3MiXSA9PT0gbnVsbCA/ICIiIDogdmFsdWVbIkxTUXVhcmFudGluZVNlbmRlckFkZHJlc3MiXSwKICAgICAgICBzZW5kZXJfbmFtZTogdHlwZW9mIHZhbHVlWyJMU1F1YXJhbnRpbmVTZW5kZXJOYW1lIl0gPT09ICJ1bmRlZmluZWQiIHx8IHZhbHVlWyJMU1F1YXJhbnRpbmVTZW5kZXJOYW1lIl0gPT09IG51bGwgPyAiIiA6IHZhbHVlWyJMU1F1YXJhbnRpbmVTZW5kZXJOYW1lIl0sCiAgICAgICAgb3JpZ2luX2FsaWFzOiB0eXBlb2YgdmFsdWVbIkxTUXVhcmFudGluZU9yaWdpbkFsaWFzIl0gPT09ICJ1bmRlZmluZWQiIHx8IHZhbHVlWyJMU1F1YXJhbnRpbmVPcmlnaW5BbGlhcyJdID09PSBudWxsID8gIiIgOiB2YWx1ZVsiTFNRdWFyYW50aW5lT3JpZ2luQWxpYXMiXSwKICAgICAgICBvcmlnaW5fdGl0bGU6IHR5cGVvZiB2YWx1ZVsiTFNRdWFyYW50aW5lT3JpZ2luVGl0bGUiXSA9PT0gInVuZGVmaW5lZCIgfHwgdmFsdWVbIkxTUXVhcmFudGluZU9yaWdpblRpdGxlIl0gPT09IG51bGwgPyAiIiA6IHZhbHVlWyJMU1F1YXJhbnRpbmVPcmlnaW5UaXRsZSJdLAogICAgICAgIG9yaWdpbl91cmw6IHR5cGVvZiB2YWx1ZVsiTFNRdWFyYW50aW5lT3JpZ2luVVJMU3RyaW5nIl0gPT09ICJ1bmRlZmluZWQiIHx8IHZhbHVlWyJMU1F1YXJhbnRpbmVPcmlnaW5VUkxTdHJpbmciXSA9PT0gbnVsbCA/ICIiIDogdmFsdWVbIkxTUXVhcmFudGluZU9yaWdpblVSTFN0cmluZyJdCiAgICAgIH07CiAgICAgIGVudHJpZXMucHVzaChlbnRyeSk7CiAgICB9CiAgICBjb25zdCBldmVudCA9IHsKICAgICAgcGF0aCwKICAgICAgZXZlbnRzOiBlbnRyaWVzCiAgICB9OwogICAgZXZlbnRzLnB1c2goZXZlbnQpOwogIH0KICByZXR1cm4gZXZlbnRzOwp9CmZ1bmN0aW9uIHF1YXJhbnRpbmVUeXBlKGRhdGEpIHsKICBzd2l0Y2ggKGRhdGEpIHsKICAgIGNhc2UgMDoKICAgICAgcmV0dXJuICJXZWJEb3dubG9hZCIgLyogV0VCRE9XTkxPQUQgKi87CiAgICBjYXNlIDE6CiAgICAgIHJldHVybiAiRG93bmxvYWQiIC8qIERPV05MT0FEICovOwogICAgY2FzZSAyOgogICAgICByZXR1cm4gIkVtYWlsQXR0YWNobWVudCIgLyogRU1BSUxBVFRBQ0hNRU5UICovOwogICAgY2FzZSAzOgogICAgICByZXR1cm4gIk1lc3NhZ2VBdHRhY2htZW50IiAvKiBNRVNTQUdFQVRUQUNITUVOVCAqLzsKICAgIGNhc2UgNDoKICAgICAgcmV0dXJuICJDYWxlbmRhckF0dGFjaG1lbnQiIC8qIENBTEVOREFSQVRUQUNITUVOVCAqLzsKICAgIGNhc2UgNToKICAgICAgcmV0dXJuICJBdHRhY2htZW50IiAvKiBBVFRBQ0hNRU5UICovOwogICAgZGVmYXVsdDoKICAgICAgcmV0dXJuICJVbmtub3duIiAvKiBVTktOT1dOICovOwogIH0KfQoKLy8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3N5c3RlbS9lcnJvci50cwp2YXIgU3lzdGVtRXJyb3IgPSBjbGFzcyBleHRlbmRzIEVycm9yQmFzZSB7Cn07CgovLyAuLi8uLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvc3lzdGVtL291dHB1dC50cwpmdW5jdGlvbiBkdW1wRGF0YShkYXRhLCBkYXRhX25hbWUsIG91dHB1dCkgewogIHRyeSB7CiAgICBjb25zdCBvdXRwdXRfc3RyaW5nID0gSlNPTi5zdHJpbmdpZnkob3V0cHV0KTsKICAgIGNvbnN0IHN0YXR1cyA9IERlbm8uY29yZS5vcHMucmF3X2R1bXAoCiAgICAgIGRhdGEsCiAgICAgIGRhdGFfbmFtZSwKICAgICAgb3V0cHV0X3N0cmluZwogICAgKTsKICAgIHJldHVybiBzdGF0dXM7CiAgfSBjYXRjaCAoZXJyKSB7CiAgICByZXR1cm4gbmV3IFN5c3RlbUVycm9yKGBPVVRQVVRgLCBgZmFpbGVkIHRvIG91dHB1dCByYXcgZGF0YTogJHtlcnJ9YCk7CiAgfQp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgZGF0YSA9IHF1YXJhbnRpbmVFdmVudHMoKTsKICBjb25zdCBvdXQgPSB7CiAgICBuYW1lOiAidGVzdCIsCiAgICBkaXJlY3Rvcnk6ICIuL3RtcCIsCiAgICBmb3JtYXQ6ICJqc29uIiAvKiBKU09OICovLAogICAgY29tcHJlc3M6IGZhbHNlLAogICAgZW5kcG9pbnRfaWQ6ICJibGFoIiwKICAgIGNvbGxlY3Rpb25faWQ6IDAsCiAgICBvdXRwdXQ6ICJsb2NhbCIgLyogTE9DQUwgKi8KICB9OwogIGNvbnN0IHN0YXR1cyA9IGR1bXBEYXRhKEpTT04uc3RyaW5naWZ5KGRhdGEpLCAicXVhcmFudGluZSIsIG91dCk7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", true);
        let script = JSScript {
            name: String::from("output_results"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

}
