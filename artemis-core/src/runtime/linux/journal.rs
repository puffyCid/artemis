use crate::artifacts::os::linux::journals::parser::grab_journal_file;
use deno_core::{error::AnyError, op};

#[op]
/// Expose parsing journal file  to `Deno`
fn get_journal(path: String) -> Result<String, AnyError> {
    let elf_results = grab_journal_file(&path);
    let elf_data = match elf_results {
        Ok(results) => results,
        Err(_err) => {
            // Parsing Journal files could fail
            // Instead of cancelling the whole script, return empty result
            return Ok(String::new());
        }
    };
    let results = serde_json::to_string(&elf_data)?;
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
            filter_name: None,
            filter_script: None,
            logging: None,
        }
    }

    #[test]
    fn test_get_journal() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbGludXgvam91cm5hbC50cwpmdW5jdGlvbiBnZXRKb3VybmFsKHBhdGgpIHsKICBjb25zdCBkYXRhID0gRGVub1tEZW5vLmludGVybmFsXS5jb3JlLm9wcy5nZXRfam91cm5hbChwYXRoKTsKICBpZiAoZGF0YSA9PT0gIiIpIHsKICAgIHJldHVybiBudWxsOwogIH0KICBjb25zdCBqb3VybmFsID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gam91cm5hbDsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvc3lzdGVtL291dHB1dC50cwpmdW5jdGlvbiBvdXRwdXRSZXN1bHRzKGRhdGEsIGRhdGFfbmFtZSwgb3V0cHV0KSB7CiAgY29uc3Qgb3V0cHV0X3N0cmluZyA9IEpTT04uc3RyaW5naWZ5KG91dHB1dCk7CiAgY29uc3Qgc3RhdHVzID0gRGVub1tEZW5vLmludGVybmFsXS5jb3JlLm9wcy5vdXRwdXRfcmVzdWx0cygKICAgIGRhdGEsCiAgICBkYXRhX25hbWUsCiAgICBvdXRwdXRfc3RyaW5nCiAgKTsKICByZXR1cm4gc3RhdHVzOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3Qgam91cm5hbHMgPSAiL3Zhci9sb2cvam91cm5hbCI7CiAgY29uc3Qgb3V0ID0gewogICAgbmFtZTogImRlbm9fam91cm5hbHMiLAogICAgZGlyZWN0b3J5OiAiLi90bXAiLAogICAgZm9ybWF0OiAianNvbiIgLyogSlNPTiAqLywKICAgIGNvbXByZXNzOiBmYWxzZSwKICAgIGVuZHBvaW50X2lkOiAiYW55dGhpbmctaS13YW50IiwKICAgIGNvbGxlY3Rpb25faWQ6IDEsCiAgICBvdXRwdXQ6ICJsb2NhbCIgLyogTE9DQUwgKi8KICB9OwogIGZvciAoY29uc3QgZW50cnkgb2YgRGVuby5yZWFkRGlyU3luYyhqb3VybmFscykpIHsKICAgIGlmICghZW50cnkuaXNEaXJlY3RvcnkpIHsKICAgICAgY29udGludWU7CiAgICB9CiAgICBjb25zdCBmdWxsX3BhdGggPSBgJHtqb3VybmFsc30vJHtlbnRyeS5uYW1lfWA7CiAgICBmb3IgKGNvbnN0IGZpbGVzIG9mIERlbm8ucmVhZERpclN5bmMoZnVsbF9wYXRoKSkgewogICAgICBpZiAoIWZpbGVzLm5hbWUuZW5kc1dpdGgoImpvdXJuYWwiKSkgewogICAgICAgIGNvbnRpbnVlOwogICAgICB9CiAgICAgIGNvbnN0IGpvdXJuYWxfZmlsZSA9IGAke2Z1bGxfcGF0aH0vJHtmaWxlcy5uYW1lfWA7CiAgICAgIGNvbnN0IGRhdGEgPSBnZXRKb3VybmFsKGpvdXJuYWxfZmlsZSk7CiAgICAgIGNvbnN0IHN0YXR1cyA9IG91dHB1dFJlc3VsdHMoSlNPTi5zdHJpbmdpZnkoZGF0YSksICJqb3VybmFsIiwgb3V0KTsKICAgICAgaWYgKCFzdGF0dXMpIHsKICAgICAgICBjb25zb2xlLmxvZygiQ291bGQgbm90IG91dHB1dCB0byBsb2NhbCBkaXJlY3RvcnkiKTsKICAgICAgfQogICAgfQogIH0KfQptYWluKCk7Cgo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);

        let script = JSScript {
            name: String::from("journal"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
