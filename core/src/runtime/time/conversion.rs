use crate::{
    runtime::helper::{bigint_arg, bytes_arg},
    utils::time::{
        cocoatime_to_unixepoch, fattime_utc_to_unixepoch, filetime_to_unixepoch, hfs_to_unixepoch,
        ole_automationtime_to_unixepoch, time_now, webkit_time_to_unixepoch,
    },
};
use boa_engine::{Context, JsResult, JsValue};

/// Expose current time now in seconds or 0
pub(crate) fn js_time_now(
    _this: &JsValue,
    _args: &[JsValue],
    _context: &mut Context,
) -> JsResult<JsValue> {
    Ok(JsValue::BigInt(time_now().into()))
}

/// Expose converting filetimes to unixepoch
pub(crate) fn js_filetime_to_unixepoch(
    _this: &JsValue,
    args: &[JsValue],
    _context: &mut Context,
) -> JsResult<JsValue> {
    let filetime = bigint_arg(args, &0)? as u64;

    Ok(JsValue::BigInt(filetime_to_unixepoch(&filetime).into()))
}

/// Expose converting cocoatimes to unixepoch
pub(crate) fn js_cocoatime_to_unixepoch(
    _this: &JsValue,
    args: &[JsValue],
    _context: &mut Context,
) -> JsResult<JsValue> {
    let cocoatime = bigint_arg(args, &0)?;

    Ok(JsValue::BigInt(cocoatime_to_unixepoch(&cocoatime).into()))
}

/// Expose converting HFS+ time to unixepoch
pub(crate) fn js_hfs_to_unixepoch(
    _this: &JsValue,
    args: &[JsValue],
    _context: &mut Context,
) -> JsResult<JsValue> {
    let hfstime = bigint_arg(args, &0)? as i64;

    Ok(JsValue::BigInt(hfs_to_unixepoch(&hfstime).into()))
}

/// Expose converting OLE time to unixepoch
pub(crate) fn js_ole_automationtime_to_unixepoch(
    _this: &JsValue,
    args: &[JsValue],
    _context: &mut Context,
) -> JsResult<JsValue> {
    let oletime = bigint_arg(args, &0)?;

    Ok(JsValue::BigInt(
        ole_automationtime_to_unixepoch(&oletime).into(),
    ))
}

/// Expose converting `WebKit` time to unixepoch
pub(crate) fn js_webkit_time_to_unixepoch(
    _this: &JsValue,
    args: &[JsValue],
    _context: &mut Context,
) -> JsResult<JsValue> {
    let webkittime = bigint_arg(args, &0)? as i64;

    Ok(JsValue::BigInt(
        webkit_time_to_unixepoch(&webkittime).into(),
    ))
}

/// Expose converting `FATTIME` time to unixepoch
pub(crate) fn js_fat_time_to_unixepoch(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let fattime = bytes_arg(args, &0, context)?;
    Ok(JsValue::BigInt(fattime_utc_to_unixepoch(&fattime).into()))
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
    fn test_js_time() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3RpbWUvY29udmVyc2lvbi50cwpmdW5jdGlvbiB0aW1lTm93KCkgewogIGNvbnN0IGRhdGEgPSBqc190aW1lX25vdygpOwogIHJldHVybiBkYXRhOwp9CmZ1bmN0aW9uIGZpbGV0aW1lVG9Vbml4RXBvY2goZmlsZXRpbWUpIHsKICBjb25zdCBkYXRhID0ganNfZmlsZXRpbWVfdG9fdW5peGVwb2NoKGZpbGV0aW1lKTsKICByZXR1cm4gZGF0YTsKfQpmdW5jdGlvbiBjb2NvYXRpbWVUb1VuaXhFcG9jaChjb2NvYXRpbWUpIHsKICBjb25zdCBkYXRhID0ganNfY29jb2F0aW1lX3RvX3VuaXhlcG9jaChjb2NvYXRpbWUpOwogIHJldHVybiBkYXRhOwp9CmZ1bmN0aW9uIGhmc1RvVW5peEVwb2NoKGhmc3RpbWUpIHsKICBjb25zdCBkYXRhID0ganNfaGZzX3RvX3VuaXhlcG9jaChoZnN0aW1lKTsKICByZXR1cm4gZGF0YTsKfQpmdW5jdGlvbiBvbGVUb1VuaXhFcG9jaChvbGV0aW1lKSB7CiAgY29uc3QgZGF0YSA9IGpzX29sZV9hdXRvbWF0aW9udGltZV90b191bml4ZXBvY2gob2xldGltZSk7CiAgcmV0dXJuIGRhdGE7Cn0KZnVuY3Rpb24gd2Via2l0VG9Vbml4RXBvY2god2Via2l0dGltZSkgewogIGNvbnN0IGRhdGEgPSBqc193ZWJraXRfdGltZV90b191bml4ZXBvY2god2Via2l0dGltZSk7CiAgcmV0dXJuIGRhdGE7Cn0KZnVuY3Rpb24gZmF0VG9Vbml4RXBvY2goZmF0dGltZSkgewogIGNvbnN0IGRhdGEgPSBqc19mYXRfdGltZV90b191bml4ZXBvY2goZmF0dGltZSk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBsZXQgZGF0YSA9IHRpbWVOb3coKTsKICBjb25zdCBiaWcgPSAxMzIyNDQ3NjY0MTg5NDAyNTRuOwogIGRhdGEgPSBmaWxldGltZVRvVW5peEVwb2NoKGJpZyk7CiAgY29uc3QgZmF0dGVzdCA9IFsxMjMsIDc5LCAxOTUsIDE0XTsKICBkYXRhID0gZmF0VG9Vbml4RXBvY2goVWludDhBcnJheS5mcm9tKGZhdHRlc3QpKTsKICBsZXQgdGVzdCA9IDQzNzk0LjAxODc1OwogIGRhdGEgPSBvbGVUb1VuaXhFcG9jaCh0ZXN0KTsKICB0ZXN0ID0gMTAuMDE4NzU7CiAgZGF0YSA9IGNvY29hdGltZVRvVW5peEVwb2NoKHRlc3QpOwogIHRlc3QgPSAxMzI4OTk4Mzk2MDsKICBkYXRhID0gd2Via2l0VG9Vbml4RXBvY2godGVzdCk7CiAgdGVzdCA9IDM0NTMxMjA4MjQ7CiAgZGF0YSA9IGhmc1RvVW5peEVwb2NoKHRlc3QpOwogIGNvbnNvbGUubG9nKGRhdGEpOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("timestuff"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
