use crate::artifacts::os::linux::logons::parser::grab_logon_file;
use deno_core::{error::AnyError, op2};

#[op2]
#[string]
/// Expose parsing logon file  to `Deno`
pub(crate) fn get_logon(#[string] path: String) -> Result<String, AnyError> {
    let mut logons = Vec::new();
    grab_logon_file(&path, &mut logons);

    let results = serde_json::to_string(&logons)?;
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
    fn test_get_logon() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbGludXgvbG9nb24udHMKZnVuY3Rpb24gZ2V0TG9nb24ocGF0aCkgewogIGlmIChwYXRoLmVuZHNXaXRoKCJidG1wIikgJiYgIXBhdGguZW5kc1dpdGgoInd0bXAiKSAmJiAhcGF0aC5lbmRzV2l0aCgidXRtcCIpKSB7CiAgICBjb25zb2xlLmVycm9yKGBQcm92aWRlZCBub24tbG9nb24gZmlsZSAke3BhdGh9YCk7CiAgICByZXR1cm4gW107CiAgfQogIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9sb2dvbihwYXRoKTsKICBjb25zdCBqb3VybmFsID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gam91cm5hbDsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHd0bXAgPSAiL3Zhci9sb2cvd3RtcCI7CiAgY29uc3QgcmVzdWx0cyA9IGdldExvZ29uKHd0bXApOwogIHJldHVybiByZXN1bHRzOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);

        let script = JSScript {
            name: String::from("logon"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
