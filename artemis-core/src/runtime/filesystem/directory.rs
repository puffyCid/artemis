use crate::{
    filesystem::{
        directory::is_directory,
        files::{file_extension, get_filename},
        metadata::{get_metadata, get_timestamps},
    },
    runtime::error::RuntimeError,
};
use deno_core::{error::AnyError, op};
use log::error;
use serde::Serialize;
use std::path::Path;
use tokio::fs::read_dir;

#[derive(Serialize, Debug)]
pub(crate) struct JsFileInfo {
    pub(crate) full_path: String,
    pub(crate) directory: String,
    pub(crate) filename: String,
    pub(crate) extension: String,
    pub(crate) created: i64,
    pub(crate) modified: i64,
    pub(crate) changed: i64,
    pub(crate) accessed: i64,
    pub(crate) size: u64,
    pub(crate) inode: u64,
    pub(crate) mode: u32,
    pub(crate) uid: u32,
    pub(crate) gid: u32,
    pub(crate) is_file: bool,
    pub(crate) is_directory: bool,
    pub(crate) is_symlink: bool,
}

#[op]
/// List all files and directories at provided directory path
async fn js_read_dir(path: String) -> Result<Vec<JsFileInfo>, AnyError> {
    if !is_directory(&path) {
        error!("[runtime] Path is not a directory");
        return Err(RuntimeError::ExecuteScript.into());
    }

    let mut dir = read_dir(&path).await?;

    let mut files: Vec<JsFileInfo> = Vec::new();
    while let Some(entry) = dir.next_entry().await? {
        let full_path = entry.path().display().to_string();
        let timestamps = get_timestamps(&full_path)?;
        let meta = get_metadata(&full_path)?;

        let mut info = JsFileInfo {
            filename: get_filename(&full_path),
            extension: file_extension(&full_path),
            full_path,
            directory: entry
                .path()
                .parent()
                .unwrap_or_else(|| &Path::new(""))
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
        files.push(info);
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use crate::runtime::deno::execute_script;
    use crate::structs::artifacts::runtime::script::JSScript;
    use crate::utils::artemis_toml::Output;

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
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL2ZpbGVzeXN0ZW0vZGlyZWN0b3J5LnRzCmZ1bmN0aW9uIHJlYWREaXIocGF0aCkgewogIGNvbnN0IGRhdGEgPSBmcy5yZWFkRGlyKHBhdGgpOwogIHJldHVybiBkYXRhOwp9CgovLyBtYWluLnRzCmFzeW5jIGZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3Qgc3RhcnQgPSAiLyI7CiAgY29uc3QgZmlsZXMgPSByZWFkRGlyKHN0YXJ0KTsKICBjb25zdCBkYXRhID0gW107CiAgZm9yIGF3YWl0IChjb25zdCBlbnRyeSBvZiBmaWxlcykgewogICAgaWYgKHR5cGVvZiBlbnRyeS5pbm9kZSA9PT0gImJpZ2ludCIpIHsKICAgICAgZW50cnkuaW5vZGUgPSBlbnRyeS5pbm9kZS50b1N0cmluZygpOwogICAgfQogICAgZGF0YS5wdXNoKGVudHJ5KTsKICB9CiAgcmV0dXJuIGRhdGE7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("root_list"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_read_dir_root() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL2ZpbGVzeXN0ZW0vZGlyZWN0b3J5LnRzCmZ1bmN0aW9uIHJlYWREaXIocGF0aCkgewogIGNvbnN0IGRhdGEgPSBmcy5yZWFkRGlyKHBhdGgpOwogIHJldHVybiBkYXRhOwp9CgovLyBtYWluLnRzCmFzeW5jIGZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3Qgc3RhcnQgPSAiQzpcXCI7CiAgY29uc3QgZmlsZXMgPSByZWFkRGlyKHN0YXJ0KTsKICBjb25zdCBkYXRhID0gW107CiAgZm9yIGF3YWl0IChjb25zdCBlbnRyeSBvZiBmaWxlcykgewogICAgaWYgKHR5cGVvZiBlbnRyeS5pbm9kZSA9PT0gImJpZ2ludCIpIHsKICAgICAgZW50cnkuaW5vZGUgPSBlbnRyeS5pbm9kZS50b1N0cmluZygpOwogICAgfQogICAgZGF0YS5wdXNoKGVudHJ5KTsKICB9CiAgcmV0dXJuIGRhdGE7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("root_list"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
