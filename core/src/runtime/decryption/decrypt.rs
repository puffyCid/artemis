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
        let test = "Ly8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3V0aWxzL2Vycm9yLnRzCnZhciBFcnJvckJhc2UgPSBjbGFzcyBleHRlbmRzIEVycm9yIHsKICBjb25zdHJ1Y3RvcihuYW1lLCBtZXNzYWdlKSB7CiAgICBzdXBlcigpOwogICAgdGhpcy5uYW1lID0gbmFtZTsKICAgIHRoaXMubWVzc2FnZSA9IG1lc3NhZ2U7CiAgfQp9OwoKLy8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL2RlY3J5cHRpb24vZXJyb3JzLnRzCnZhciBEZWNyeXB0RXJyb3IgPSBjbGFzcyBleHRlbmRzIEVycm9yQmFzZSB7Cn07CgovLyAuLi8uLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvZGVjcnlwdGlvbi9kZWNyeXB0LnRzCmZ1bmN0aW9uIGRlY3J5cHRfYWVzKGtleSwgaXYsIGRhdGEpIHsKICBjb25zdCBrZXlfbGVuZ3RoID0gMzI7CiAgaWYgKGtleS5sZW5ndGggIT0ga2V5X2xlbmd0aCkgewogICAgcmV0dXJuIG5ldyBEZWNyeXB0RXJyb3IoCiAgICAgIGBBRVNgLAogICAgICBgSW5jb3JyZWN0IGtleSBsZW5ndGgsIHdhbnRlZCAzMiBieXRlcyBnb3Q6ICR7a2V5Lmxlbmd0aH1gCiAgICApOwogIH0KICB0cnkgewogICAgY29uc3QgYnl0ZXMgPSBkZWNyeXB0aW9uLmRlY3J5cHRfYWVzKGtleSwgaXYsIGRhdGEpOwogICAgcmV0dXJuIGJ5dGVzOwogIH0gY2F0Y2ggKGVycikgewogICAgcmV0dXJuIG5ldyBEZWNyeXB0RXJyb3IoYEFFU2AsIGBmYWlsZWQgdG8gZGVjcnlwdDogJHtlcnJ9YCk7CiAgfQp9CgovLyAuLi8uLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvZW5jb2RpbmcvZXJyb3JzLnRzCnZhciBFbmNvZGluZ0Vycm9yID0gY2xhc3MgZXh0ZW5kcyBFcnJvckJhc2Ugewp9OwoKLy8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL2VuY29kaW5nL2Jhc2U2NC50cwpmdW5jdGlvbiBkZWNvZGUoYjY0KSB7CiAgdHJ5IHsKICAgIGNvbnN0IGJ5dGVzID0gZW5jb2RpbmcuYXRvYihiNjQpOwogICAgcmV0dXJuIGJ5dGVzOwogIH0gY2F0Y2ggKGVycikgewogICAgcmV0dXJuIG5ldyBFbmNvZGluZ0Vycm9yKGBCQVNFNjRgLCBgZmFpbGVkIHRvIGRlY29kZSAke2I2NH06ICR7ZXJyfWApOwogIH0KfQoKLy8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL2VuY29kaW5nL3N0cmluZ3MudHMKZnVuY3Rpb24gZXh0cmFjdFV0ZjhTdHJpbmcoZGF0YSkgewogIGNvbnN0IHJlc3VsdCA9IGVuY29kaW5nLmV4dHJhY3RfdXRmOF9zdHJpbmcoZGF0YSk7CiAgcmV0dXJuIHJlc3VsdDsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IGRhdGEgPSBkZWNvZGUoIklsRkE4cDk1b3EvRVk4dEh2UjR6ZkE9PSIpOwogIGNvbnN0IHZhbHVlID0gZGVjcnlwdF9hZXMoZGVjb2RlKCJPTmFMTDgwdzQxNmVEWGYxZCtnaTlXcHRPZ2tjSjdoREZQbTJ1b2orUlFZPSIpLCBkZWNvZGUoIk1EQXdNREF3TURBd01EQXdNREF3TUE9PSIpLCBkYXRhKTsKICBjb25zdCB0ZXh0ID0gZXh0cmFjdFV0ZjhTdHJpbmcodmFsdWUpOwogIGlmICh0ZXh0ICE9ICJoZWxsbyBBRVMiKSB7CiAgICB0aHJvdyAiYmFkIGRlY3J5cHRpb24hIjsKICB9CiAgY29uc29sZS5sb2coYEkgZGVjcnlwdGVkICR7dGV4dH1gKTsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("aes_decrypt"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
