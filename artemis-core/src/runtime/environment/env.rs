use crate::utils::environment::{get_env, get_env_value};
use deno_core::op;
use std::collections::HashMap;

#[op]
/// Get all Environmental variables
fn js_env() -> HashMap<String, String> {
    get_env()
}

#[op]
/// Get a specific Environmental variable
fn js_env_value(var: String) -> String {
    get_env_value(&var)
}

#[cfg(test)]
mod tests {
    use crate::runtime::deno::execute_script;
    use crate::structs::artifacts::runtime::script::JSScript;
    use crate::structs::toml::Output;

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
    fn test_js_env() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZW52aXJvbm1lbnQvZW52LnRzCmZ1bmN0aW9uIGxpc3RFbnYoKSB7CiAgY29uc3QgZGF0YSA9IGVudi5lbnZpcm9ubWVudCgpOwogIHJldHVybiBkYXRhOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgZW52cyA9IGxpc3RFbnYoKTsKICByZXR1cm4gZW52czsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("envs_list"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn test_js_env_value() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZW52aXJvbm1lbnQvZW52LnRzCmZ1bmN0aW9uIGdldEVudlZhbHVlKGtleSkgewogIGNvbnN0IGRhdGEgPSBlbnYuZW52aXJvbm1lbnRWYWx1ZShrZXkpOwogIHJldHVybiBkYXRhOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgZW52cyA9IGdldEVudlZhbHVlKCJQV0QiKTsKICByZXR1cm4gZW52czsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("env_pwd"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_js_env_value() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZW52aXJvbm1lbnQvZW52LnRzCmZ1bmN0aW9uIGdldEVudlZhbHVlKGtleSkgewogIGNvbnN0IGRhdGEgPSBlbnYuZW52aXJvbm1lbnRWYWx1ZShrZXkpOwogIHJldHVybiBkYXRhOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgZW52cyA9IGdldEVudlZhbHVlKCJTeXN0ZW1Ecml2ZSIpOwogIHJldHVybiBlbnZzOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("env_pwd"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
