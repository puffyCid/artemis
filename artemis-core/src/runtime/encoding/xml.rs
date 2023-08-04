use crate::utils::encoding::read_xml;
use deno_core::{error::AnyError, op};
use quick_xml::de::from_str;
use serde_json::Value;

#[op]
/// Read XML file into a JSON object
fn js_read_xml(path: String) -> Result<String, AnyError> {
    // read_xml supports UTF16 and UTF8 encodings
    let xml = read_xml(&path)?;

    // Parse XML string into generic serde Value
    let xml_json: Value = from_str(&xml)?;

    let json = serde_json::to_string(&xml_json)?;
    Ok(json)
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
            logging: None,
        }
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_js_read_xml() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21haW4vc3JjL2ZpbGVzeXN0ZW0vZmlsZXMudHMKZnVuY3Rpb24gZ2xvYihwYXR0ZXJuKSB7CiAgY29uc3QgZGF0YSA9IGZzLmdsb2IocGF0dGVybik7CiAgY29uc3QgcmVzdWx0ID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gcmVzdWx0Owp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFpbi9zcmMvZW5jb2RpbmcveG1sLnRzCmZ1bmN0aW9uIHJlYWRYbWwocGF0aCkgewogIGNvbnN0IHJlc3VsdCA9IGVuY29kaW5nLnJlYWRfeG1sKHBhdGgpOwogIGNvbnN0IHZhbHVlID0gSlNPTi5wYXJzZShyZXN1bHQpOwogIHJldHVybiB2YWx1ZTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHBhdGhzID0gZ2xvYigiQzpcXCpcXCoueG1sIik7CiAgaWYgKHBhdGhzIGluc3RhbmNlb2YgRXJyb3IpIHsKICAgIGNvbnNvbGUuZXJyb3IoYEZhaWxlZCB0byBnbG9iIHBhdGg6ICR7cGF0aHN9YCk7CiAgICByZXR1cm4gcGF0aHM7CiAgfQogIGZvciAoY29uc3QgZW50cnkgb2YgcGF0aHMpIHsKICAgIGlmICghZW50cnkuaXNfZmlsZSkgewogICAgICBjb250aW51ZTsKICAgIH0KICAgIGNvbnN0IHJldXNsdCA9IHJlYWRYbWwoZW50cnkuZnVsbF9wYXRoKTsKICAgIHJldHVybiByZXVzbHQ7CiAgfQp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("xml_test"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn test_js_read_xml() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21haW4vc3JjL2ZpbGVzeXN0ZW0vZmlsZXMudHMKZnVuY3Rpb24gZ2xvYihwYXR0ZXJuKSB7CiAgY29uc3QgZGF0YSA9IGZzLmdsb2IocGF0dGVybik7CiAgY29uc3QgcmVzdWx0ID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gcmVzdWx0Owp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFpbi9zcmMvZW5jb2RpbmcveG1sLnRzCmZ1bmN0aW9uIHJlYWRYbWwocGF0aCkgewogIGNvbnN0IHJlc3VsdCA9IGVuY29kaW5nLnJlYWRfeG1sKHBhdGgpOwogIGNvbnN0IHZhbHVlID0gSlNPTi5wYXJzZShyZXN1bHQpOwogIHJldHVybiB2YWx1ZTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHBhdGhzID0gZ2xvYigiLyovKi54bWwiKTsKICBpZiAocGF0aHMgaW5zdGFuY2VvZiBFcnJvcikgewogICAgY29uc29sZS5lcnJvcihgRmFpbGVkIHRvIGdsb2IgcGF0aDogJHtwYXRoc31gKTsKICAgIHJldHVybiBwYXRoczsKICB9CiAgZm9yIChjb25zdCBlbnRyeSBvZiBwYXRocykgewogICAgaWYgKCFlbnRyeS5pc19maWxlKSB7CiAgICAgIGNvbnRpbnVlOwogICAgfQogICAgY29uc3QgcmV1c2x0ID0gcmVhZFhtbChlbnRyeS5mdWxsX3BhdGgpOwogICAgcmV0dXJuIHJldXNsdDsKICB9Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("xml_test"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
