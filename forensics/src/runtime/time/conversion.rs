use crate::{
    runtime::helper::{bigint_arg, bytes_arg},
    utils::time::{
        cocoatime_to_iso, fattime_utc_to_iso, filetime_to_iso, ole_automationtime_to_iso, time_now,
    },
};
use boa_engine::{Context, JsBigInt, JsResult, JsValue, js_string};

/// Expose current time now in seconds or 0
pub(crate) fn js_time_now(
    _this: &JsValue,
    _args: &[JsValue],
    _context: &mut Context,
) -> JsResult<JsValue> {
    Ok(JsValue::new::<JsBigInt>(time_now().into()))
}

/// Expose converting filetimes to unixepoch
pub(crate) fn js_filetime_to_iso(
    _this: &JsValue,
    args: &[JsValue],
    _context: &mut Context,
) -> JsResult<JsValue> {
    let filetime = bigint_arg(args, 0)? as u64;

    Ok(js_string!(filetime_to_iso(filetime)).into())
}

/// Expose converting cocoatimes to unixepoch
pub(crate) fn js_cocoatime_to_iso(
    _this: &JsValue,
    args: &[JsValue],
    _context: &mut Context,
) -> JsResult<JsValue> {
    let cocoatime = bigint_arg(args, 0)?;

    Ok(js_string!(cocoatime_to_iso(cocoatime)).into())
}

/// Expose converting OLE time to unixepoch
pub(crate) fn js_ole_automationtime_to_iso(
    _this: &JsValue,
    args: &[JsValue],
    _context: &mut Context,
) -> JsResult<JsValue> {
    let oletime = bigint_arg(args, 0)?;

    Ok(js_string!(ole_automationtime_to_iso(oletime)).into())
}

/// Expose converting `FATTIME` time to unixepoch
pub(crate) fn js_fat_time_to_iso(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let fattime = bytes_arg(args, 0, context)?;
    Ok(js_string!(fattime_utc_to_iso(&fattime)).into())
}

#[cfg(test)]
mod tests {
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use crate::{
        output2::manager::OutputManager, runtime::run::execute_script,
        structs::artifacts::runtime::script::JSScript,
    };
    use std::path::PathBuf;

    fn output_options(name: &str, directory: &str, compress: bool) -> OutputManager {
        let config = OutputConfig {
            name: name.to_string(),
            directory: PathBuf::from(directory),
            format: OutputFormat::Jsonl,
            compress,
            endpoint_id: String::from("abcd"),
            destination: OutputDestination::Local,
            ..Default::default()
        };
        OutputManager::new(config).unwrap()
    }

    #[test]
    fn test_js_time() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3RpbWUvY29udmVyc2lvbi50cwpmdW5jdGlvbiB0aW1lTm93KCkgewogIGNvbnN0IGRhdGEgPSBqc190aW1lX25vdygpOwogIHJldHVybiBkYXRhOwp9CmZ1bmN0aW9uIGZpbGV0aW1lVG9Vbml4RXBvY2goZmlsZXRpbWUpIHsKICBjb25zdCBkYXRhID0ganNfZmlsZXRpbWVfdG9faXNvKGZpbGV0aW1lKTsKICByZXR1cm4gZGF0YTsKfQpmdW5jdGlvbiBjb2NvYXRpbWVUb1VuaXhFcG9jaChjb2NvYXRpbWUpIHsKICBjb25zdCBkYXRhID0ganNfY29jb2F0aW1lX3RvX2lzbyhjb2NvYXRpbWUpOwogIHJldHVybiBkYXRhOwp9CgpmdW5jdGlvbiBvbGVUb1VuaXhFcG9jaChvbGV0aW1lKSB7CiAgY29uc3QgZGF0YSA9IGpzX29sZV9hdXRvbWF0aW9udGltZV90b19pc28ob2xldGltZSk7CiAgcmV0dXJuIGRhdGE7Cn0KCmZ1bmN0aW9uIGZhdFRvVW5peEVwb2NoKGZhdHRpbWUpIHsKICBjb25zdCBkYXRhID0ganNfZmF0X3RpbWVfdG9faXNvKGZhdHRpbWUpOwogIHJldHVybiBkYXRhOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgbGV0IGRhdGEgPSB0aW1lTm93KCk7CiAgY29uc3QgYmlnID0gMTMyMjQ0NzY2NDE4OTQwMjU0bjsKICBkYXRhID0gZmlsZXRpbWVUb1VuaXhFcG9jaChiaWcpOwogIGNvbnN0IGZhdHRlc3QgPSBbMTIzLCA3OSwgMTk1LCAxNF07CiAgZGF0YSA9IGZhdFRvVW5peEVwb2NoKFVpbnQ4QXJyYXkuZnJvbShmYXR0ZXN0KSk7CiAgbGV0IHRlc3QgPSA0Mzc5NC4wMTg3NTsKICBkYXRhID0gb2xlVG9Vbml4RXBvY2godGVzdCk7CiAgdGVzdCA9IDEwLjAxODc1OwogIGRhdGEgPSBjb2NvYXRpbWVUb1VuaXhFcG9jaCh0ZXN0KTsKICBjb25zb2xlLmxvZyhkYXRhKTsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "./tmp", false);
        let script = JSScript {
            name: String::from("timestuff"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
