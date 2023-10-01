use super::directory::JsFileInfo;
use crate::filesystem::{
    files::{file_extension, get_filename, hash_file, read_file, read_text_file, Hashes},
    metadata::{get_metadata, get_timestamps, glob_paths},
};
use deno_core::{error::AnyError, op, ToJsBuffer};
use serde::Serialize;
use std::path::Path;

#[op]
/// Return metadata about provided path or file
fn js_stat(path: String) -> Result<String, AnyError> {
    let timestamps = get_timestamps(&path)?;
    let meta = get_metadata(&path)?;

    let info = JsFileInfo {
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

    let data = serde_json::to_string(&info)?;

    Ok(data)
}

#[op]
/// Return glob info based on provided glob string
fn js_glob(glob: String) -> Result<String, AnyError> {
    let globs = glob_paths(&glob)?;
    let data = serde_json::to_string(&globs)?;

    Ok(data)
}

#[derive(Serialize, Debug)]
struct HashInfo {
    md5: String,
    sha1: String,
    sha256: String,
}

#[op]
/// Hash a file from provided path based on hashing algorithms. If file is not provided empty values are returned
fn js_hash_file(path: String, md5: bool, sha1: bool, sha256: bool) -> HashInfo {
    let hashes = Hashes { md5, sha1, sha256 };
    let (md5_value, sha1_value, sha256_value) = hash_file(&hashes, &path);
    HashInfo {
        md5: md5_value,
        sha1: sha1_value,
        sha256: sha256_value,
    }
}

#[op]
/// Read a text file at provided path. Currently only files smaller than 2GB can be read
fn js_read_text_file(path: String) -> Result<String, AnyError> {
    let data = read_text_file(&path)?;
    Ok(data)
}

#[op]
/// Read a file at provided path. Currently only files smaller than 2GB can be read
fn js_read_file(path: String) -> Result<ToJsBuffer, AnyError> {
    let data = read_file(&path)?;
    Ok(data.into())
}

#[cfg(test)]
mod tests {
    use crate::runtime::deno::execute_script;
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
    #[cfg(target_os = "macos")]
    fn test_js_stat_mac() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL2ZpbGVzeXN0ZW0vZmlsZXMudHMKZnVuY3Rpb24gc3RhdChwYXRoKSB7CiAgY29uc3QgZGF0YSA9IEpTT04ucGFyc2UoZnMuc3RhdChwYXRoKSk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCB0YXJnZXQgPSAiL1VzZXJzIjsKICBjb25zdCBkYXRhID0gc3RhdCh0YXJnZXQpOwogIHJldHVybiBkYXRhOwp9Cm1haW4oKTsK";
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
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL2ZpbGVzeXN0ZW0vZmlsZXMudHMKZnVuY3Rpb24gc3RhdChwYXRoKSB7CiAgY29uc3QgZGF0YSA9IEpTT04ucGFyc2UoZnMuc3RhdChwYXRoKSk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCB0YXJnZXQgPSAiQzpcXFVzZXJzIjsKICBjb25zdCBkYXRhID0gc3RhdCh0YXJnZXQpOwogIHJldHVybiBkYXRhOwp9Cm1haW4oKTsK";
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
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL2ZpbGVzeXN0ZW0vZmlsZXMudHMKZnVuY3Rpb24gc3RhdChwYXRoKSB7CiAgY29uc3QgZGF0YSA9IEpTT04ucGFyc2UoZnMuc3RhdChwYXRoKSk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCB0YXJnZXQgPSAiL2V0YyI7CiAgY29uc3QgZGF0YSA9IHN0YXQodGFyZ2V0KTsKICByZXR1cm4gZGF0YTsKfQptYWluKCk7Cg==";
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
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9kaXJlY3RvcnkudHMKYXN5bmMgZnVuY3Rpb24gcmVhZERpcihwYXRoKSB7CiAgY29uc3QgZGF0YSA9IEpTT04ucGFyc2UoYXdhaXQgZnMucmVhZERpcihwYXRoKSk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL2ZpbGVzeXN0ZW0vZmlsZXMudHMKZnVuY3Rpb24gaGFzaChwYXRoLCBtZDUsIHNoYTEsIHNoYTI1NikgewogIGNvbnN0IGRhdGEgPSBmcy5oYXNoKHBhdGgsIG1kNSwgc2hhMSwgc2hhMjU2KTsKICByZXR1cm4gZGF0YTsKfQoKLy8gbWFpbi50cwphc3luYyBmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHN0YXJ0ID0gIi8iOwogIGNvbnN0IGZpbGVzID0gYXdhaXQgcmVhZERpcihzdGFydCk7CiAgZm9yIChjb25zdCBlbnRyeSBvZiBmaWxlcykgewogICAgaWYgKCFlbnRyeS5pc19maWxlKSB7CiAgICAgIGNvbnRpbnVlOwogICAgfQogICAgY29uc3QgaGFzaGVzID0gaGFzaChlbnRyeS5mdWxsX3BhdGgsIHRydWUsIGZhbHNlLCBmYWxzZSk7CiAgICByZXR1cm4gaGFzaGVzOwogIH0KfQptYWluKCk7Cg==";
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
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9kaXJlY3RvcnkudHMKYXN5bmMgZnVuY3Rpb24gcmVhZERpcihwYXRoKSB7CiAgY29uc3QgZGF0YSA9IEpTT04ucGFyc2UoYXdhaXQgZnMucmVhZERpcihwYXRoKSk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL2ZpbGVzeXN0ZW0vZmlsZXMudHMKZnVuY3Rpb24gaGFzaChwYXRoLCBtZDUsIHNoYTEsIHNoYTI1NikgewogIGNvbnN0IGRhdGEgPSBmcy5oYXNoKHBhdGgsIG1kNSwgc2hhMSwgc2hhMjU2KTsKICByZXR1cm4gZGF0YTsKfQoKLy8gbWFpbi50cwphc3luYyBmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHN0YXJ0ID0gIkM6XFwiOwogIGNvbnN0IGZpbGVzID0gYXdhaXQgcmVhZERpcihzdGFydCk7CiAgZm9yIChjb25zdCBlbnRyeSBvZiBmaWxlcykgewogICAgaWYgKCFlbnRyeS5pc19maWxlKSB7CiAgICAgIGNvbnRpbnVlOwogICAgfQogICAgY29uc3QgaGFzaGVzID0gaGFzaChlbnRyeS5mdWxsX3BhdGgsIHRydWUsIGZhbHNlLCBmYWxzZSk7CiAgICByZXR1cm4gaGFzaGVzOwogIH0KfQptYWluKCk7Cg==";
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
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9maWxlcy50cwpmdW5jdGlvbiByZWFkVGV4dEZpbGUocGF0aCkgewogIGNvbnN0IGRhdGEgPSBmcy5yZWFkVGV4dEZpbGUocGF0aCk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBwYXRoID0gIi9ldGMvcmVzb2x2LmNvbmYiOwogIGNvbnN0IGRhdGEgPSByZWFkVGV4dEZpbGUocGF0aCk7CiAgcmV0dXJuIGRhdGE7Cn0KbWFpbigpOwo=";
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
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9maWxlcy50cwpmdW5jdGlvbiByZWFkRmlsZShwYXRoKSB7CiAgY29uc3QgZGF0YSA9IGZzLnJlYWRGaWxlKHBhdGgpOwogIHJldHVybiBkYXRhOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgcGF0aCA9ICIvZXRjL3Jlc29sdi5jb25mIjsKICBjb25zdCBkYXRhID0gcmVhZEZpbGUocGF0aCk7CiAgcmV0dXJuIEFycmF5LmZyb20oZGF0YSk7Cn0KbWFpbigpOwo=";
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
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9maWxlcy50cwpmdW5jdGlvbiByZWFkVGV4dEZpbGUocGF0aCkgewogIGNvbnN0IGRhdGEgPSBmcy5yZWFkVGV4dEZpbGUocGF0aCk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBwYXRoID0gIkM6XFxXaW5kb3dzXFx3aW4uaW5pIjsKICBjb25zdCBkYXRhID0gcmVhZFRleHRGaWxlKHBhdGgpOwogIHJldHVybiBkYXRhOwp9Cm1haW4oKTsK";
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
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9maWxlcy50cwpmdW5jdGlvbiByZWFkRmlsZShwYXRoKSB7CiAgY29uc3QgZGF0YSA9IGZzLnJlYWRGaWxlKHBhdGgpOwogIHJldHVybiBkYXRhOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgcGF0aCA9ICJDOlxcV2luZG93c1xcd2luLmluaSI7CiAgY29uc3QgZGF0YSA9IHJlYWRGaWxlKHBhdGgpOwogIHJldHVybiBBcnJheS5mcm9tKGRhdGEpOwp9Cm1haW4oKTsK";
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
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21haW4vc3JjL2ZpbGVzeXN0ZW0vZmlsZXMudHMKZnVuY3Rpb24gZ2xvYihwYXR0ZXJuKSB7CiAgY29uc3QgZGF0YSA9IGZzLmdsb2IocGF0dGVybik7CiAgY29uc3QgcmVzdWx0ID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gcmVzdWx0Owp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgcGF0aHMgPSBnbG9iKCJDOlxcKiIpOwogIGlmIChwYXRocyBpbnN0YW5jZW9mIEVycm9yKSB7CiAgICBjb25zb2xlLmVycm9yKGBGYWlsZWQgdG8gZ2xvYiBwYXRoOiAke3BhdGhzfWApOwogIH0KICByZXR1cm4gcGF0aHM7Cn0KbWFpbigpOwo=";
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
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21haW4vc3JjL2ZpbGVzeXN0ZW0vZmlsZXMudHMKZnVuY3Rpb24gZ2xvYihwYXR0ZXJuKSB7CiAgY29uc3QgZGF0YSA9IGZzLmdsb2IocGF0dGVybik7CiAgY29uc3QgcmVzdWx0ID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gcmVzdWx0Owp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgcGF0aHMgPSBnbG9iKCIvKiIpOwogIGlmIChwYXRocyBpbnN0YW5jZW9mIEVycm9yKSB7CiAgICBjb25zb2xlLmVycm9yKGBGYWlsZWQgdG8gZ2xvYiBwYXRoOiAke3BhdGhzfWApOwogIH0KICByZXR1cm4gcGF0aHM7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("glob"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
