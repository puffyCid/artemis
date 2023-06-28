use crate::artifacts::os::linux::executable::parser::parse_elf_file;
use deno_core::{error::AnyError, op};

#[op]
/// Expose parsing elf file  to `Deno`
fn get_elf(path: String) -> Result<String, AnyError> {
    let elf_results = parse_elf_file(&path);
    let elf_data = match elf_results {
        Ok(results) => results,
        Err(_err) => {
            // Parsing elf files could fail for many reasons
            // Instead of cancelling the whole script, return empty result
            return Ok(String::new());
        }
    };
    let results = serde_json::to_string_pretty(&elf_data)?;
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
            filter_name: None,
            filter_script: None,
            logging: None,
        }
    }

    #[test]
    fn test_get_elf() {
        let test = "";
        let mut output = output_options("runtime_test", "local", "./tmp", false);

        let script = JSScript {
            name: String::from("elf"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
