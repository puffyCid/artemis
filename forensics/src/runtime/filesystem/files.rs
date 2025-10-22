use super::directory::JsFileInfo;
use crate::{
    filesystem::{
        files::{file_extension, file_lines, get_filename, hash_file, read_file, read_text_file},
        metadata::{get_metadata, get_timestamps, glob_paths},
    },
    runtime::helper::{boolean_arg, number_arg, string_arg},
};
use boa_engine::{
    Context, JsError, JsResult, JsString, JsValue, js_string, object::builtins::JsUint8Array,
};
use common::files::Hashes;
use serde::Serialize;
use std::path::Path;

/// Return metadata about provided path or file
pub(crate) fn js_stat(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, 0)?;
    let timestamps = match get_timestamps(&path) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Could not get timestamp for {path}: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let meta = match get_metadata(&path) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Could not get metadata for {path}: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let mut info = JsFileInfo {
        filename: get_filename(&path),
        extension: file_extension(&path),
        directory: Path::new(&path)
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .display()
            .to_string(),
        full_path: path,
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
        is_symlink: meta.is_symlink(),
    };

    #[cfg(target_family = "unix")]
    {
        use std::os::unix::prelude::MetadataExt;
        info.inode = meta.ino();
        info.mode = meta.mode();
        info.uid = meta.uid();
        info.gid = meta.gid();
    }

    let data = serde_json::to_value(&info).unwrap_or_default();
    let value = JsValue::from_json(&data, context)?;

    Ok(value)
}

/// Return glob info based on provided glob string
pub(crate) fn js_glob(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, 0)?;

    let globs = match glob_paths(&path) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Could not glob for {path}: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let data = serde_json::to_value(&globs).unwrap_or_default();
    let value = JsValue::from_json(&data, context)?;

    Ok(value)
}

#[derive(Serialize, Debug)]
pub(crate) struct HashInfo {
    md5: String,
    sha1: String,
    sha256: String,
}

/// Hash a file from provided path based on hashing algorithms. If file is not provided empty values are returned
pub(crate) fn js_hash_file(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, 0)?;
    let hashes = Hashes {
        md5: boolean_arg(args, 1)?,
        sha1: boolean_arg(args, 2)?,
        sha256: boolean_arg(args, 3)?,
    };
    let (md5_value, sha1_value, sha256_value) = hash_file(&hashes, &path);
    let info = HashInfo {
        md5: md5_value,
        sha1: sha1_value,
        sha256: sha256_value,
    };
    let data = serde_json::to_value(&info).unwrap_or_default();
    let value = JsValue::from_json(&data, context)?;

    Ok(value)
}

/// Read a text file at provided path. Currently only files smaller than 2GB can be read
pub(crate) fn js_read_text_file(
    _this: &JsValue,
    args: &[JsValue],
    _context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, 0)?;
    let data = match read_text_file(&path) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Could not read text {path}: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    Ok(JsValue::new::<JsString>(data.into()))
}

pub(crate) fn js_read_lines(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, 0)?;
    let offset = number_arg(args, 1)? as usize;
    let limit = number_arg(args, 2)? as u64;

    let reader = match file_lines(&path) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Could not read text {path} lines: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let mut lines = Vec::new();
    let mut count = 0;
    let mut start_reader = reader.skip(offset);
    while let Some(Ok(line)) = start_reader.next() {
        if count == limit {
            break;
        }
        lines.push(line);
        count += 1;
    }

    let data = serde_json::to_value(&lines).unwrap_or_default();
    let value = JsValue::from_json(&data, context)?;

    Ok(value)
}

/// Read a file at provided path. Currently only files smaller than 2GB can be read
pub(crate) fn js_read_file(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, 0)?;
    let data = match read_file(&path) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Could not read {path}: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let value = JsUint8Array::from_iter(data, context)?;

    Ok(value.into())
}

