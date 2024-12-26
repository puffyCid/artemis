use crate::artifacts::os::linux::executable::parser::parse_elf_file;
use deno_core::{error::AnyError, op2};

#[op2]
#[string]
/// Expose parsing elf file  to `Deno`
pub(crate) fn get_elf(#[string] path: String) -> Result<String, AnyError> {
    let elf_data = parse_elf_file(&path)?;
    let results = serde_json::to_string(&elf_data)?;
    Ok(results)
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
    fn test_get_elf() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbGludXgvZWxmLnRzCmZ1bmN0aW9uIGdldEVsZihwYXRoKSB7CiAgdHJ5IHsKICAgIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9lbGYocGF0aCk7CiAgICBjb25zdCBlbGYgPSBKU09OLnBhcnNlKGRhdGEpOwogICAgcmV0dXJuIGVsZjsKICB9IGNhdGNoIChlcnIpIHsKICAgIHJldHVybiBudWxsOwogIH0KfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9kaXJlY3RvcnkudHMKYXN5bmMgZnVuY3Rpb24gcmVhZERpcihwYXRoKSB7CiAgY29uc3QgZGF0YSA9IEpTT04ucGFyc2UoYXdhaXQgZnMucmVhZERpcihwYXRoKSk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIG1haW4udHMKYXN5bmMgZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBiaW5fcGF0aCA9ICIvYmluIjsKICBjb25zdCBlbGZzID0gW107CiAgZm9yIChjb25zdCBlbnRyeSBvZiBhd2FpdCByZWFkRGlyKGJpbl9wYXRoKSkgewogICAgaWYgKCFlbnRyeS5pc19maWxlKSB7CiAgICAgIGNvbnRpbnVlOwogICAgfQogICAgY29uc3QgZWxmX3BhdGggPSBgJHtiaW5fcGF0aH0vJHtlbnRyeS5maWxlbmFtZX1gOwogICAgY29uc3QgaW5mbyA9IGdldEVsZihlbGZfcGF0aCk7CiAgICBpZiAoaW5mbyA9PT0gbnVsbCkgewogICAgICBjb250aW51ZTsKICAgIH0KICAgIGNvbnN0IG1ldGEgPSB7CiAgICAgIHBhdGg6IGVsZl9wYXRoLAogICAgICBlbGY6IGluZm8sCiAgICB9OwogICAgZWxmcy5wdXNoKG1ldGEpOwogIH0KICByZXR1cm4gZWxmczsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);

        let script = JSScript {
            name: String::from("elf"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
