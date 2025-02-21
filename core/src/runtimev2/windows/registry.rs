use crate::{
    artifacts::os::windows::registry::helper::{get_registry_keys, lookup_sk_info},
    runtimev2::helper::{number_arg, string_arg},
    utils::regex_options::create_regex,
};
use boa_engine::{js_string, Context, JsError, JsResult, JsValue};

/// Expose parsing Registry file to `BoaJS`
pub(crate) fn js_registry(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, &0)?;
    let all = create_regex("").unwrap(); // Valid regex
    let start_root = "";
    let reg = match get_registry_keys(start_root, &all, &path) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to parse registry: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&reg).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

/// Expose parsing the Security Key to `BoaJS`
pub(crate) fn js_sk_info(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, &0)?;
    let offset = number_arg(args, &1)? as i32;

    let sk = match lookup_sk_info(&path, offset) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to parse security key info: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&sk).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
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
    fn test_js_registry() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfcmVnaXN0cnkocGF0aCkgewogICAgY29uc3QgZGF0YSA9IGpzX3JlZ2lzdHJ5KHBhdGgpOwogICAgcmV0dXJuIGRhdGE7Cn0KZnVuY3Rpb24gZ2V0UmVnaXN0cnkocGF0aCkgewogICAgcmV0dXJuIGdldF9yZWdpc3RyeShwYXRoKTsKfQpmdW5jdGlvbiBncmFiX2luZm8ocmVnKSB7CiAgICBjb25zdCBwcm9ncmFtcyA9IFtdOwogICAgZm9yIChjb25zdCBlbnRyaWVzIG9mIHJlZyl7CiAgICAgICAgaWYgKGVudHJpZXMudmFsdWVzLmxlbmd0aCA8IDMpIHsKICAgICAgICAgICAgY29udGludWU7CiAgICAgICAgfQogICAgICAgIGNvbnN0IHByb2dyYW0gPSB7CiAgICAgICAgICAgIG5hbWU6ICIiLAogICAgICAgICAgICB2ZXJzaW9uOiAiIiwKICAgICAgICAgICAgaW5zdGFsbF9sb2NhdGlvbjogIiIsCiAgICAgICAgICAgIGluc3RhbGxfc291cmNlOiAiIiwKICAgICAgICAgICAgbGFuZ3VhZ2U6ICIiLAogICAgICAgICAgICBwdWJsaXNoZXI6ICIiLAogICAgICAgICAgICBpbnN0YWxsX3N0cmluZzogIiIsCiAgICAgICAgICAgIGluc3RhbGxfZGF0ZTogIiIsCiAgICAgICAgICAgIHVuaW5zdGFsbF9zdHJpbmc6ICIiLAogICAgICAgICAgICB1cmxfaW5mbzogIiIsCiAgICAgICAgICAgIHJlZ19wYXRoOiBlbnRyaWVzLnBhdGgKICAgICAgICB9OwogICAgICAgIGZvciAoY29uc3QgdmFsdWUgb2YgZW50cmllcy52YWx1ZXMpewogICAgICAgICAgICBzd2l0Y2godmFsdWUudmFsdWUpewogICAgICAgICAgICAgICAgY2FzZSAiRGlzcGxheU5hbWUiOgogICAgICAgICAgICAgICAgICAgIHByb2dyYW0ubmFtZSA9IHZhbHVlLmRhdGE7CiAgICAgICAgICAgICAgICAgICAgYnJlYWs7CiAgICAgICAgICAgICAgICBjYXNlICJEaXNwbGF5VmVyc2lvbiI6CiAgICAgICAgICAgICAgICAgICAgcHJvZ3JhbS52ZXJzaW9uID0gdmFsdWUuZGF0YTsKICAgICAgICAgICAgICAgICAgICBicmVhazsKICAgICAgICAgICAgICAgIGNhc2UgIkluc3RhbGxEYXRlIjoKICAgICAgICAgICAgICAgICAgICBwcm9ncmFtLmluc3RhbGxfZGF0ZSA9IHZhbHVlLmRhdGE7CiAgICAgICAgICAgICAgICAgICAgYnJlYWs7CiAgICAgICAgICAgICAgICBjYXNlICJJbnN0YWxsTG9jYXRpb24iOgogICAgICAgICAgICAgICAgICAgIHByb2dyYW0uaW5zdGFsbF9sb2NhdGlvbiA9IHZhbHVlLmRhdGE7CiAgICAgICAgICAgICAgICAgICAgYnJlYWs7CiAgICAgICAgICAgICAgICBjYXNlICJJbnN0YWxsU291cmNlIjoKICAgICAgICAgICAgICAgICAgICBwcm9ncmFtLmluc3RhbGxfc291cmNlID0gdmFsdWUuZGF0YTsKICAgICAgICAgICAgICAgICAgICBicmVhazsKICAgICAgICAgICAgICAgIGNhc2UgIkxhbmd1YWdlIjoKICAgICAgICAgICAgICAgICAgICBwcm9ncmFtLmxhbmd1YWdlID0gdmFsdWUuZGF0YTsKICAgICAgICAgICAgICAgICAgICBicmVhazsKICAgICAgICAgICAgICAgIGNhc2UgIlB1Ymxpc2hlciI6CiAgICAgICAgICAgICAgICAgICAgcHJvZ3JhbS5wdWJsaXNoZXIgPSB2YWx1ZS5kYXRhOwogICAgICAgICAgICAgICAgICAgIGJyZWFrOwogICAgICAgICAgICAgICAgY2FzZSAiVW5pbnN0YWxsU3RyaW5nIjoKICAgICAgICAgICAgICAgICAgICBwcm9ncmFtLnVuaW5zdGFsbF9zdHJpbmcgPSB2YWx1ZS5kYXRhOwogICAgICAgICAgICAgICAgICAgIGJyZWFrOwogICAgICAgICAgICAgICAgY2FzZSAiVVJMSW5mb0Fib3V0IjoKICAgICAgICAgICAgICAgICAgICBwcm9ncmFtLnVybF9pbmZvID0gdmFsdWUuZGF0YTsKICAgICAgICAgICAgICAgICAgICBicmVhazsKICAgICAgICAgICAgICAgIGRlZmF1bHQ6CiAgICAgICAgICAgICAgICAgICAgY29udGludWU7CiAgICAgICAgICAgIH0KICAgICAgICB9CiAgICAgICAgcHJvZ3JhbXMucHVzaChwcm9ncmFtKTsKICAgIH0KICAgIHJldHVybiBwcm9ncmFtczsKfQpmdW5jdGlvbiBtYWluKCkgewogICAgY29uc3QgcGF0aCA9ICJDOlxcV2luZG93c1xcU3lzdGVtMzJcXGNvbmZpZ1xcU09GVFdBUkUiOwogICAgY29uc3QgcmVnID0gZ2V0UmVnaXN0cnkocGF0aCk7CiAgICBjb25zdCBwcm9ncmFtcyA9IFtdOwogICAgZm9yIChjb25zdCBlbnRyaWVzIG9mIHJlZyl7CiAgICAgICAgaWYgKCFlbnRyaWVzLnBhdGguaW5jbHVkZXMoIk1pY3Jvc29mdFxcV2luZG93c1xcQ3VycmVudFZlcnNpb25cXFVuaW5zdGFsbCIpKSB7CiAgICAgICAgICAgIGNvbnRpbnVlOwogICAgICAgIH0KICAgICAgICBwcm9ncmFtcy5wdXNoKGVudHJpZXMpOwogICAgfQogICAgcmV0dXJuIGdyYWJfaW5mbyhwcm9ncmFtcyk7Cn0KbWFpbigpOwoK";
        let mut output = output_options("runtime_test", "local", "./tmp", true);
        let script = JSScript {
            name: String::from("programs"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_js_sk_info() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfcmVnaXN0cnkocGF0aCkgewogICAgY29uc3QgZGF0YSA9IGpzX3JlZ2lzdHJ5KHBhdGgpOwogICAgcmV0dXJuIGRhdGE7Cn0KCmZ1bmN0aW9uIGxvb2t1cFNlY3VyaXR5S2V5KHBhdGgsIG9mZnNldCkgewogICAgaWYgKG9mZnNldCA8PSAwKSB7CiAgICAgIHJldHVybiBuZXcgRXJyb3IoIkNhbm5vdCB1c2UgbmVnYXRpdmUgb2Zmc2V0IG9yIHplcm8hIik7CiAgICB9CiAgICAgIC8vQHRzLWlnbm9yZTogQ3VzdG9tIEFydGVtaXMgZnVuY3Rpb24KICAgICAgY29uc3QgZGF0YSA9IGpzX3NrX2luZm8ocGF0aCwgb2Zmc2V0KTsKICAKICAgICAgcmV0dXJuIGRhdGE7CiAgfQoKZnVuY3Rpb24gbWFpbigpIHsKICAgIGNvbnN0IHBhdGggPSAiQzpcXFdpbmRvd3NcXFN5c3RlbTMyXFxjb25maWdcXERSSVZFUlMiOwogICAgY29uc3QgcmVnID0gZ2V0X3JlZ2lzdHJ5KHBhdGgpOwogICAgZm9yIChjb25zdCBlbnRyaWVzIG9mIHJlZyl7CiAgICAgICAgcmV0dXJuIGxvb2t1cFNlY3VyaXR5S2V5KHBhdGgsIGVudHJpZXMuc2VjdXJpdHlfb2Zmc2V0KTsKICAgIH0KfQptYWluKCk7";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("sk"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
