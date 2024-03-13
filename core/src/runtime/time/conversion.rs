use crate::utils::time::{
    cocoatime_to_unixepoch, fattime_utc_to_unixepoch, filetime_to_unixepoch, hfs_to_unixepoch,
    ole_automationtime_to_unixepoch, time_now, webkit_time_to_unixepoch,
};
use deno_core::{op2, JsBuffer};

#[op2(fast)]
#[bigint]
/// Expose current time now in seconds or 0
pub(crate) fn js_time_now() -> u64 {
    time_now()
}

#[op2(fast)]
#[bigint]
/// Expose converting filetimes to unixepoch
pub(crate) fn js_filetime_to_unixepoch(#[bigint] filetime: u64) -> i64 {
    filetime_to_unixepoch(&filetime)
}

#[op2(fast)]
#[bigint]
/// Expose converting cocoatimes to unixepoch
pub(crate) fn js_cocoatime_to_unixepoch(cocoatime: f64) -> i64 {
    cocoatime_to_unixepoch(&cocoatime)
}

#[op2(fast)]
#[bigint]
/// Expose converting HFS+ time to unixepoch
pub(crate) fn js_hfs_to_unixepoch(#[bigint] hfstime: i64) -> i64 {
    hfs_to_unixepoch(&hfstime)
}

#[op2(fast)]
#[bigint]
/// Expose converting OLE time to unixepoch
pub(crate) fn js_ole_automationtime_to_unixepoch(oletime: f64) -> i64 {
    ole_automationtime_to_unixepoch(&oletime)
}

#[op2(fast)]
#[bigint]
/// Expose converting WebKit time to unixepoch
pub(crate) fn js_webkit_time_to_uniexepoch(#[bigint] webkittime: i64) -> i64 {
    webkit_time_to_unixepoch(&webkittime)
}

#[op2]
#[bigint]
/// Expose converting WebKit time to unixepoch
pub(crate) fn js_fat_time_to_unixepoch(#[buffer] fattime: JsBuffer) -> i64 {
    fattime_utc_to_unixepoch(&fattime)
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
    fn test_js_time() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3RpbWUvY29udmVyc2lvbi50cwpmdW5jdGlvbiB0aW1lTm93KCkgewogIGNvbnN0IGRhdGEgPSB0aW1lLnRpbWVfbm93KCk7CiAgcmV0dXJuIGRhdGE7Cn0KZnVuY3Rpb24gZmlsZXRpbWVUb1VuaXhFcG9jaChmaWxldGltZSkgewogIGNvbnN0IGRhdGEgPSB0aW1lLmZpbGV0aW1lX3RvX3VuaXhlcG9jaChmaWxldGltZSk7CiAgcmV0dXJuIGRhdGE7Cn0KZnVuY3Rpb24gY29jb2F0aW1lVG9Vbml4RXBvY2goY29jb2F0aW1lKSB7CiAgY29uc3QgZGF0YSA9IHRpbWUuY29jb2F0aW1lX3RvX3VuaXhlcG9jaChjb2NvYXRpbWUpOwogIHJldHVybiBkYXRhOwp9CmZ1bmN0aW9uIGhmc1RvVW5peEVwb2NoKGhmc3RpbWUpIHsKICBjb25zdCBkYXRhID0gdGltZS5oZnNfdG9fdW5peGVwb2NoKGhmc3RpbWUpOwogIHJldHVybiBkYXRhOwp9CmZ1bmN0aW9uIG9sZVRvVW5peEVwb2NoKG9sZXRpbWUpIHsKICBjb25zdCBkYXRhID0gdGltZS5vbGVfYXV0b21hdGlvbnRpbWVfdG9fdW5peGVwb2NoKG9sZXRpbWUpOwogIHJldHVybiBkYXRhOwp9CmZ1bmN0aW9uIHdlYmtpdFRvVW5peEVwb2NoKHdlYmtpdHRpbWUpIHsKICBjb25zdCBkYXRhID0gdGltZS53ZWJraXRfdGltZV90b191bml4ZXBvY2god2Via2l0dGltZSk7CiAgcmV0dXJuIGRhdGE7Cn0KZnVuY3Rpb24gZmF0VG9Vbml4RXBvY2goZmF0dGltZSkgewogIGNvbnN0IGRhdGEgPSB0aW1lLmZhdHRpbWVfdXRjX3RvX3VuaXhlcG9jaChmYXR0aW1lKTsKICByZXR1cm4gZGF0YTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGxldCBkYXRhID0gdGltZU5vdygpOwogIGNvbnN0IGJpZyA9IDEzMjI0NDc2NjQxODk0MDI1NG47CiAgZGF0YSA9IGZpbGV0aW1lVG9Vbml4RXBvY2goYmlnKTsKICBjb25zdCBmYXR0ZXN0ID0gWzEyMywgNzksIDE5NSwgMTRdOwogIGRhdGEgPSBmYXRUb1VuaXhFcG9jaChVaW50OEFycmF5LmZyb20oZmF0dGVzdCkpOwogIGxldCB0ZXN0ID0gNDM3OTQuMDE4NzU7CiAgZGF0YSA9IG9sZVRvVW5peEVwb2NoKHRlc3QpOwogIHRlc3QgPSAxMC4wMTg3NTsKICBkYXRhID0gY29jb2F0aW1lVG9Vbml4RXBvY2godGVzdCk7CiAgdGVzdCA9IDEzMjg5OTgzOTYwOwogIGRhdGEgPSB3ZWJraXRUb1VuaXhFcG9jaCh0ZXN0KTsKICB0ZXN0ID0gMzQ1MzEyMDgyNDsKICBkYXRhID0gaGZzVG9Vbml4RXBvY2godGVzdCk7CiAgY29uc29sZS5sb2coZGF0YSk7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("timestuff"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
