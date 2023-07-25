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
    let results = serde_json::to_string(&macho)?;
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
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbWFjb3MvbWFjaG8udHMKZnVuY3Rpb24gZ2V0TWFjaG8ocGF0aCkgewogIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLmdldF9tYWNobyhwYXRoKTsKICBpZiAoZGF0YSA9PT0gIiIpIHsKICAgIHJldHVybiBudWxsOwogIH0KICBjb25zdCBtYWNobyA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIG1hY2hvOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9maWxlc3lzdGVtL2RpcmVjdG9yeS50cwphc3luYyBmdW5jdGlvbiByZWFkRGlyKHBhdGgpIHsKICBjb25zdCBkYXRhID0gSlNPTi5wYXJzZShhd2FpdCBmcy5yZWFkRGlyKHBhdGgpKTsKICByZXR1cm4gZGF0YTsKfQoKLy8gbWFpbi50cwphc3luYyBmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IGJpbl9wYXRoID0gIi9iaW4iOwogIGNvbnN0IG1hY2hvcyA9IFtdOwogIGZvciAoY29uc3QgZW50cnkgb2YgYXdhaXQgcmVhZERpcihiaW5fcGF0aCkpIHsKICAgIGlmICghZW50cnkuaXNfZmlsZSkgewogICAgICBjb250aW51ZTsKICAgIH0KICAgIGNvbnN0IG1hY2hvX3BhdGggPSBgJHtiaW5fcGF0aH0vJHtlbnRyeS5maWxlbmFtZX1gOwogICAgY29uc3QgaW5mbyA9IGdldE1hY2hvKG1hY2hvX3BhdGgpOwogICAgaWYgKGluZm8gPT09IG51bGwpIHsKICAgICAgY29udGludWU7CiAgICB9CiAgICBjb25zdCBtZXRhID0gewogICAgICBwYXRoOiBtYWNob19wYXRoLAogICAgICBtYWNobzogaW5mbwogICAgfTsKICAgIG1hY2hvcy5wdXNoKG1ldGEpOwogIH0KICByZXR1cm4gbWFjaG9zOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", true);

        let script = JSScript {
            name: String::from("bin_machos"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
