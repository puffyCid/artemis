use crate::artifacts::os::unix::shell_history::{
    bash::get_user_bash_history, python::get_user_python_history, zsh::get_user_zsh_history,
};
use deno_core::{error::AnyError, op2};

#[op2]
#[string]
/// Get `Bash history` for all users
pub(crate) fn get_bash_history() -> Result<String, AnyError> {
    let history = get_user_bash_history()?;
    let results = serde_json::to_string(&history)?;
    Ok(results)
}

#[op2]
#[string]
/// Get `Zsh history` for all users
pub(crate) fn get_zsh_history() -> Result<String, AnyError> {
    let history = get_user_zsh_history()?;
    let results = serde_json::to_string(&history)?;
    Ok(results)
}

#[op2]
#[string]
/// Get `Python history` for all users
pub(crate) fn get_python_history() -> Result<String, AnyError> {
    let history = get_user_python_history()?;
    let results = serde_json::to_string(&history)?;
    Ok(results)
}

#[cfg(test)]
#[cfg(target_family = "unix")]
mod tests {
    use crate::{
        runtime::deno::execute_script, structs::artifacts::runtime::script::JSScript,
        structs::toml::Output,
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
    fn test_get_bash_history() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3VuaXgvc2hlbGxfaGlzdG9yeS50cwpmdW5jdGlvbiBnZXRfYmFzaF9oaXN0b3J5KCkgewogIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9iYXNoX2hpc3RvcnkoKTsKICBjb25zdCBoaXN0b3J5ID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gaGlzdG9yeTsKfQoKLy8gLi4vLi4vYXJ0ZW1pcy1hcGkvbW9kLnRzCmZ1bmN0aW9uIGdldEJhc2hIaXN0b3J5KCkgewogIHJldHVybiBnZXRfYmFzaF9oaXN0b3J5KCk7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBkYXRhID0gZ2V0QmFzaEhpc3RvcnkoKTsKICByZXR1cm4gZGF0YTsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("bash_history"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_python_history() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3VuaXgvc2hlbGxfaGlzdG9yeS50cwpmdW5jdGlvbiBnZXRfcHl0aG9uX2hpc3RvcnkoKSB7CiAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X3B5dGhvbl9oaXN0b3J5KCk7CiAgY29uc3QgaGlzdG9yeSA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGhpc3Rvcnk7Cn0KCi8vIC4uLy4uL2FydGVtaXMtYXBpL21vZC50cwpmdW5jdGlvbiBnZXRQeXRob25IaXN0b3J5KCkgewogIHJldHVybiBnZXRfcHl0aG9uX2hpc3RvcnkoKTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IGRhdGEgPSBnZXRQeXRob25IaXN0b3J5KCk7CiAgcmV0dXJuIGRhdGE7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("python_history"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_zsh_history() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3VuaXgvc2hlbGxfaGlzdG9yeS50cwpmdW5jdGlvbiBnZXRfenNoX2hpc3RvcnkoKSB7CiAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X3pzaF9oaXN0b3J5KCk7CiAgY29uc3QgaGlzdG9yeSA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGhpc3Rvcnk7Cn0KCi8vIC4uLy4uL2FydGVtaXMtYXBpL21vZC50cwpmdW5jdGlvbiBnZXRac2hIaXN0b3J5KCkgewogIHJldHVybiBnZXRfenNoX2hpc3RvcnkoKTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IGRhdGEgPSBnZXRac2hIaXN0b3J5KCk7CiAgcmV0dXJuIGRhdGE7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("zsh_history"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
