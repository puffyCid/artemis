use crate::{
    filesystem::ntfs::raw_files::{raw_read_file, read_attribute},
    runtime::error::RuntimeError,
    utils::encoding::base64_encode_standard,
};
use deno_core::{error::AnyError, op};
use log::error;

#[op]
/// Expose reading a raw file to `Deno`
fn read_raw_file(path: String) -> Result<String, AnyError> {
    let data_result = raw_read_file(&path);
    let data = match data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to read file {path}: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };
    // We return a base64 string to play nice with Deno/V8 limitations on large byte arrays
    Ok(base64_encode_standard(&data))
}

#[op]
/// Expose reading an alternative data stream (ADS) to `Deno`
fn read_ads_data(path: String, ads_name: String) -> Result<String, AnyError> {
    let data_result = read_attribute(&path, &ads_name);
    let data = match data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to ADS data at {path}: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };
    // We return a base64 string to play nice with Deno/V8 limitations on large byte arrays
    Ok(base64_encode_standard(&data))
}

#[cfg(test)]
mod tests {
    use crate::{
        runtime::deno::execute_script, structs::artifacts::runtime::script::JSScript,
        utils::artemis_toml::Output,
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
        }
    }

    #[test]
    fn test_read_ads_data_motw() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBkZWNvZGUoYjY0KSB7CiAgICBjb25zdCBiaW5TdHJpbmcgPSBhdG9iKGI2NCk7CiAgICBjb25zdCBzaXplID0gYmluU3RyaW5nLmxlbmd0aDsKICAgIGNvbnN0IGJ5dGVzID0gbmV3IFVpbnQ4QXJyYXkoc2l6ZSk7CiAgICBmb3IobGV0IGkgPSAwOyBpIDwgc2l6ZTsgaSsrKXsKICAgICAgICBieXRlc1tpXSA9IGJpblN0cmluZy5jaGFyQ29kZUF0KGkpOwogICAgfQogICAgcmV0dXJuIGJ5dGVzOwp9CmZ1bmN0aW9uIHJlYWRfYWRzX2RhdGEocGF0aCwgYWRzX25hbWUpIHsKICAgIGNvbnN0IGRhdGEgPSBEZW5vW0Rlbm8uaW50ZXJuYWxdLmNvcmUub3BzLnJlYWRfYWRzX2RhdGEocGF0aCwgYWRzX25hbWUpOwogICAgcmV0dXJuIGRlY29kZShkYXRhKTsKfQpmdW5jdGlvbiByZWFkQWRzRGF0YShwYXRoLCBhZHNfbmFtZSkgewogICAgcmV0dXJuIHJlYWRfYWRzX2RhdGEocGF0aCwgYWRzX25hbWUpOwp9CmZ1bmN0aW9uIG1haW4oKSB7CiAgICBjb25zdCBkcml2ZSA9IERlbm8uZW52LmdldCgiU3lzdGVtRHJpdmUiKTsKICAgIGlmIChkcml2ZSA9PT0gdW5kZWZpbmVkKSB7CiAgICAgICAgcmV0dXJuIFtdOwogICAgfQogICAgY29uc3Qgd2ViX2ZpbGVzID0gW107CiAgICBjb25zdCB1c2VycyA9IGAke2RyaXZlfVxcVXNlcnNgOwogICAgZm9yIChjb25zdCBlbnRyeSBvZiBEZW5vLnJlYWREaXJTeW5jKHVzZXJzKSl7CiAgICAgICAgdHJ5IHsKICAgICAgICAgICAgY29uc3QgcGF0aCA9IGAke3VzZXJzfVxcJHtlbnRyeS5uYW1lfVxcRG93bmxvYWRzYDsKICAgICAgICAgICAgZm9yIChjb25zdCBmaWxlX2VudHJ5IG9mIERlbm8ucmVhZERpclN5bmMocGF0aCkpewogICAgICAgICAgICAgICAgdHJ5IHsKICAgICAgICAgICAgICAgICAgICBpZiAoIWZpbGVfZW50cnkuaXNGaWxlKSB7CiAgICAgICAgICAgICAgICAgICAgICAgIGNvbnRpbnVlOwogICAgICAgICAgICAgICAgICAgIH0KICAgICAgICAgICAgICAgICAgICBjb25zdCBmdWxsX3BhdGggPSBgJHtwYXRofVxcJHtmaWxlX2VudHJ5Lm5hbWV9YDsKICAgICAgICAgICAgICAgICAgICBjb25zdCBhZHMgPSAiWm9uZS5JZGVudGlmaWVyIjsKICAgICAgICAgICAgICAgICAgICBjb25zdCBkYXRhID0gcmVhZEFkc0RhdGEoZnVsbF9wYXRoLCBhZHMpOwogICAgICAgICAgICAgICAgICAgIGlmIChkYXRhLmxlbmd0aCA9PT0gMCkgewogICAgICAgICAgICAgICAgICAgICAgICBjb250aW51ZTsKICAgICAgICAgICAgICAgICAgICB9CiAgICAgICAgICAgICAgICAgICAgY29uc3QgaW5mbyA9IERlbm8uc3RhdFN5bmMoZnVsbF9wYXRoKTsKICAgICAgICAgICAgICAgICAgICBpZiAoaW5mby5tdGltZSA9PT0gbnVsbCB8fCBpbmZvLmJpcnRodGltZSA9PT0gbnVsbCB8fCBpbmZvLmF0aW1lID09PSBudWxsKSB7CiAgICAgICAgICAgICAgICAgICAgICAgIGNvbnRpbnVlOwogICAgICAgICAgICAgICAgICAgIH0KICAgICAgICAgICAgICAgICAgICBjb25zdCBtYXJrX2luZm8gPSBuZXcgVGV4dERlY29kZXIoKS5kZWNvZGUoZGF0YSk7CiAgICAgICAgICAgICAgICAgICAgY29uc3Qgd2ViX2ZpbGUgPSB7CiAgICAgICAgICAgICAgICAgICAgICAgIG1hcms6IG1hcmtfaW5mbywKICAgICAgICAgICAgICAgICAgICAgICAgcGF0aDogZnVsbF9wYXRoLAogICAgICAgICAgICAgICAgICAgICAgICBjcmVhdGVkOiBpbmZvLmJpcnRodGltZSwKICAgICAgICAgICAgICAgICAgICAgICAgbW9kaWZpZWQ6IGluZm8ubXRpbWUsCiAgICAgICAgICAgICAgICAgICAgICAgIGFjY2Vzc2VkOiBpbmZvLmF0aW1lLAogICAgICAgICAgICAgICAgICAgICAgICBzaXplOiBpbmZvLnNpemUKICAgICAgICAgICAgICAgICAgICB9OwogICAgICAgICAgICAgICAgICAgIHdlYl9maWxlcy5wdXNoKHdlYl9maWxlKTsKICAgICAgICAgICAgICAgIH0gY2F0Y2ggKF9lcnJvcikgewogICAgICAgICAgICAgICAgICAgIGNvbnRpbnVlOwogICAgICAgICAgICAgICAgfQogICAgICAgICAgICB9CiAgICAgICAgfSBjYXRjaCAoX2Vycm9yKSB7CiAgICAgICAgICAgIGNvbnRpbnVlOwogICAgICAgIH0KICAgIH0KICAgIHJldHVybiB3ZWJfZmlsZXM7Cn0KbWFpbigpOwoK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("read_ads_motw"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_read_raw_file_swapfile() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBkZWNvZGUoYjY0KSB7CiAgICBjb25zdCBiaW5TdHJpbmcgPSBhdG9iKGI2NCk7CiAgICBjb25zdCBzaXplID0gYmluU3RyaW5nLmxlbmd0aDsKICAgIGNvbnN0IGJ5dGVzID0gbmV3IFVpbnQ4QXJyYXkoc2l6ZSk7CiAgICBmb3IobGV0IGkgPSAwOyBpIDwgc2l6ZTsgaSsrKXsKICAgICAgICBieXRlc1tpXSA9IGJpblN0cmluZy5jaGFyQ29kZUF0KGkpOwogICAgfQogICAgcmV0dXJuIGJ5dGVzOwp9CmZ1bmN0aW9uIHJlYWRfcmF3X2ZpbGUocGF0aCkgewogICAgY29uc3QgZGF0YSA9IERlbm9bRGVuby5pbnRlcm5hbF0uY29yZS5vcHMucmVhZF9yYXdfZmlsZShwYXRoKTsKICAgIHJldHVybiBkZWNvZGUoZGF0YSk7Cn0KZnVuY3Rpb24gcmVhZFJhd0ZpbGUocGF0aCkgewogICAgcmV0dXJuIHJlYWRfcmF3X2ZpbGUocGF0aCk7Cn0KZnVuY3Rpb24gbWFpbigpIHsKICAgIGNvbnN0IGRyaXZlID0gRGVuby5lbnYuZ2V0KCJTeXN0ZW1Ecml2ZSIpOwogICAgaWYgKGRyaXZlID09PSB1bmRlZmluZWQpIHsKICAgICAgICByZXR1cm4gMDsKICAgIH0KICAgIHRyeSB7CiAgICAgICAgY29uc3Qgc3dhcCA9IGAke2RyaXZlfVxcc3dhcGZpbGUuc3lzYDsKICAgICAgICBjb25zdCBpbmZvID0gRGVuby5zdGF0U3luYyhzd2FwKTsKICAgICAgICBpZiAoIWluZm8uaXNGaWxlKSB7CiAgICAgICAgICAgIHJldHVybiAwOwogICAgICAgIH0KICAgICAgICBpZiAoaW5mby5zaXplID4gMjE0NzQ4MzY0OCkgewogICAgICAgICAgICByZXR1cm4gMDsKICAgICAgICB9CiAgICAgICAgY29uc3QgZGF0YSA9IHJlYWRSYXdGaWxlKHN3YXApOwogICAgICAgIHJldHVybiBkYXRhLmxlbmd0aDsKICAgIH0gY2F0Y2ggKF9lcnJvcikgewogICAgICAgIHJldHVybiAwOwogICAgfQp9Cm1haW4oKTsKCg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("swapfile"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