#[cfg(test)]
mod tests {
    use crate::runtime::run::execute_script;
    use crate::structs::artifacts::runtime::script::JSScript;
    use crate::structs::toml::Output;

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("json"),
            compress,
            timeline: false,
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
    #[cfg(target_os = "macos")]
    fn test_js_stat_mac() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL2ZpbGVzeXN0ZW0vZmlsZXMudHMKZnVuY3Rpb24gc3RhdChwYXRoKSB7CiAgY29uc3QgZGF0YSA9IGpzX3N0YXQocGF0aCk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCB0YXJnZXQgPSAiL1VzZXJzIjsKICBjb25zdCBkYXRhID0gc3RhdCh0YXJnZXQpOwogIHJldHVybiBkYXRhOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("stat_path"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_js_stat_windows() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL2ZpbGVzeXN0ZW0vZmlsZXMudHMKZnVuY3Rpb24gc3RhdChwYXRoKSB7CiAgY29uc3QgZGF0YSA9IGpzX3N0YXQocGF0aCk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCB0YXJnZXQgPSAiQzpcXFVzZXJzIjsKICBjb25zdCBkYXRhID0gc3RhdCh0YXJnZXQpOwogIHJldHVybiBkYXRhOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("stat_path"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_js_stat_linux() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL2ZpbGVzeXN0ZW0vZmlsZXMudHMKZnVuY3Rpb24gc3RhdChwYXRoKSB7CiAgY29uc3QgZGF0YSA9IGpzX3N0YXQocGF0aCk7CiAgY29uc29sZS5sb2coSlNPTi5zdHJpbmdpZnkoZGF0YSkpOwogIHJldHVybiBkYXRhOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgdGFyZ2V0ID0gIi9ldGMiOwogIGNvbnN0IGRhdGEgPSBzdGF0KHRhcmdldCk7CiAgcmV0dXJuIGRhdGE7Cn0KbWFpbigpOw==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("stat_path"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn test_js_hash_file() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9kaXJlY3RvcnkudHMKYXN5bmMgZnVuY3Rpb24gcmVhZERpcihwYXRoKSB7CiAgY29uc3QgZGF0YSA9IGF3YWl0IGpzX3JlYWRfZGlyKHBhdGgpOwogIHJldHVybiBkYXRhOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9maWxlc3lzdGVtL2ZpbGVzLnRzCmZ1bmN0aW9uIGhhc2gocGF0aCwgbWQ1LCBzaGExLCBzaGEyNTYpIHsKICBjb25zdCBkYXRhID0ganNfaGFzaF9maWxlKHBhdGgsIG1kNSwgc2hhMSwgc2hhMjU2KTsKICByZXR1cm4gZGF0YTsKfQoKLy8gbWFpbi50cwphc3luYyBmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHN0YXJ0ID0gIi9iaW4iOwogIGNvbnN0IGZpbGVzID0gYXdhaXQgcmVhZERpcihzdGFydCk7CiAgZm9yIChjb25zdCBlbnRyeSBvZiBmaWxlcykgewogICAgaWYgKCFlbnRyeS5pc19maWxlKSB7CiAgICAgIGNvbnRpbnVlOwogICAgfQogICAgY29uc3QgaGFzaGVzID0gaGFzaChlbnRyeS5mdWxsX3BhdGgsIHRydWUsIGZhbHNlLCBmYWxzZSk7CiAgICByZXR1cm4gaGFzaGVzOwogIH0KfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("hash_files"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_js_hash_file() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9kaXJlY3RvcnkudHMKYXN5bmMgZnVuY3Rpb24gcmVhZERpcihwYXRoKSB7CiAgY29uc3QgZGF0YSA9IGF3YWl0IGpzX3JlYWRfZGlyKHBhdGgpOwogIHJldHVybiBkYXRhOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9maWxlc3lzdGVtL2ZpbGVzLnRzCmZ1bmN0aW9uIGhhc2gocGF0aCwgbWQ1LCBzaGExLCBzaGEyNTYpIHsKICBjb25zdCBkYXRhID0ganNfaGFzaF9maWxlKHBhdGgsIG1kNSwgc2hhMSwgc2hhMjU2KTsKICByZXR1cm4gZGF0YTsKfQoKLy8gbWFpbi50cwphc3luYyBmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHN0YXJ0ID0gIkM6XFwiOwogIGNvbnN0IGZpbGVzID0gYXdhaXQgcmVhZERpcihzdGFydCk7CiAgZm9yIChjb25zdCBlbnRyeSBvZiBmaWxlcykgewogICAgaWYgKCFlbnRyeS5pc19maWxlKSB7CiAgICAgIGNvbnRpbnVlOwogICAgfQogICAgY29uc3QgaGFzaGVzID0gaGFzaChlbnRyeS5mdWxsX3BhdGgsIHRydWUsIGZhbHNlLCBmYWxzZSk7CiAgICByZXR1cm4gaGFzaGVzOwogIH0KfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("hash_files"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn test_js_read_text_file() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9maWxlcy50cwpmdW5jdGlvbiByZWFkVGV4dEZpbGUocGF0aCkgewogIGNvbnN0IGRhdGEgPSBqc19yZWFkX3RleHRfZmlsZShwYXRoKTsKICByZXR1cm4gZGF0YTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHBhdGggPSAiL2V0Yy9yZXNvbHYuY29uZiI7CiAgY29uc3QgZGF0YSA9IHJlYWRUZXh0RmlsZShwYXRoKTsKICByZXR1cm4gZGF0YTsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("read_text"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn test_js_read_file() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9maWxlcy50cwpmdW5jdGlvbiByZWFkRmlsZShwYXRoKSB7CiAgY29uc3QgZGF0YSA9IGpzX3JlYWRfZmlsZShwYXRoKTsKICByZXR1cm4gZGF0YTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHBhdGggPSAiL2V0Yy9yZXNvbHYuY29uZiI7CiAgY29uc3QgZGF0YSA9IHJlYWRGaWxlKHBhdGgpOwogIHJldHVybiBBcnJheS5mcm9tKGRhdGEpOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("read_file"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_js_read_text_file() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9maWxlcy50cwpmdW5jdGlvbiByZWFkVGV4dEZpbGUocGF0aCkgewogIGNvbnN0IGRhdGEgPSBqc19yZWFkX3RleHRfZmlsZShwYXRoKTsKICByZXR1cm4gZGF0YTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHBhdGggPSAiQzpcXFdpbmRvd3NcXHdpbi5pbmkiOwogIGNvbnN0IGRhdGEgPSByZWFkVGV4dEZpbGUocGF0aCk7CiAgcmV0dXJuIGRhdGE7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("read_text"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_js_read_file() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9maWxlcy50cwpmdW5jdGlvbiByZWFkRmlsZShwYXRoKSB7CiAgY29uc3QgZGF0YSA9IGpzX3JlYWRfZmlsZShwYXRoKTsKICByZXR1cm4gZGF0YTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHBhdGggPSAiQzpcXFdpbmRvd3NcXHdpbi5pbmkiOwogIGNvbnN0IGRhdGEgPSByZWFkRmlsZShwYXRoKTsKICByZXR1cm4gQXJyYXkuZnJvbShkYXRhKTsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("read_text"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_js_glob() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21haW4vc3JjL2ZpbGVzeXN0ZW0vZmlsZXMudHMKZnVuY3Rpb24gZ2xvYihwYXR0ZXJuKSB7CiAgY29uc3QgZGF0YSA9IGpzX2dsb2IocGF0dGVybik7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBwYXRocyA9IGdsb2IoIkM6XFwqIik7CiAgaWYgKHBhdGhzIGluc3RhbmNlb2YgRXJyb3IpIHsKICAgIGNvbnNvbGUuZXJyb3IoYEZhaWxlZCB0byBnbG9iIHBhdGg6ICR7cGF0aHN9YCk7CiAgfQogIHJldHVybiBwYXRoczsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("glob"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn test_js_glob() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21haW4vc3JjL2ZpbGVzeXN0ZW0vZmlsZXMudHMKZnVuY3Rpb24gZ2xvYihwYXR0ZXJuKSB7CiAgY29uc3QgZGF0YSA9IGpzX2dsb2IocGF0dGVybik7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBwYXRocyA9IGdsb2IoIi8qIik7CiAgaWYgKHBhdGhzIGluc3RhbmNlb2YgRXJyb3IpIHsKICAgIGNvbnNvbGUuZXJyb3IoYEZhaWxlZCB0byBnbG9iIHBhdGg6ICR7cGF0aHN9YCk7CiAgfQogIHJldHVybiBwYXRoczsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("glob"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_read_lines() {
        let test = "KCgpID0+IHsKICAvLyAuLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvdXRpbHMvZXJyb3IudHMKICB2YXIgRXJyb3JCYXNlID0gY2xhc3MgZXh0ZW5kcyBFcnJvciB7CiAgICBuYW1lOwogICAgbWVzc2FnZTsKICAgIGNvbnN0cnVjdG9yKG5hbWUsIG1lc3NhZ2UpIHsKICAgICAgc3VwZXIoKTsKICAgICAgdGhpcy5uYW1lID0gbmFtZTsKICAgICAgdGhpcy5tZXNzYWdlID0gbWVzc2FnZTsKICAgIH0KICB9OwoKICAvLyAuLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvZmlsZXN5c3RlbS9lcnJvcnMudHMKICB2YXIgRmlsZUVycm9yID0gY2xhc3MgZXh0ZW5kcyBFcnJvckJhc2UgewogIH07CgogIC8vIC4uL1Byb2plY3RzL2FydGVtaXMtYXBpL3NyYy9maWxlc3lzdGVtL2ZpbGVzLnRzCiAgZnVuY3Rpb24gcmVhZExpbmVzKHBhdGgsIG9mZnNldCA9IDAsIGxpbWl0ID0gMTAwKSB7CiAgICBpZiAob2Zmc2V0IDwgMCB8fCBsaW1pdCA8IDApIHsKICAgICAgcmV0dXJuIG5ldyBGaWxlRXJyb3IoIlJFQURfTElORVMiLCBgbmVpdGhlciBvZmZzZXQgKCR7b2Zmc2V0fSkgb3IgbGltaXQgKCR7bGltaXR9KSBjYW4gYmUgbGVzcyB0aGFuIDBgKTsKICAgIH0KICAgIHRyeSB7CiAgICAgIGNvbnN0IHJlc3VsdCA9IGpzX3JlYWRfbGluZXMocGF0aCwgb2Zmc2V0LCBsaW1pdCk7CiAgICAgIHJldHVybiByZXN1bHQ7CiAgICB9IGNhdGNoIChlcnIpIHsKICAgICAgcmV0dXJuIG5ldyBGaWxlRXJyb3IoIlJFQURfTElORVMiLCBgZmFpbGVkIHRvIHJlYWQgbGluZXMgZm9yICR7cGF0aH06ICR7ZXJyfWApOwogICAgfQogIH0KCiAgLy8gbWFpbi50cwogIGZ1bmN0aW9uIG1haW4oKSB7CiAgICBjb25zdCBwYXRoID0gIi9ldGMvcmVzb2x2LmNvbmYiOwogICAgY29uc3QgbGluZXMgPSByZWFkTGluZXMocGF0aCk7CiAgICBpZiAobGluZXMgaW5zdGFuY2VvZiBGaWxlRXJyb3IpIHsKICAgICAgcmV0dXJuOwogICAgfQogICAgY29uc29sZS5sb2coYHJlYWQ6ICR7bGluZXMubGVuZ3RofWApOwogIH0KICBtYWluKCk7Cn0pKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("read_lines"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
