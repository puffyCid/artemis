use crate::artifacts::os::systeminfo::info::get_cpu;
use boa_engine::{Context, JsResult, JsValue};
use sysinfo::System;

/// Return cpu info about the system
pub(crate) fn js_cpu(
    _this: &JsValue,
    _args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let mut info = System::new();
    let cpu = get_cpu(&mut info);
    let results = serde_json::to_value(&cpu).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use crate::{
        runtime::run::execute_script,
        structs::{artifacts::runtime::script::JSScript, toml::Output},
    };

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("json"),
            compress,
            endpoint_id: String::from("abcd"),
            output: output.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_js_cpu_info() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvc3lzdGVtL3N5c3RlbWluZm8udHMKZnVuY3Rpb24gdXB0aW1lKCkgewogIGNvbnN0IGRhdGEgPSBqc191cHRpbWUoKTsKICByZXR1cm4gZGF0YTsKfQpmdW5jdGlvbiBvc1ZlcnNpb24oKSB7CiAgY29uc3QgZGF0YSA9IGpzX29zX3ZlcnNpb24oKTsKICByZXR1cm4gZGF0YTsKfQpmdW5jdGlvbiBrZXJuZWxWZXJzaW9uKCkgewogIGNvbnN0IGRhdGEgPSBqc19rZXJuZWxfdmVyc2lvbigpOwogIHJldHVybiBkYXRhOwp9CmZ1bmN0aW9uIHBsYXRmb3JtKCkgewogIGNvbnN0IGRhdGEgPSBqc19wbGF0Zm9ybSgpOwogIHJldHVybiBkYXRhOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9zeXN0ZW0vZGlza3MudHMKZnVuY3Rpb24gZGlza3MoKSB7CiAgY29uc3QgZGF0YSA9IGpzX2Rpc2tzKCk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL3N5c3RlbS9jcHUudHMKZnVuY3Rpb24gY3B1cygpIHsKICBjb25zdCBkYXRhID0ganNfY3B1KCk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL3N5c3RlbS9tZW1vcnkudHMKZnVuY3Rpb24gbWVtb3J5KCkgewogIGNvbnN0IGRhdGEgPSBqc19tZW1vcnkoKTsKICByZXR1cm4gZGF0YTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHRpbWUgPSB1cHRpbWUoKTsKICBjb25zdCBrZXJuZWwgPSBrZXJuZWxWZXJzaW9uKCk7CiAgY29uc3Qgb3MgPSBvc1ZlcnNpb24oKTsKICBjb25zdCBpbmZvID0gcGxhdGZvcm0oKTsKICBjb25zdCBkaXNrID0gZGlza3MoKTsKICBjb25zdCBtZW0gPSBtZW1vcnkoKTsKICBjb25zdCBjcHUgPSBjcHVzKCk7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("systeminfo"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
