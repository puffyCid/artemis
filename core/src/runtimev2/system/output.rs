use crate::{
    output::formats::{json::raw_json, jsonl::raw_jsonl},
    runtimev2::{
        helper::{string_arg, value_arg},
        run::output_data,
    },
    structs::toml::Output,
};
use boa_engine::{js_string, Context, JsError, JsResult, JsValue};
use log::error;

pub(crate) fn js_output_results(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let mut data = value_arg(args, &0, context)?;
    let output_name = string_arg(args, &1)?;
    let output_format = value_arg(args, &2, context)?;

    let sucess = true;

    let output_result = serde_json::from_value(output_format);
    let mut output: Output = match output_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed deserialize output format data: {err:?}");
            let issue = format!("Failed deserialize output format data: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let empty_start = 0;
    let status = output_data(&mut data, &output_name, &mut output, &empty_start);
    if status.is_err() {
        error!("[runtime] Failed could not output script data");
        let issue = String::from("Failed could not output script data");
        return Err(JsError::from_opaque(js_string!(issue).into()));
    }

    Ok(JsValue::Boolean(sucess))
}

pub(crate) fn js_raw_dump(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let data = value_arg(args, &0, context)?;
    let output_name = string_arg(args, &1)?;
    let output_format = value_arg(args, &2, context)?;
    let sucess = true;

    let output_result = serde_json::from_value(output_format);
    let mut output: Output = match output_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed deserialize output format data: {err:?}");
            let issue = format!("Failed deserialize output format data: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    if output.format == "jsonl" {
        if raw_jsonl(&data, &output_name, &mut output).is_err() {
            let issue = String::from("Failed could not output raw jsonl data");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    } else if output.format == "json" {
        if raw_json(&data, &output_name, &mut output).is_err() {
            let issue = String::from("Failed could not output raw json data");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    } else {
        return Err(JsError::from_opaque(
            js_string!(format!("bad format: {}", output.format)).into(),
        ));
    }

    Ok(JsValue::Boolean(sucess))
}

#[cfg(test)]
mod tests {
    use crate::{
        runtimev2::run::execute_script,
        structs::{artifacts::runtime::script::JSScript, toml::Output},
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
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy9wcm9jZXNzZXMudHMKZnVuY3Rpb24gZ2V0X3dpbl9wcm9jZXNzZXMobWQ1LCBzaGExLCBzaGEyNTYsIHBlX2luZm8pIHsKICBjb25zdCBoYXNoZXMgPSB7CiAgICBtZDUsCiAgICBzaGExLAogICAgc2hhMjU2CiAgfTsKICBjb25zdCBkYXRhID0ganNfZ2V0X3Byb2Nlc3NlcygKICAgIGhhc2hlcywKICAgIHBlX2luZm8KICApOwogIHJldHVybiBkYXRhOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9zeXN0ZW0vb3V0cHV0LnRzCmZ1bmN0aW9uIG91dHB1dFJlc3VsdHMoZGF0YSwgZGF0YV9uYW1lLCBvdXRwdXQpIHsKICBjb25zdCBzdGF0dXMgPSBqc19vdXRwdXRfcmVzdWx0cygKICAgIGRhdGEsCiAgICBkYXRhX25hbWUsCiAgICBvdXRwdXQKICApOwogIHJldHVybiBzdGF0dXM7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvbW9kLnRzCmZ1bmN0aW9uIGdldFdpblByb2Nlc3NlcyhtZDUsIHNoYTEsIHNoYTI1NiwgcGVfaW5mbykgewogIHJldHVybiBnZXRfd2luX3Byb2Nlc3NlcyhtZDUsIHNoYTEsIHNoYTI1NiwgcGVfaW5mbyk7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBtZDUgPSB0cnVlOwogIGNvbnN0IHNoYTEgPSBmYWxzZTsKICBjb25zdCBzaGEyNTYgPSBmYWxzZTsKICBjb25zdCBwZV9pbmZvID0gZmFsc2U7CiAgY29uc3QgcHJvY19saXN0ID0gZ2V0V2luUHJvY2Vzc2VzKG1kNSwgc2hhMSwgc2hhMjU2LCBwZV9pbmZvKTsKICBmb3IgKGNvbnN0IGVudHJ5IG9mIHByb2NfbGlzdCkgewogICAgaWYgKGVudHJ5Lm5hbWUuaW5jbHVkZXMoImFydGVtaXMiKSkgewogICAgICBjb25zdCBvdXQgPSB7CiAgICAgICAgbmFtZTogImFydGVtaXNfcHJvYyIsCiAgICAgICAgZGlyZWN0b3J5OiAiLi90bXAiLAogICAgICAgIGZvcm1hdDogImpzb24iIC8qIEpTT04gKi8sCiAgICAgICAgY29tcHJlc3M6IGZhbHNlLAogICAgICAgIGVuZHBvaW50X2lkOiAiYW55dGhpbmctaS13YW50IiwKICAgICAgICBjb2xsZWN0aW9uX2lkOiAxLAogICAgICAgIG91dHB1dDogImxvY2FsIiAvKiBMT0NBTCAqLwogICAgICB9OwogICAgICBjb25zdCBzdGF0dXMgPSBvdXRwdXRSZXN1bHRzKGVudHJ5LCAiYXJ0ZW1pc19pbmZvIiwgb3V0KTsKICAgICAgaWYgKCFzdGF0dXMpIHsKICAgICAgICBjb25zb2xlLmxvZygiQ291bGQgbm90IG91dHB1dCB0byBsb2NhbCBkaXJlY3RvcnkiKTsKICAgICAgfQogICAgfQogIH0KfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", true);
        let script = JSScript {
            name: String::from("output_results"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_output_results_jsonl_compress() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy9wcm9jZXNzZXMudHMKZnVuY3Rpb24gZ2V0X3dpbl9wcm9jZXNzZXMobWQ1LCBzaGExLCBzaGEyNTYsIHBlX2luZm8pIHsKICBjb25zdCBoYXNoZXMgPSB7CiAgICBtZDUsCiAgICBzaGExLAogICAgc2hhMjU2CiAgfTsKICBjb25zdCBkYXRhID0ganNfZ2V0X3Byb2Nlc3NlcygKICAgIGhhc2hlcywKICAgIHBlX2luZm8KICApOwogIHJldHVybiBkYXRhOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9zeXN0ZW0vb3V0cHV0LnRzCmZ1bmN0aW9uIG91dHB1dFJlc3VsdHMoZGF0YSwgZGF0YV9uYW1lLCBvdXRwdXQpIHsKICBjb25zdCBzdGF0dXMgPSBqc19vdXRwdXRfcmVzdWx0cygKICAgIGRhdGEsCiAgICBkYXRhX25hbWUsCiAgICBvdXRwdXQKICApOwogIHJldHVybiBzdGF0dXM7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvbW9kLnRzCmZ1bmN0aW9uIGdldFdpblByb2Nlc3NlcyhtZDUsIHNoYTEsIHNoYTI1NiwgcGVfaW5mbykgewogIHJldHVybiBnZXRfd2luX3Byb2Nlc3NlcyhtZDUsIHNoYTEsIHNoYTI1NiwgcGVfaW5mbyk7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBtZDUgPSBmYWxzZTsKICBjb25zdCBzaGExID0gZmFsc2U7CiAgY29uc3Qgc2hhMjU2ID0gZmFsc2U7CiAgY29uc3QgcGVfaW5mbyA9IGZhbHNlOwogIGNvbnN0IHByb2NfbGlzdCA9IGdldFdpblByb2Nlc3NlcyhtZDUsIHNoYTEsIHNoYTI1NiwgcGVfaW5mbyk7CiAgZm9yIChjb25zdCBlbnRyeSBvZiBwcm9jX2xpc3QpIHsKICAgIGlmIChlbnRyeS5uYW1lLmluY2x1ZGVzKCJhcnRlbWlzIikgfHwgZW50cnkubmFtZS5pbmNsdWRlcygiY29yZSIpKSB7CiAgICAgIGNvbnN0IG91dCA9IHsKICAgICAgICBuYW1lOiAiYXJ0ZW1pc19wcm9jIiwKICAgICAgICBkaXJlY3Rvcnk6ICIuL3RtcCIsCiAgICAgICAgZm9ybWF0OiAianNvbmwiIC8qIEpTT04gKi8sCiAgICAgICAgY29tcHJlc3M6IHRydWUsCiAgICAgICAgZW5kcG9pbnRfaWQ6ICJhbnl0aGluZy1pLXdhbnQiLAogICAgICAgIGNvbGxlY3Rpb25faWQ6IDEsCiAgICAgICAgb3V0cHV0OiAibG9jYWwiIC8qIExPQ0FMICovCiAgICAgIH07CiAgICAgIGNvbnN0IHN0YXR1cyA9IG91dHB1dFJlc3VsdHMoZW50cnksICJhcnRlbWlzX2luZm8iLCBvdXQpOwogICAgICBpZiAoIXN0YXR1cykgewogICAgICAgIGNvbnNvbGUubG9nKCJDb3VsZCBub3Qgb3V0cHV0IHRvIGxvY2FsIGRpcmVjdG9yeSIpOwogICAgICB9CiAgICB9CiAgfQp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", true);
        let script = JSScript {
            name: String::from("output_results"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_raw_dump() {
        let test = "Ly8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3V0aWxzL2Vycm9yLnRzCnZhciBFcnJvckJhc2UgPSBjbGFzcyBleHRlbmRzIEVycm9yIHsKICBjb25zdHJ1Y3RvcihuYW1lLCBtZXNzYWdlKSB7CiAgICBzdXBlcigpOwogICAgdGhpcy5uYW1lID0gbmFtZTsKICAgIHRoaXMubWVzc2FnZSA9IG1lc3NhZ2U7CiAgfQp9OwoKLy8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL2ZpbGVzeXN0ZW0vZXJyb3JzLnRzCnZhciBGaWxlRXJyb3IgPSBjbGFzcyBleHRlbmRzIEVycm9yQmFzZSB7Cn07CgovLyAuLi8uLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvZmlsZXN5c3RlbS9maWxlcy50cwpmdW5jdGlvbiBnbG9iKHBhdHRlcm4pIHsKICB0cnkgewogICAgY29uc3QgcmVzdWx0ID0ganNfZ2xvYihwYXR0ZXJuKTsKICAgIHJldHVybiByZXN1bHQ7CiAgfSBjYXRjaCAoZXJyKSB7CiAgICByZXR1cm4gbmV3IEZpbGVFcnJvcigiR0xPQiIsIGBmYWlsZWQgdG8gZ2xvYiBwYXR0ZXJuICR7cGF0dGVybn0iICR7ZXJyfWApOwogIH0KfQoKLy8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL2FwcGxpY2F0aW9ucy9lcnJvcnMudHMKdmFyIEFwcGxpY2F0aW9uRXJyb3IgPSBjbGFzcyBleHRlbmRzIEVycm9yQmFzZSB7Cn07CgovLyAuLi8uLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvYXBwbGljYXRpb25zL3NxbGl0ZS50cwpmdW5jdGlvbiBxdWVyeVNxbGl0ZShwYXRoLCBxdWVyeSkgewogIHRyeSB7CiAgICBjb25zdCBkYXRhID0ganNfcXVlcnlfc3FsaXRlKHBhdGgsIHF1ZXJ5KTsKICAgIHJldHVybiBkYXRhOwogIH0gY2F0Y2ggKGVycikgewogICAgcmV0dXJuIG5ldyBBcHBsaWNhdGlvbkVycm9yKAogICAgICAiU1FMSVRFIiwKICAgICAgYGZhaWxlZCB0byBleGVjdXRlIHF1ZXJ5ICR7ZXJyfWAKICAgICk7CiAgfQp9CgovLyAuLi8uLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvdGltZS9jb252ZXJzaW9uLnRzCmZ1bmN0aW9uIGNvY29hdGltZVRvVW5peEVwb2NoKGNvY29hdGltZSkgewogIGNvbnN0IGRhdGEgPSBqc19jb2NvYXRpbWVfdG9fdW5peGVwb2NoKGNvY29hdGltZSk7CiAgcmV0dXJuIE51bWJlcihkYXRhKTsKfQoKLy8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL21hY29zL2Vycm9ycy50cwp2YXIgTWFjb3NFcnJvciA9IGNsYXNzIGV4dGVuZHMgRXJyb3JCYXNlIHsKfTsKCi8vIC4uLy4uL1Byb2plY3RzL2FydGVtaXMtYXBpL3NyYy9tYWNvcy9zcWxpdGUvcXVhcmFudGluZS50cwpmdW5jdGlvbiBxdWFyYW50aW5lRXZlbnRzKGFsdF9maWxlKSB7CiAgbGV0IHBhdGhzID0gW107CiAgaWYgKGFsdF9maWxlICE9IHZvaWQgMCkgewogICAgcGF0aHMgPSBbYWx0X2ZpbGVdOwogIH0gZWxzZSB7CiAgICBjb25zdCBnbG9iX3BhdGggPSAiL1VzZXJzLyovTGlicmFyeS9QcmVmZXJlbmNlcy9jb20uYXBwbGUuTGF1bmNoU2VydmljZXMuUXVhcmFudGluZUV2ZW50c1YyIjsKICAgIGNvbnN0IHBhdGhzX3Jlc3VsdHMgPSBnbG9iKGdsb2JfcGF0aCk7CiAgICBpZiAocGF0aHNfcmVzdWx0cyBpbnN0YW5jZW9mIEZpbGVFcnJvcikgewogICAgICByZXR1cm4gbmV3IE1hY29zRXJyb3IoCiAgICAgICAgYFFVQVJBTlRJTkVfRVZFTlRgLAogICAgICAgIGBmYWlsZWQgdG8gZ2xvYiBwYXRoOiAke2dsb2JfcGF0aH06ICR7cGF0aHNfcmVzdWx0c31gCiAgICAgICk7CiAgICB9CiAgICBmb3IgKGNvbnN0IGVudHJ5IG9mIHBhdGhzX3Jlc3VsdHMpIHsKICAgICAgcGF0aHMucHVzaChlbnRyeS5mdWxsX3BhdGgpOwogICAgfQogIH0KICBjb25zdCBxdWVyeSA9ICJzZWxlY3QgKiBmcm9tIExTUXVhcmFudGluZUV2ZW50IjsKICBjb25zdCBldmVudHMgPSBbXTsKICBmb3IgKGNvbnN0IHBhdGggb2YgcGF0aHMpIHsKICAgIGNvbnN0IHJlc3VsdHMgPSBxdWVyeVNxbGl0ZShwYXRoLCBxdWVyeSk7CiAgICBpZiAocmVzdWx0cyBpbnN0YW5jZW9mIEFwcGxpY2F0aW9uRXJyb3IpIHsKICAgICAgcmV0dXJuIG5ldyBNYWNvc0Vycm9yKAogICAgICAgIGBRVUFSQU5USU5FX0VWRU5UYCwKICAgICAgICBgZmFpbGVkIHRvIHF1ZXJ5ICR7cGF0aH06ICR7cmVzdWx0c31gCiAgICAgICk7CiAgICB9CiAgICBjb25zdCBlbnRyaWVzID0gW107CiAgICBmb3IgKGNvbnN0IHZhbHVlIG9mIHJlc3VsdHMpIHsKICAgICAgY29uc3QgZW50cnkgPSB7CiAgICAgICAgaWQ6IHZhbHVlWyJMU1F1YXJhbnRpbmVFdmVudElkZW50aWZpZXIiXSwKICAgICAgICB0aW1lc3RhbXA6IGNvY29hdGltZVRvVW5peEVwb2NoKAogICAgICAgICAgdmFsdWVbIkxTUXVhcmFudGluZVRpbWVTdGFtcCJdCiAgICAgICAgKSwKICAgICAgICBhZ2VudF9uYW1lOiB2YWx1ZVsiTFNRdWFyYW50aW5lQWdlbnROYW1lIl0sCiAgICAgICAgdHlwZTogcXVhcmFudGluZVR5cGUodmFsdWVbIkxTUXVhcmFudGluZVR5cGVOdW1iZXIiXSksCiAgICAgICAgYnVuZGxlX2lkOiB0eXBlb2YgdmFsdWVbIkxTUXVhcmFudGluZUFnZW50QnVuZGxlSWRlbnRpZmllciJdID09PSAidW5kZWZpbmVkIiB8fCB2YWx1ZVsiTFNRdWFyYW50aW5lQWdlbnRCdW5kbGVJZGVudGlmaWVyIl0gPT09IG51bGwgPyAiIiA6IHZhbHVlWyJMU1F1YXJhbnRpbmVBZ2VudEJ1bmRsZUlkZW50aWZpZXIiXSwKICAgICAgICB1cmxfc3RyaW5nOiB0eXBlb2YgdmFsdWVbIkxTUXVhcmFudGluZURhdGFVUkxTdHJpbmciXSA9PT0gInVuZGVmaW5lZCIgfHwgdmFsdWVbIkxTUXVhcmFudGluZURhdGFVUkxTdHJpbmciXSA9PT0gbnVsbCA/ICIiIDogdmFsdWVbIkxTUXVhcmFudGluZURhdGFVUkxTdHJpbmciXSwKICAgICAgICBzZW5kZXJfYWRkcmVzczogdHlwZW9mIHZhbHVlWyJMU1F1YXJhbnRpbmVTZW5kZXJBZGRyZXNzIl0gPT09ICJ1bmRlZmluZWQiIHx8IHZhbHVlWyJMU1F1YXJhbnRpbmVTZW5kZXJBZGRyZXNzIl0gPT09IG51bGwgPyAiIiA6IHZhbHVlWyJMU1F1YXJhbnRpbmVTZW5kZXJBZGRyZXNzIl0sCiAgICAgICAgc2VuZGVyX25hbWU6IHR5cGVvZiB2YWx1ZVsiTFNRdWFyYW50aW5lU2VuZGVyTmFtZSJdID09PSAidW5kZWZpbmVkIiB8fCB2YWx1ZVsiTFNRdWFyYW50aW5lU2VuZGVyTmFtZSJdID09PSBudWxsID8gIiIgOiB2YWx1ZVsiTFNRdWFyYW50aW5lU2VuZGVyTmFtZSJdLAogICAgICAgIG9yaWdpbl9hbGlhczogdHlwZW9mIHZhbHVlWyJMU1F1YXJhbnRpbmVPcmlnaW5BbGlhcyJdID09PSAidW5kZWZpbmVkIiB8fCB2YWx1ZVsiTFNRdWFyYW50aW5lT3JpZ2luQWxpYXMiXSA9PT0gbnVsbCA/ICIiIDogdmFsdWVbIkxTUXVhcmFudGluZU9yaWdpbkFsaWFzIl0sCiAgICAgICAgb3JpZ2luX3RpdGxlOiB0eXBlb2YgdmFsdWVbIkxTUXVhcmFudGluZU9yaWdpblRpdGxlIl0gPT09ICJ1bmRlZmluZWQiIHx8IHZhbHVlWyJMU1F1YXJhbnRpbmVPcmlnaW5UaXRsZSJdID09PSBudWxsID8gIiIgOiB2YWx1ZVsiTFNRdWFyYW50aW5lT3JpZ2luVGl0bGUiXSwKICAgICAgICBvcmlnaW5fdXJsOiB0eXBlb2YgdmFsdWVbIkxTUXVhcmFudGluZU9yaWdpblVSTFN0cmluZyJdID09PSAidW5kZWZpbmVkIiB8fCB2YWx1ZVsiTFNRdWFyYW50aW5lT3JpZ2luVVJMU3RyaW5nIl0gPT09IG51bGwgPyAiIiA6IHZhbHVlWyJMU1F1YXJhbnRpbmVPcmlnaW5VUkxTdHJpbmciXQogICAgICB9OwogICAgICBlbnRyaWVzLnB1c2goZW50cnkpOwogICAgfQogICAgY29uc3QgZXZlbnQgPSB7CiAgICAgIHBhdGgsCiAgICAgIGV2ZW50czogZW50cmllcwogICAgfTsKICAgIGV2ZW50cy5wdXNoKGV2ZW50KTsKICB9CiAgcmV0dXJuIGV2ZW50czsKfQpmdW5jdGlvbiBxdWFyYW50aW5lVHlwZShkYXRhKSB7CiAgc3dpdGNoIChkYXRhKSB7CiAgICBjYXNlIDA6CiAgICAgIHJldHVybiAiV2ViRG93bmxvYWQiIC8qIFdFQkRPV05MT0FEICovOwogICAgY2FzZSAxOgogICAgICByZXR1cm4gIkRvd25sb2FkIiAvKiBET1dOTE9BRCAqLzsKICAgIGNhc2UgMjoKICAgICAgcmV0dXJuICJFbWFpbEF0dGFjaG1lbnQiIC8qIEVNQUlMQVRUQUNITUVOVCAqLzsKICAgIGNhc2UgMzoKICAgICAgcmV0dXJuICJNZXNzYWdlQXR0YWNobWVudCIgLyogTUVTU0FHRUFUVEFDSE1FTlQgKi87CiAgICBjYXNlIDQ6CiAgICAgIHJldHVybiAiQ2FsZW5kYXJBdHRhY2htZW50IiAvKiBDQUxFTkRBUkFUVEFDSE1FTlQgKi87CiAgICBjYXNlIDU6CiAgICAgIHJldHVybiAiQXR0YWNobWVudCIgLyogQVRUQUNITUVOVCAqLzsKICAgIGRlZmF1bHQ6CiAgICAgIHJldHVybiAiVW5rbm93biIgLyogVU5LTk9XTiAqLzsKICB9Cn0KCi8vIC4uLy4uL1Byb2plY3RzL2FydGVtaXMtYXBpL3NyYy9zeXN0ZW0vZXJyb3IudHMKdmFyIFN5c3RlbUVycm9yID0gY2xhc3MgZXh0ZW5kcyBFcnJvckJhc2Ugewp9OwoKLy8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3N5c3RlbS9vdXRwdXQudHMKZnVuY3Rpb24gZHVtcERhdGEoZGF0YSwgZGF0YV9uYW1lLCBvdXRwdXQpIHsKICB0cnkgewogICAgY29uc3Qgc3RhdHVzID0ganNfcmF3X2R1bXAoCiAgICAgIGRhdGEsCiAgICAgIGRhdGFfbmFtZSwKICAgICAgb3V0cHV0CiAgICApOwogICAgcmV0dXJuIHN0YXR1czsKICB9IGNhdGNoIChlcnIpIHsKICAgIHJldHVybiBuZXcgU3lzdGVtRXJyb3IoYE9VVFBVVGAsIGBmYWlsZWQgdG8gb3V0cHV0IHJhdyBkYXRhOiAke2Vycn1gKTsKICB9Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBkYXRhID0gcXVhcmFudGluZUV2ZW50cygpOwogIGNvbnN0IG91dCA9IHsKICAgIG5hbWU6ICJ0ZXN0IiwKICAgIGRpcmVjdG9yeTogIi4vdG1wIiwKICAgIGZvcm1hdDogImpzb24iIC8qIEpTT04gKi8sCiAgICBjb21wcmVzczogZmFsc2UsCiAgICBlbmRwb2ludF9pZDogImJsYWgiLAogICAgY29sbGVjdGlvbl9pZDogMCwKICAgIG91dHB1dDogImxvY2FsIiAvKiBMT0NBTCAqLwogIH07CiAgY29uc3Qgc3RhdHVzID0gZHVtcERhdGEoZGF0YSwgInF1YXJhbnRpbmUiLCBvdXQpOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", true);
        let script = JSScript {
            name: String::from("output_results"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
