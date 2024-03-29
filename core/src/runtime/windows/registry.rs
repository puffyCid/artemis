use crate::{
    artifacts::os::windows::registry::helper::{get_registry_keys, lookup_sk_info},
    utils::regex_options::create_regex,
};
use deno_core::{error::AnyError, op2};

#[op2]
#[string]
/// Expose parsing Registry file to `Deno`
pub(crate) fn get_registry(#[string] path: String) -> Result<String, AnyError> {
    let all = create_regex("").unwrap(); // Valid regex
    let start_root = "";
    let reg = get_registry_keys(start_root, &all, &path)?;

    let results = serde_json::to_string(&reg)?;
    Ok(results)
}

#[op2]
#[string]
/// Expose parsing the Security Key to `Deno`
pub(crate) fn get_sk_info(#[string] path: String, offset: i32) -> Result<String, AnyError> {
    let sk = lookup_sk_info(&path, offset)?;

    let results = serde_json::to_string(&sk)?;
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
    fn test_get_registry() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfcmVnaXN0cnkocGF0aCkgewogICAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X3JlZ2lzdHJ5KHBhdGgpOwogICAgY29uc3QgcmVnX2FycmF5ID0gSlNPTi5wYXJzZShkYXRhKTsKICAgIHJldHVybiByZWdfYXJyYXk7Cn0KZnVuY3Rpb24gZ2V0UmVnaXN0cnkocGF0aCkgewogICAgcmV0dXJuIGdldF9yZWdpc3RyeShwYXRoKTsKfQpmdW5jdGlvbiBncmFiX2luZm8ocmVnKSB7CiAgICBjb25zdCBwcm9ncmFtcyA9IFtdOwogICAgZm9yIChjb25zdCBlbnRyaWVzIG9mIHJlZyl7CiAgICAgICAgaWYgKGVudHJpZXMudmFsdWVzLmxlbmd0aCA8IDMpIHsKICAgICAgICAgICAgY29udGludWU7CiAgICAgICAgfQogICAgICAgIGNvbnN0IHByb2dyYW0gPSB7CiAgICAgICAgICAgIG5hbWU6ICIiLAogICAgICAgICAgICB2ZXJzaW9uOiAiIiwKICAgICAgICAgICAgaW5zdGFsbF9sb2NhdGlvbjogIiIsCiAgICAgICAgICAgIGluc3RhbGxfc291cmNlOiAiIiwKICAgICAgICAgICAgbGFuZ3VhZ2U6ICIiLAogICAgICAgICAgICBwdWJsaXNoZXI6ICIiLAogICAgICAgICAgICBpbnN0YWxsX3N0cmluZzogIiIsCiAgICAgICAgICAgIGluc3RhbGxfZGF0ZTogIiIsCiAgICAgICAgICAgIHVuaW5zdGFsbF9zdHJpbmc6ICIiLAogICAgICAgICAgICB1cmxfaW5mbzogIiIsCiAgICAgICAgICAgIHJlZ19wYXRoOiBlbnRyaWVzLnBhdGgKICAgICAgICB9OwogICAgICAgIGZvciAoY29uc3QgdmFsdWUgb2YgZW50cmllcy52YWx1ZXMpewogICAgICAgICAgICBzd2l0Y2godmFsdWUudmFsdWUpewogICAgICAgICAgICAgICAgY2FzZSAiRGlzcGxheU5hbWUiOgogICAgICAgICAgICAgICAgICAgIHByb2dyYW0ubmFtZSA9IHZhbHVlLmRhdGE7CiAgICAgICAgICAgICAgICAgICAgYnJlYWs7CiAgICAgICAgICAgICAgICBjYXNlICJEaXNwbGF5VmVyc2lvbiI6CiAgICAgICAgICAgICAgICAgICAgcHJvZ3JhbS52ZXJzaW9uID0gdmFsdWUuZGF0YTsKICAgICAgICAgICAgICAgICAgICBicmVhazsKICAgICAgICAgICAgICAgIGNhc2UgIkluc3RhbGxEYXRlIjoKICAgICAgICAgICAgICAgICAgICBwcm9ncmFtLmluc3RhbGxfZGF0ZSA9IHZhbHVlLmRhdGE7CiAgICAgICAgICAgICAgICAgICAgYnJlYWs7CiAgICAgICAgICAgICAgICBjYXNlICJJbnN0YWxsTG9jYXRpb24iOgogICAgICAgICAgICAgICAgICAgIHByb2dyYW0uaW5zdGFsbF9sb2NhdGlvbiA9IHZhbHVlLmRhdGE7CiAgICAgICAgICAgICAgICAgICAgYnJlYWs7CiAgICAgICAgICAgICAgICBjYXNlICJJbnN0YWxsU291cmNlIjoKICAgICAgICAgICAgICAgICAgICBwcm9ncmFtLmluc3RhbGxfc291cmNlID0gdmFsdWUuZGF0YTsKICAgICAgICAgICAgICAgICAgICBicmVhazsKICAgICAgICAgICAgICAgIGNhc2UgIkxhbmd1YWdlIjoKICAgICAgICAgICAgICAgICAgICBwcm9ncmFtLmxhbmd1YWdlID0gdmFsdWUuZGF0YTsKICAgICAgICAgICAgICAgICAgICBicmVhazsKICAgICAgICAgICAgICAgIGNhc2UgIlB1Ymxpc2hlciI6CiAgICAgICAgICAgICAgICAgICAgcHJvZ3JhbS5wdWJsaXNoZXIgPSB2YWx1ZS5kYXRhOwogICAgICAgICAgICAgICAgICAgIGJyZWFrOwogICAgICAgICAgICAgICAgY2FzZSAiVW5pbnN0YWxsU3RyaW5nIjoKICAgICAgICAgICAgICAgICAgICBwcm9ncmFtLnVuaW5zdGFsbF9zdHJpbmcgPSB2YWx1ZS5kYXRhOwogICAgICAgICAgICAgICAgICAgIGJyZWFrOwogICAgICAgICAgICAgICAgY2FzZSAiVVJMSW5mb0Fib3V0IjoKICAgICAgICAgICAgICAgICAgICBwcm9ncmFtLnVybF9pbmZvID0gdmFsdWUuZGF0YTsKICAgICAgICAgICAgICAgICAgICBicmVhazsKICAgICAgICAgICAgICAgIGRlZmF1bHQ6CiAgICAgICAgICAgICAgICAgICAgY29udGludWU7CiAgICAgICAgICAgIH0KICAgICAgICB9CiAgICAgICAgcHJvZ3JhbXMucHVzaChwcm9ncmFtKTsKICAgIH0KICAgIHJldHVybiBwcm9ncmFtczsKfQpmdW5jdGlvbiBtYWluKCkgewogICAgY29uc3QgcGF0aCA9ICJDOlxcV2luZG93c1xcU3lzdGVtMzJcXGNvbmZpZ1xcU09GVFdBUkUiOwogICAgY29uc3QgcmVnID0gZ2V0UmVnaXN0cnkocGF0aCk7CiAgICBjb25zdCBwcm9ncmFtcyA9IFtdOwogICAgZm9yIChjb25zdCBlbnRyaWVzIG9mIHJlZyl7CiAgICAgICAgaWYgKCFlbnRyaWVzLnBhdGguaW5jbHVkZXMoIk1pY3Jvc29mdFxcV2luZG93c1xcQ3VycmVudFZlcnNpb25cXFVuaW5zdGFsbCIpKSB7CiAgICAgICAgICAgIGNvbnRpbnVlOwogICAgICAgIH0KICAgICAgICBwcm9ncmFtcy5wdXNoKGVudHJpZXMpOwogICAgfQogICAgcmV0dXJuIGdyYWJfaW5mbyhwcm9ncmFtcyk7Cn0KbWFpbigpOwoK";
        let mut output = output_options("runtime_test", "local", "./tmp", true);
        let script = JSScript {
            name: String::from("programs"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_get_sk_info() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfcmVnaXN0cnkocGF0aCkgewogICAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X3JlZ2lzdHJ5KHBhdGgpOwogICAgY29uc3QgcmVnX2FycmF5ID0gSlNPTi5wYXJzZShkYXRhKTsKICAgIHJldHVybiByZWdfYXJyYXk7Cn0KCmZ1bmN0aW9uIGxvb2t1cFNlY3VyaXR5S2V5KHBhdGgsIG9mZnNldCkgewogICAgaWYgKG9mZnNldCA8PSAwKSB7CiAgICAgIHJldHVybiBuZXcgRXJyb3IoIkNhbm5vdCB1c2UgbmVnYXRpdmUgb2Zmc2V0IG9yIHplcm8hIik7CiAgICB9CiAgICAgIC8vQHRzLWlnbm9yZTogQ3VzdG9tIEFydGVtaXMgZnVuY3Rpb24KICAgICAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X3NrX2luZm8ocGF0aCwgb2Zmc2V0KTsKICAKICAgICAgY29uc3QgcmVzdWx0cyA9IEpTT04ucGFyc2UoZGF0YSk7CiAgICAgIHJldHVybiByZXN1bHRzOwogIH0KCmZ1bmN0aW9uIG1haW4oKSB7CiAgICBjb25zdCBwYXRoID0gIkM6XFxXaW5kb3dzXFxTeXN0ZW0zMlxcY29uZmlnXFxEUklWRVJTIjsKICAgIGNvbnN0IHJlZyA9IGdldF9yZWdpc3RyeShwYXRoKTsKICAgIGZvciAoY29uc3QgZW50cmllcyBvZiByZWcpewogICAgICAgIHJldHVybiBsb29rdXBTZWN1cml0eUtleShwYXRoLCBlbnRyaWVzLnNlY3VyaXR5X29mZnNldCk7CiAgICB9Cn0KbWFpbigpOw==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("sk"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
