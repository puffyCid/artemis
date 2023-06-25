use crate::artifacts::os::macos::macho::parser::MachoInfo;
use deno_core::{error::AnyError, op};
use log::error;

#[op]
/// Expose parsing macho file  to `Deno`
fn get_macho(path: String) -> Result<String, AnyError> {
    let macho_results = MachoInfo::parse_macho(&path);
    let macho = match macho_results {
        Ok(results) => results,
        Err(err) => {
            // Parsing macho files could fail for many reasons
            // Instead of cancelling the whole script, return empty result
            error!("[runtime] Failed to parse macho file: {err:?}");
            return Ok(String::new());
        }
    };
    let results = serde_json::to_string_pretty(&macho)?;
    Ok(results)
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
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        }
    }

    #[test]
    fn test_get_macho() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfbWFjaG8ocGF0aCkgewogICAgY29uc3QgZGF0YSA9IERlbm9bRGVuby5pbnRlcm5hbF0uY29yZS5vcHMuZ2V0X21hY2hvKHBhdGgpOwogICAgaWYgKGRhdGEgPT09ICIiKSB7CiAgICAgICAgcmV0dXJuIG51bGw7CiAgICB9CiAgICBjb25zdCBtYWNobyA9IEpTT04ucGFyc2UoZGF0YSk7CiAgICByZXR1cm4gbWFjaG87Cn0KZnVuY3Rpb24gZ2V0TWFjaG8ocGF0aCkgewogICAgcmV0dXJuIGdldF9tYWNobyhwYXRoKTsKfQpmdW5jdGlvbiBtYWluKCkgewogICAgY29uc3QgYmluX3BhdGggPSAiL2JpbiI7CiAgICBjb25zdCBtYWNob3MgPSBbXTsKICAgIGZvciAoY29uc3QgZW50cnkgb2YgRGVuby5yZWFkRGlyU3luYyhiaW5fcGF0aCkpewogICAgICAgIGlmICghZW50cnkuaXNGaWxlKSB7CiAgICAgICAgICAgIGNvbnRpbnVlOwogICAgICAgIH0KICAgICAgICBjb25zdCBtYWNob19wYXRoID0gYCR7YmluX3BhdGh9LyR7ZW50cnkubmFtZX1gOwogICAgICAgIGNvbnN0IGluZm8gPSBnZXRNYWNobyhtYWNob19wYXRoKTsKICAgICAgICBpZiAoaW5mbyA9PT0gbnVsbCkgewogICAgICAgICAgICBjb250aW51ZTsKICAgICAgICB9CiAgICAgICAgY29uc3QgbWV0YSA9IHsKICAgICAgICAgICAgcGF0aDogbWFjaG9fcGF0aCwKICAgICAgICAgICAgbWFjaG86IGluZm8KICAgICAgICB9OwogICAgICAgIG1hY2hvcy5wdXNoKG1ldGEpOwogICAgfQogICAgcmV0dXJuIG1hY2hvczsKfQptYWluKCk7Cgo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);

        let script = JSScript {
            name: String::from("bin_machos"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
