use crate::{
    filesystem::{
        directory::is_directory,
        files::{file_extension, get_filename},
        metadata::{get_metadata, get_timestamps},
    },
    runtimev2::helper::string_arg,
};
use boa_engine::{
    builtins::promise::PromiseState, js_string, object::builtins::JsPromise, Context, JsError,
    JsResult, JsValue, NativeFunction,
};
use log::{error, warn};
use serde::Serialize;
use std::path::Path;
use tokio::fs::read_dir;

#[derive(Serialize, Debug)]
pub(crate) struct JsFileInfo {
    pub(crate) full_path: String,
    pub(crate) directory: String,
    pub(crate) filename: String,
    pub(crate) extension: String,
    pub(crate) created: String,
    pub(crate) modified: String,
    pub(crate) changed: String,
    pub(crate) accessed: String,
    pub(crate) size: u64,
    pub(crate) inode: u64,
    pub(crate) mode: u32,
    pub(crate) uid: u32,
    pub(crate) gid: u32,
    pub(crate) is_file: bool,
    pub(crate) is_directory: bool,
    pub(crate) is_symlink: bool,
}

/// List all files and directories at provided directory path
pub(crate) fn js_read_dir(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, &0)?;
    if !is_directory(&path) {
        error!("[runtime] Path is not a directory");
        return Err(JsError::from_opaque(js_string!("Not a directory").into()));
    }

    // Create a promise to execute our async script
    let promise = JsPromise::from_future(
        async move {
            let mut dir = match read_dir(&path).await {
                Ok(result) => result,
                Err(err) => {
                    let issue = format!("Failed to read {path}: {err:?}");
                    return Err(JsError::from_opaque(js_string!(issue).into()));
                }
            };

            let mut files: Vec<JsFileInfo> = Vec::new();
            while let Ok(Some(entry)) = dir.next_entry().await {
                let full_path = entry.path().display().to_string();
                let timestamps = match get_timestamps(&full_path) {
                    Ok(result) => result,
                    Err(err) => {
                        warn!("[runtime] Failed to get timestamps for {path}: {err:?}");
                        continue;
                    }
                };
                let meta = match get_metadata(&full_path) {
                    Ok(result) => result,
                    Err(err) => {
                        warn!("[runtime] Failed to get metadata for {path}: {err:?}");
                        continue;
                    }
                };

                let mut info = JsFileInfo {
                    filename: get_filename(&full_path),
                    extension: file_extension(&full_path),
                    full_path,
                    directory: entry
                        .path()
                        .parent()
                        .unwrap_or_else(|| Path::new(""))
                        .display()
                        .to_string(),
                    created: timestamps.created,
                    modified: timestamps.modified,
                    accessed: timestamps.accessed,
                    changed: timestamps.changed,
                    size: meta.len(),
                    inode: 0,
                    mode: 0,
                    uid: 0,
                    gid: 0,
                    is_file: meta.is_file(),
                    is_directory: meta.is_dir(),
                    is_symlink: false,
                };
                info.is_symlink = meta.is_symlink();

                #[cfg(target_family = "unix")]
                {
                    use std::os::unix::prelude::MetadataExt;
                    info.inode = meta.ino();
                    info.mode = meta.mode();
                    info.uid = meta.uid();
                    info.gid = meta.gid();
                }
                files.push(info);
            }

            // We have to serialize to string for now
            let data = serde_json::to_string(&files).unwrap_or_default();
            Ok(js_string!(data).into())
        },
        context,
    )
    .then(
        Some(
            NativeFunction::from_fn_ptr(|_, args, ctx| {
                // Get the value from the script
                let script_value = string_arg(args, &0)?;
                let serde_value = serde_json::from_str(&script_value).unwrap_or_default();
                let value = JsValue::from_json(&serde_value, ctx)?;
                // Returh the JavaScript object
                Ok(value)
            })
            .to_js_function(context.realm()),
        ),
        None,
        context,
    );

    // Return a promise and let setup.rs handle the results
    Ok(promise.into())
}

#[cfg(test)]
mod tests {
    use crate::runtimev2::run::execute_script;
    use crate::structs::artifacts::runtime::script::JSScript;
    use crate::structs::toml::Output;

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
    #[cfg(target_family = "unix")]
    fn test_read_dir_root() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9kaXJlY3RvcnkudHMKYXN5bmMgZnVuY3Rpb24gcmVhZERpcihwYXRoKSB7CiAgY29uc3QgZGF0YSA9IGF3YWl0IGpzX3JlYWRfZGlyKHBhdGgpOwogIHJldHVybiBkYXRhOwp9CgovLyBtYWluLnRzCmFzeW5jIGZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3Qgc3RhcnQgPSAiLyI7CiAgY29uc3QgZmlsZXMgPSBhd2FpdCByZWFkRGlyKHN0YXJ0KTsKICByZXR1cm4gZmlsZXM7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("root_list"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_read_dir_root_windows() {
        let test = "Ly8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3V0aWxzL2Vycm9yLnRzCnZhciBFcnJvckJhc2UgPSBjbGFzcyBleHRlbmRzIEVycm9yIHsKICBjb25zdHJ1Y3RvcihuYW1lLCBtZXNzYWdlKSB7CiAgICBzdXBlcigpOwogICAgdGhpcy5uYW1lID0gbmFtZTsKICAgIHRoaXMubWVzc2FnZSA9IG1lc3NhZ2U7CiAgfQp9OwoKLy8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL2ZpbGVzeXN0ZW0vZXJyb3JzLnRzCnZhciBGaWxlRXJyb3IgPSBjbGFzcyBleHRlbmRzIEVycm9yQmFzZSB7Cn07CgovLyAuLi8uLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvZmlsZXN5c3RlbS9kaXJlY3RvcnkudHMKYXN5bmMgZnVuY3Rpb24gcmVhZERpcihwYXRoKSB7CiAgdHJ5IHsKICAgIGNvbnN0IHJlc3VsdCA9IGF3YWl0IGpzX3JlYWRfZGlyKHBhdGgpOwogICAgcmV0dXJuIHJlc3VsdDsKICB9IGNhdGNoIChlcnIpIHsKICAgIHJldHVybiBuZXcgRmlsZUVycm9yKAogICAgICAiUkVBRF9ESVIiLAogICAgICBgZmFpbGVkIHRvIHJlYWQgZGlyZWN0b3J5ICR7cGF0aH06ICR7ZXJyfWAKICAgICk7CiAgfQp9CgovLyBtYWluLnRzCmFzeW5jIGZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgcmVzdWx0ID0gYXdhaXQgcmVhZERpcigiQzpcXCIpOwogIHJldHVybiByZXN1bHQ7Cn0KbWFpbigpOwoK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("root_list"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
