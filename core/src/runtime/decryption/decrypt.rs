use crate::utils::decryption::decrypt_aes::decrypt_aes_data;
use deno_core::{error::AnyError, op2, JsBuffer};

#[op2]
#[buffer]
/// Decrypt AES256
pub(crate) fn js_decrypt_aes(
    #[buffer] key: JsBuffer,
    #[buffer] iv: JsBuffer,
    #[buffer] mut data: JsBuffer,
) -> Result<Vec<u8>, AnyError> {
    let results = decrypt_aes_data(&key, &iv, &mut data)?;

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
    fn test_js_decrypt_aes() {
        let test = "";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("aes_decrypt"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
