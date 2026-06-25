use crate::{
    output::manager::OutputManager,
    runtime::{
        helper::{string_arg, value_arg},
        run::output_data,
    },
    structs::{artifacts::runtime::script::JSScript, toml::OutputConfig},
};
use boa_engine::{Context, JsError, JsResult, JsValue, js_string};
use tracing::error;

pub(crate) fn js_output(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let data = value_arg(args, 0, context)?;
    let output_name = string_arg(args, 1)?;
    let output_format = value_arg(args, 2, context)?;

    let output_result = serde_json::from_value(output_format);
    let config: OutputConfig = match output_result {
        Ok(results) => results,
        Err(err) => {
            error!("Failed deserialize output config format: {err:?}");
            let issue = format!("Failed deserialize output config format: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let mut manager = match OutputManager::new(config) {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to create OutputManager: {err:?}");
            let issue = format!("Failed to create OutputManager: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let script_dump = JSScript {
        name: output_name,
        script: String::new(),
    };

    let status = output_data(data, &script_dump, &mut manager);
    if status.is_err() {
        error!("Failed could not output script data");
        let issue = String::from("Failed could not output script data");
        return Err(JsError::from_opaque(js_string!(issue).into()));
    }
    if let Err(err) = manager.finalize() {
        error!("Could not complete record from data: {err:?}");
        let issue = format!("Could not complete record from data: {err:?}");
        return Err(JsError::from_opaque(js_string!(issue).into()));
    }
    let sucess = true;
    Ok(JsValue::new(sucess))
}

#[cfg(test)]
mod tests {
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use crate::{
        output::manager::OutputManager, runtime::run::execute_script,
        structs::artifacts::runtime::script::JSScript,
    };
    use std::{
        fs::{read_dir, read_to_string},
        path::PathBuf,
    };

    fn output_options(name: &str, directory: &str, compress: bool) -> OutputManager {
        let config = OutputConfig {
            name: name.to_string(),
            directory: PathBuf::from(directory),
            format: OutputFormat::Jsonl,
            compress,
            endpoint_id: String::from("abcd"),
            destination: OutputDestination::Local,
            ..Default::default()
        };
        OutputManager::new(config).unwrap()
    }

    #[test]
    fn test_output_results() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy9wcm9jZXNzZXMudHMKZnVuY3Rpb24gZ2V0X3dpbl9wcm9jZXNzZXMobWQ1LCBzaGExLCBzaGEyNTYsIHBlX2luZm8pIHsKICBjb25zdCBoYXNoZXMgPSB7CiAgICBtZDUsCiAgICBzaGExLAogICAgc2hhMjU2CiAgfTsKICBjb25zdCBkYXRhID0ganNfZ2V0X3Byb2Nlc3NlcygKICAgIGhhc2hlcywKICAgIHBlX2luZm8KICApOwogIHJldHVybiBkYXRhOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9zeXN0ZW0vb3V0cHV0LnRzCmZ1bmN0aW9uIG91dHB1dFJlc3VsdHMoZGF0YSwgZGF0YV9uYW1lLCBvdXRwdXQpIHsKICBjb25zdCBzdGF0dXMgPSBqc19vdXRwdXQoCiAgICBkYXRhLAogICAgZGF0YV9uYW1lLAogICAgb3V0cHV0CiAgKTsKICByZXR1cm4gc3RhdHVzOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL21vZC50cwpmdW5jdGlvbiBnZXRXaW5Qcm9jZXNzZXMobWQ1LCBzaGExLCBzaGEyNTYsIHBlX2luZm8pIHsKICByZXR1cm4gZ2V0X3dpbl9wcm9jZXNzZXMobWQ1LCBzaGExLCBzaGEyNTYsIHBlX2luZm8pOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgbWQ1ID0gZmFsc2U7CiAgY29uc3Qgc2hhMSA9IGZhbHNlOwogIGNvbnN0IHNoYTI1NiA9IGZhbHNlOwogIGNvbnN0IHBlX2luZm8gPSBmYWxzZTsKICBjb25zdCBwcm9jX2xpc3QgPSBnZXRXaW5Qcm9jZXNzZXMobWQ1LCBzaGExLCBzaGEyNTYsIHBlX2luZm8pOwogIGZvciAoY29uc3QgZW50cnkgb2YgcHJvY19saXN0KSB7CiAgICBpZiAoZW50cnkubmFtZS5pbmNsdWRlcygiZm9yZW5zaWNzIikpIHsKICAgICAgY29uc3Qgb3V0ID0gewogICAgICAgIG5hbWU6ICJhcnRlbWlzX3Byb2NfdmFsaWRhdGUiLAogICAgICAgIGRpcmVjdG9yeTogIi4vdG1wIiwKICAgICAgICBmb3JtYXQ6ICJqc29uIiAvKiBKU09OICovLAogICAgICAgIGNvbXByZXNzOiBmYWxzZSwKICAgICAgICBlbmRwb2ludF9pZDogImFueXRoaW5nLWktd2FudCIsCiAgICAgICAgY29sbGVjdGlvbl9pZDogMSwKICAgICAgICBkZXN0aW5hdGlvbjogImxvY2FsIiAvKiBMT0NBTCAqLwogICAgICB9OwogICAgICBjb25zdCBzdGF0dXMgPSBvdXRwdXRSZXN1bHRzKGVudHJ5LCAiYXJ0ZW1pc19pbmZvIiwgb3V0KTsKICAgICAgaWYgKCFzdGF0dXMpIHsKICAgICAgICBjb25zb2xlLmxvZygiQ291bGQgbm90IG91dHB1dCB0byBsb2NhbCBkaXJlY3RvcnkiKTsKICAgICAgfQogICAgfQogIH0KfQptYWluKCk7Cg==";
        let mut output = output_options("artemis_proc_validate", "./tmp", false);
        let script = JSScript {
            name: String::from("output_results"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();

        let output_dir = PathBuf::from("./tmp").join(String::from("artemis_proc_validate"));
        assert!(output_dir.exists());
        let mut json_files = Vec::new();
        for entry in read_dir(&output_dir).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy();
            if name.starts_with("artemis_info") && name.ends_with(".json") {
                json_files.push(path);
            }
        }
        assert!(json_files.len() >= 1);
        let text = read_to_string(&json_files[0]).unwrap();
        assert!(text.contains("forensics-"));
    }

    #[test]
    fn test_output_results_jsonl_compress() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy9wcm9jZXNzZXMudHMKZnVuY3Rpb24gZ2V0X3dpbl9wcm9jZXNzZXMobWQ1LCBzaGExLCBzaGEyNTYsIHBlX2luZm8pIHsKICBjb25zdCBoYXNoZXMgPSB7CiAgICBtZDUsCiAgICBzaGExLAogICAgc2hhMjU2CiAgfTsKICBjb25zdCBkYXRhID0ganNfZ2V0X3Byb2Nlc3NlcygKICAgIGhhc2hlcywKICAgIHBlX2luZm8KICApOwogIHJldHVybiBkYXRhOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9zeXN0ZW0vb3V0cHV0LnRzCmZ1bmN0aW9uIG91dHB1dFJlc3VsdHMoZGF0YSwgZGF0YV9uYW1lLCBvdXRwdXQpIHsKICBjb25zdCBzdGF0dXMgPSBqc19vdXRwdXQoCiAgICBkYXRhLAogICAgZGF0YV9uYW1lLAogICAgb3V0cHV0CiAgKTsKICByZXR1cm4gc3RhdHVzOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL21vZC50cwpmdW5jdGlvbiBnZXRXaW5Qcm9jZXNzZXMobWQ1LCBzaGExLCBzaGEyNTYsIHBlX2luZm8pIHsKICByZXR1cm4gZ2V0X3dpbl9wcm9jZXNzZXMobWQ1LCBzaGExLCBzaGEyNTYsIHBlX2luZm8pOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgbWQ1ID0gZmFsc2U7CiAgY29uc3Qgc2hhMSA9IGZhbHNlOwogIGNvbnN0IHNoYTI1NiA9IGZhbHNlOwogIGNvbnN0IHBlX2luZm8gPSBmYWxzZTsKICBjb25zdCBwcm9jX2xpc3QgPSBnZXRXaW5Qcm9jZXNzZXMobWQ1LCBzaGExLCBzaGEyNTYsIHBlX2luZm8pOwogIGZvciAoY29uc3QgZW50cnkgb2YgcHJvY19saXN0KSB7CiAgICBpZiAoZW50cnkubmFtZS5pbmNsdWRlcygiYXJ0ZW1pcyIpIHx8IGVudHJ5Lm5hbWUuaW5jbHVkZXMoImZvcmVuc2ljcyIpKSB7CiAgICAgIGNvbnN0IG91dCA9IHsKICAgICAgICBuYW1lOiAicnVudGltZV90ZXN0IiwKICAgICAgICBkaXJlY3Rvcnk6ICIuL3RtcCIsCiAgICAgICAgZm9ybWF0OiAianNvbmwiIC8qIEpTT04gKi8sCiAgICAgICAgY29tcHJlc3M6IHRydWUsCiAgICAgICAgZW5kcG9pbnRfaWQ6ICJhbnl0aGluZy1pLXdhbnQiLAogICAgICAgIGNvbGxlY3Rpb25faWQ6IDEsCiAgICAgICAgZGVzdGluYXRpb246ICJsb2NhbCIgLyogTE9DQUwgKi8KICAgICAgfTsKICAgICAgY29uc3Qgc3RhdHVzID0gb3V0cHV0UmVzdWx0cyhlbnRyeSwgImFydGVtaXNfaW5mbyIsIG91dCk7CiAgICAgIGlmICghc3RhdHVzKSB7CiAgICAgICAgY29uc29sZS5sb2coIkNvdWxkIG5vdCBvdXRwdXQgdG8gbG9jYWwgZGlyZWN0b3J5Iik7CiAgICAgIH0KICAgIH0KICB9Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "./tmp", true);
        let script = JSScript {
            name: String::from("output_results"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
