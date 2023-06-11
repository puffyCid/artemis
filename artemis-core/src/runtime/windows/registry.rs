use crate::{
    artifacts::os::windows::registry::helper::get_registry_keys, runtime::error::RuntimeError,
    utils::regex_options::create_regex,
};
use deno_core::{error::AnyError, op};
use log::error;

#[op]
/// Expose parsing Registry file to `Deno`
fn get_registry(path: String) -> Result<String, AnyError> {
    let all = create_regex("").unwrap(); // Valid regex
    let start_root = "";
    let reg_results = get_registry_keys(start_root, &all, &path);
    let reg = match reg_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to parse registry file: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };

    let results = serde_json::to_string_pretty(&reg)?;
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
            port: Some(0),
            api_key: Some(String::new()),
            username: Some(String::new()),
            password: Some(String::new()),
            generic_keys: Some(Vec::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: None,
            filter_script: None,
        }
    }

    #[test]
    fn test_get_registry() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfcmVnaXN0cnkocGF0aCkgewogICAgY29uc3QgZGF0YSA9IERlbm9bRGVuby5pbnRlcm5hbF0uY29yZS5vcHMuZ2V0X3JlZ2lzdHJ5KHBhdGgpOwogICAgY29uc3QgcmVnX2FycmF5ID0gSlNPTi5wYXJzZShkYXRhKTsKICAgIHJldHVybiByZWdfYXJyYXk7Cn0KZnVuY3Rpb24gZ2V0UmVnaXN0cnkocGF0aCkgewogICAgcmV0dXJuIGdldF9yZWdpc3RyeShwYXRoKTsKfQpmdW5jdGlvbiBncmFiX2luZm8ocmVnKSB7CiAgICBjb25zdCBwcm9ncmFtcyA9IFtdOwogICAgZm9yIChjb25zdCBlbnRyaWVzIG9mIHJlZyl7CiAgICAgICAgaWYgKGVudHJpZXMudmFsdWVzLmxlbmd0aCA8IDMpIHsKICAgICAgICAgICAgY29udGludWU7CiAgICAgICAgfQogICAgICAgIGNvbnN0IHByb2dyYW0gPSB7CiAgICAgICAgICAgIG5hbWU6ICIiLAogICAgICAgICAgICB2ZXJzaW9uOiAiIiwKICAgICAgICAgICAgaW5zdGFsbF9sb2NhdGlvbjogIiIsCiAgICAgICAgICAgIGluc3RhbGxfc291cmNlOiAiIiwKICAgICAgICAgICAgbGFuZ3VhZ2U6ICIiLAogICAgICAgICAgICBwdWJsaXNoZXI6ICIiLAogICAgICAgICAgICBpbnN0YWxsX3N0cmluZzogIiIsCiAgICAgICAgICAgIGluc3RhbGxfZGF0ZTogIiIsCiAgICAgICAgICAgIHVuaW5zdGFsbF9zdHJpbmc6ICIiLAogICAgICAgICAgICB1cmxfaW5mbzogIiIsCiAgICAgICAgICAgIHJlZ19wYXRoOiBlbnRyaWVzLnBhdGgKICAgICAgICB9OwogICAgICAgIGZvciAoY29uc3QgdmFsdWUgb2YgZW50cmllcy52YWx1ZXMpewogICAgICAgICAgICBzd2l0Y2godmFsdWUudmFsdWUpewogICAgICAgICAgICAgICAgY2FzZSAiRGlzcGxheU5hbWUiOgogICAgICAgICAgICAgICAgICAgIHByb2dyYW0ubmFtZSA9IHZhbHVlLmRhdGE7CiAgICAgICAgICAgICAgICAgICAgYnJlYWs7CiAgICAgICAgICAgICAgICBjYXNlICJEaXNwbGF5VmVyc2lvbiI6CiAgICAgICAgICAgICAgICAgICAgcHJvZ3JhbS52ZXJzaW9uID0gdmFsdWUuZGF0YTsKICAgICAgICAgICAgICAgICAgICBicmVhazsKICAgICAgICAgICAgICAgIGNhc2UgIkluc3RhbGxEYXRlIjoKICAgICAgICAgICAgICAgICAgICBwcm9ncmFtLmluc3RhbGxfZGF0ZSA9IHZhbHVlLmRhdGE7CiAgICAgICAgICAgICAgICAgICAgYnJlYWs7CiAgICAgICAgICAgICAgICBjYXNlICJJbnN0YWxsTG9jYXRpb24iOgogICAgICAgICAgICAgICAgICAgIHByb2dyYW0uaW5zdGFsbF9sb2NhdGlvbiA9IHZhbHVlLmRhdGE7CiAgICAgICAgICAgICAgICAgICAgYnJlYWs7CiAgICAgICAgICAgICAgICBjYXNlICJJbnN0YWxsU291cmNlIjoKICAgICAgICAgICAgICAgICAgICBwcm9ncmFtLmluc3RhbGxfc291cmNlID0gdmFsdWUuZGF0YTsKICAgICAgICAgICAgICAgICAgICBicmVhazsKICAgICAgICAgICAgICAgIGNhc2UgIkxhbmd1YWdlIjoKICAgICAgICAgICAgICAgICAgICBwcm9ncmFtLmxhbmd1YWdlID0gdmFsdWUuZGF0YTsKICAgICAgICAgICAgICAgICAgICBicmVhazsKICAgICAgICAgICAgICAgIGNhc2UgIlB1Ymxpc2hlciI6CiAgICAgICAgICAgICAgICAgICAgcHJvZ3JhbS5wdWJsaXNoZXIgPSB2YWx1ZS5kYXRhOwogICAgICAgICAgICAgICAgICAgIGJyZWFrOwogICAgICAgICAgICAgICAgY2FzZSAiVW5pbnN0YWxsU3RyaW5nIjoKICAgICAgICAgICAgICAgICAgICBwcm9ncmFtLnVuaW5zdGFsbF9zdHJpbmcgPSB2YWx1ZS5kYXRhOwogICAgICAgICAgICAgICAgICAgIGJyZWFrOwogICAgICAgICAgICAgICAgY2FzZSAiVVJMSW5mb0Fib3V0IjoKICAgICAgICAgICAgICAgICAgICBwcm9ncmFtLnVybF9pbmZvID0gdmFsdWUuZGF0YTsKICAgICAgICAgICAgICAgICAgICBicmVhazsKICAgICAgICAgICAgICAgIGRlZmF1bHQ6CiAgICAgICAgICAgICAgICAgICAgY29udGludWU7CiAgICAgICAgICAgIH0KICAgICAgICB9CiAgICAgICAgcHJvZ3JhbXMucHVzaChwcm9ncmFtKTsKICAgIH0KICAgIHJldHVybiBwcm9ncmFtczsKfQpmdW5jdGlvbiBtYWluKCkgewogICAgY29uc3QgcGF0aCA9ICJDOlxcV2luZG93c1xcU3lzdGVtMzJcXGNvbmZpZ1xcU09GVFdBUkUiOwogICAgY29uc3QgcmVnID0gZ2V0UmVnaXN0cnkocGF0aCk7CiAgICBjb25zdCBwcm9ncmFtcyA9IFtdOwogICAgZm9yIChjb25zdCBlbnRyaWVzIG9mIHJlZyl7CiAgICAgICAgaWYgKCFlbnRyaWVzLnBhdGguaW5jbHVkZXMoIk1pY3Jvc29mdFxcV2luZG93c1xcQ3VycmVudFZlcnNpb25cXFVuaW5zdGFsbCIpKSB7CiAgICAgICAgICAgIGNvbnRpbnVlOwogICAgICAgIH0KICAgICAgICBwcm9ncmFtcy5wdXNoKGVudHJpZXMpOwogICAgfQogICAgcmV0dXJuIGdyYWJfaW5mbyhwcm9ncmFtcyk7Cn0KbWFpbigpOwoK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("programs"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
