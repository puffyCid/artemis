use crate::artifacts::os::systeminfo::info::get_info;
use boa_engine::{js_string, Context, JsResult, JsValue};

/// Expose pulling systeminfo to `BoaJS`
pub(crate) fn js_get_systeminfo(
    _this: &JsValue,
    _args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let info = get_info();
    let results = serde_json::to_value(&info).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

/// Return uptime of the system
pub(crate) fn js_uptime(
    _this: &JsValue,
    _args: &[JsValue],
    _context: &mut Context,
) -> JsResult<JsValue> {
    Ok(JsValue::BigInt(sysinfo::System::uptime().into()))
}

/// Return hostname of the system
pub(crate) fn js_hostname(
    _this: &JsValue,
    _args: &[JsValue],
    _context: &mut Context,
) -> JsResult<JsValue> {
    Ok(js_string!(sysinfo::System::host_name()
        .unwrap_or_else(|| String::from("Unknown hostname"))).into())
}

/// Return OS version of the system
pub(crate) fn js_os_version(
    _this: &JsValue,
    _args: &[JsValue],
    _context: &mut Context,
) -> JsResult<JsValue> {
    Ok(js_string!(
        sysinfo::System::os_version().unwrap_or_else(|| String::from("Unknown OS version"))
    )
    .into())
}

/// Returns kernel version of the system
pub(crate) fn js_kernel_version(
    _this: &JsValue,
    _args: &[JsValue],
    _context: &mut Context,
) -> JsResult<JsValue> {
    Ok(js_string!(
        sysinfo::System::kernel_version().unwrap_or_else(|| String::from("Unknown Kernel version"))
    )
    .into())
}

/// Returns the platform of the system
pub(crate) fn js_platform(
    _this: &JsValue,
    _args: &[JsValue],
    _context: &mut Context,
) -> JsResult<JsValue> {
    Ok(
        js_string!(sysinfo::System::name().unwrap_or_else(|| String::from("Unknown platform")))
            .into(),
    )
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
    fn test_get_systeminfo() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3dpbmRvd3Mvc3lzdGVtaW5mby50cwpmdW5jdGlvbiBnZXRfc3lzdGVtaW5mb193aW4oKSB7CiAgY29uc3QgZGF0YSA9IGpzX2dldF9zeXN0ZW1pbmZvKCk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIC4uLy4uL2FydGVtaXMtYXBpL21vZC50cwpmdW5jdGlvbiBnZXRTeXN0ZW1JbmZvV2luKCkgewogIHJldHVybiBnZXRfc3lzdGVtaW5mb193aW4oKTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IGluZm8gPSBnZXRTeXN0ZW1JbmZvV2luKCk7CiAgcmV0dXJuIGluZm87Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("systeminfo"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_js_uptime() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvc3lzdGVtL3N5c3RlbWluZm8udHMKZnVuY3Rpb24gdXB0aW1lKCkgewogIGNvbnN0IGRhdGEgPSBqc191cHRpbWUoKTsKICByZXR1cm4gZGF0YTsKfQpmdW5jdGlvbiBvc1ZlcnNpb24oKSB7CiAgY29uc3QgZGF0YSA9IGpzX29zX3ZlcnNpb24oKTsKICByZXR1cm4gZGF0YTsKfQpmdW5jdGlvbiBrZXJuZWxWZXJzaW9uKCkgewogIGNvbnN0IGRhdGEgPSBqc19rZXJuZWxfdmVyc2lvbigpOwogIHJldHVybiBkYXRhOwp9CmZ1bmN0aW9uIHBsYXRmb3JtKCkgewogIGNvbnN0IGRhdGEgPSBqc19wbGF0Zm9ybSgpOwogIHJldHVybiBkYXRhOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9zeXN0ZW0vZGlza3MudHMKZnVuY3Rpb24gZGlza3MoKSB7CiAgY29uc3QgZGF0YSA9IGpzX2Rpc2tzKCk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL3N5c3RlbS9jcHUudHMKZnVuY3Rpb24gY3B1cygpIHsKICBjb25zdCBkYXRhID0ganNfY3B1KCk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL3N5c3RlbS9tZW1vcnkudHMKZnVuY3Rpb24gbWVtb3J5KCkgewogIGNvbnN0IGRhdGEgPSBqc19tZW1vcnkoKTsKICByZXR1cm4gZGF0YTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHRpbWUgPSB1cHRpbWUoKTsKICBjb25zdCBrZXJuZWwgPSBrZXJuZWxWZXJzaW9uKCk7CiAgY29uc3Qgb3MgPSBvc1ZlcnNpb24oKTsKICBjb25zdCBpbmZvID0gcGxhdGZvcm0oKTsKICBjb25zdCBkaXNrID0gZGlza3MoKTsKICBjb25zdCBtZW0gPSBtZW1vcnkoKTsKICBjb25zdCBjcHUgPSBjcHVzKCk7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("systeminfo"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_js_platform() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvc3lzdGVtL3N5c3RlbWluZm8udHMKZnVuY3Rpb24gdXB0aW1lKCkgewogIGNvbnN0IGRhdGEgPSBqc191cHRpbWUoKTsKICByZXR1cm4gZGF0YTsKfQpmdW5jdGlvbiBob3N0bmFtZSgpIHsKICBjb25zdCBkYXRhID0ganNfaG9zdG5hbWUoKTsKICByZXR1cm4gZGF0YTsKfQpmdW5jdGlvbiBvc1ZlcnNpb24oKSB7CiAgY29uc3QgZGF0YSA9IGpzX29zX3ZlcnNpb24oKTsKICByZXR1cm4gZGF0YTsKfQpmdW5jdGlvbiBrZXJuZWxWZXJzaW9uKCkgewogIGNvbnN0IGRhdGEgPSBqc19rZXJuZWxfdmVyc2lvbigpOwogIHJldHVybiBkYXRhOwp9CmZ1bmN0aW9uIHBsYXRmb3JtKCkgewogIGNvbnN0IGRhdGEgPSBqc19wbGF0Zm9ybSgpOwogIHJldHVybiBkYXRhOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9zeXN0ZW0vZGlza3MudHMKZnVuY3Rpb24gZGlza3MoKSB7CiAgY29uc3QgZGF0YSA9IGpzX2Rpc2tzKCk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL3N5c3RlbS9jcHUudHMKZnVuY3Rpb24gY3B1cygpIHsKICBjb25zdCBkYXRhID0ganNfY3B1KCk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL3N5c3RlbS9tZW1vcnkudHMKZnVuY3Rpb24gbWVtb3J5KCkgewogIGNvbnN0IGRhdGEgPSBqc19tZW1vcnkoKTsKICByZXR1cm4gZGF0YTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHRpbWUgPSB1cHRpbWUoKTsKICBjb25zdCBrZXJuZWwgPSBrZXJuZWxWZXJzaW9uKCk7CiAgY29uc3Qgb3MgPSBvc1ZlcnNpb24oKTsKICBjb25zdCBpbmZvID0gcGxhdGZvcm0oKTsKICBjb25zdCBkaXNrID0gZGlza3MoKTsKICBjb25zdCBtZW0gPSBtZW1vcnkoKTsKICBjb25zdCBjcHUgPSBjcHVzKCk7CiAgY29uc3QgaG9zdCA9IGhvc3RuYW1lKCk7CiAgY29uc29sZS5sb2coCiAgICBgVXB0aW1lOiAke3RpbWV9IC0gS2VybmVsOiAke2tlcm5lbH0gLSBPUzogJHtvc30gLSBQbGF0Zm9ybTogJHtpbmZvfSAtIEhvc3RuYW1lOiAke2hvc3R9YAogICk7CiAgY29uc29sZS5sb2coCiAgICBgRGlza3MgU3BhY2U6ICR7ZGlza1swXS50b3RhbF9zcGFjZX0gLSBUb3RhbCBNZW1vcnk6ICR7bWVtLnRvdGFsX21lbW9yeX0gLSBDUFUgQnJhbmQ6ICR7Y3B1WzBdLmJyYW5kfWAKICApOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("systeminfo"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_js_hostname() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvc3lzdGVtL3N5c3RlbWluZm8udHMKZnVuY3Rpb24gdXB0aW1lKCkgewogIGNvbnN0IGRhdGEgPSBqc191cHRpbWUoKTsKICByZXR1cm4gZGF0YTsKfQpmdW5jdGlvbiBob3N0bmFtZSgpIHsKICBjb25zdCBkYXRhID0ganNfaG9zdG5hbWUoKTsKICByZXR1cm4gZGF0YTsKfQpmdW5jdGlvbiBvc1ZlcnNpb24oKSB7CiAgY29uc3QgZGF0YSA9IGpzX29zX3ZlcnNpb24oKTsKICByZXR1cm4gZGF0YTsKfQpmdW5jdGlvbiBrZXJuZWxWZXJzaW9uKCkgewogIGNvbnN0IGRhdGEgPSBqc19rZXJuZWxfdmVyc2lvbigpOwogIHJldHVybiBkYXRhOwp9CmZ1bmN0aW9uIHBsYXRmb3JtKCkgewogIGNvbnN0IGRhdGEgPSBqc19wbGF0Zm9ybSgpOwogIHJldHVybiBkYXRhOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9zeXN0ZW0vZGlza3MudHMKZnVuY3Rpb24gZGlza3MoKSB7CiAgY29uc3QgZGF0YSA9IGpzX2Rpc2tzKCk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL3N5c3RlbS9jcHUudHMKZnVuY3Rpb24gY3B1cygpIHsKICBjb25zdCBkYXRhID0ganNfY3B1KCk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL3N5c3RlbS9tZW1vcnkudHMKZnVuY3Rpb24gbWVtb3J5KCkgewogIGNvbnN0IGRhdGEgPSBqc19tZW1vcnkoKTsKICByZXR1cm4gZGF0YTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHRpbWUgPSB1cHRpbWUoKTsKICBjb25zdCBrZXJuZWwgPSBrZXJuZWxWZXJzaW9uKCk7CiAgY29uc3Qgb3MgPSBvc1ZlcnNpb24oKTsKICBjb25zdCBpbmZvID0gcGxhdGZvcm0oKTsKICBjb25zdCBkaXNrID0gZGlza3MoKTsKICBjb25zdCBtZW0gPSBtZW1vcnkoKTsKICBjb25zdCBjcHUgPSBjcHVzKCk7CiAgY29uc3QgaG9zdCA9IGhvc3RuYW1lKCk7CiAgY29uc29sZS5sb2coCiAgICBgVXB0aW1lOiAke3RpbWV9IC0gS2VybmVsOiAke2tlcm5lbH0gLSBPUzogJHtvc30gLSBQbGF0Zm9ybTogJHtpbmZvfSAtIEhvc3RuYW1lOiAke2hvc3R9YAogICk7CiAgY29uc29sZS5sb2coCiAgICBgRGlza3MgU3BhY2U6ICR7ZGlza1swXS50b3RhbF9zcGFjZX0gLSBUb3RhbCBNZW1vcnk6ICR7bWVtLnRvdGFsX21lbW9yeX0gLSBDUFUgQnJhbmQ6ICR7Y3B1WzBdLmJyYW5kfWAKICApOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("systeminfo"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_js_os_version() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvc3lzdGVtL3N5c3RlbWluZm8udHMKZnVuY3Rpb24gdXB0aW1lKCkgewogIGNvbnN0IGRhdGEgPSBqc191cHRpbWUoKTsKICByZXR1cm4gZGF0YTsKfQpmdW5jdGlvbiBvc1ZlcnNpb24oKSB7CiAgY29uc3QgZGF0YSA9IGpzX29zX3ZlcnNpb24oKTsKICByZXR1cm4gZGF0YTsKfQpmdW5jdGlvbiBrZXJuZWxWZXJzaW9uKCkgewogIGNvbnN0IGRhdGEgPSBqc19rZXJuZWxfdmVyc2lvbigpOwogIHJldHVybiBkYXRhOwp9CmZ1bmN0aW9uIHBsYXRmb3JtKCkgewogIGNvbnN0IGRhdGEgPSBqc19wbGF0Zm9ybSgpOwogIHJldHVybiBkYXRhOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9zeXN0ZW0vZGlza3MudHMKZnVuY3Rpb24gZGlza3MoKSB7CiAgY29uc3QgZGF0YSA9IGpzX2Rpc2tzKCk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL3N5c3RlbS9jcHUudHMKZnVuY3Rpb24gY3B1cygpIHsKICBjb25zdCBkYXRhID0ganNfY3B1KCk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL3N5c3RlbS9tZW1vcnkudHMKZnVuY3Rpb24gbWVtb3J5KCkgewogIGNvbnN0IGRhdGEgPSBqc19tZW1vcnkoKTsKICByZXR1cm4gZGF0YTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHRpbWUgPSB1cHRpbWUoKTsKICBjb25zdCBrZXJuZWwgPSBrZXJuZWxWZXJzaW9uKCk7CiAgY29uc3Qgb3MgPSBvc1ZlcnNpb24oKTsKICBjb25zdCBpbmZvID0gcGxhdGZvcm0oKTsKICBjb25zdCBkaXNrID0gZGlza3MoKTsKICBjb25zdCBtZW0gPSBtZW1vcnkoKTsKICBjb25zdCBjcHUgPSBjcHVzKCk7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("systeminfo"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_js_kernel_version() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvc3lzdGVtL3N5c3RlbWluZm8udHMKZnVuY3Rpb24gdXB0aW1lKCkgewogIGNvbnN0IGRhdGEgPSBqc191cHRpbWUoKTsKICByZXR1cm4gZGF0YTsKfQpmdW5jdGlvbiBvc1ZlcnNpb24oKSB7CiAgY29uc3QgZGF0YSA9IGpzX29zX3ZlcnNpb24oKTsKICByZXR1cm4gZGF0YTsKfQpmdW5jdGlvbiBrZXJuZWxWZXJzaW9uKCkgewogIGNvbnN0IGRhdGEgPSBqc19rZXJuZWxfdmVyc2lvbigpOwogIHJldHVybiBkYXRhOwp9CmZ1bmN0aW9uIHBsYXRmb3JtKCkgewogIGNvbnN0IGRhdGEgPSBqc19wbGF0Zm9ybSgpOwogIHJldHVybiBkYXRhOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9zeXN0ZW0vZGlza3MudHMKZnVuY3Rpb24gZGlza3MoKSB7CiAgY29uc3QgZGF0YSA9IGpzX2Rpc2tzKCk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL3N5c3RlbS9jcHUudHMKZnVuY3Rpb24gY3B1cygpIHsKICBjb25zdCBkYXRhID0ganNfY3B1KCk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL3N5c3RlbS9tZW1vcnkudHMKZnVuY3Rpb24gbWVtb3J5KCkgewogIGNvbnN0IGRhdGEgPSBqc19tZW1vcnkoKTsKICByZXR1cm4gZGF0YTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHRpbWUgPSB1cHRpbWUoKTsKICBjb25zdCBrZXJuZWwgPSBrZXJuZWxWZXJzaW9uKCk7CiAgY29uc3Qgb3MgPSBvc1ZlcnNpb24oKTsKICBjb25zdCBpbmZvID0gcGxhdGZvcm0oKTsKICBjb25zdCBkaXNrID0gZGlza3MoKTsKICBjb25zdCBtZW0gPSBtZW1vcnkoKTsKICBjb25zdCBjcHUgPSBjcHVzKCk7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("systeminfo"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
