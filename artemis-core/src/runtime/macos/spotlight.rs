use crate::artifacts::os::macos::spotlight::light::{
    parse_spotlight_reader, setup_spotlight_reader, StoreMeta,
};
use deno_core::{error::AnyError, op2};
use log::error;

#[op2]
#[string]
/// Expose parsing Spotlight to `Deno`
pub(crate) fn get_spotlight(
    #[string] store_file: String,
    #[string] meta: String,
    #[bigint] offset: usize,
) -> Result<String, AnyError> {
    let serde_result = serde_json::from_str(&meta);
    let store_meta: StoreMeta = match serde_result {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed deserialize store metadata: {err:?}");
            return Err(err.into());
        }
    };

    let entries = parse_spotlight_reader(
        &store_file,
        &store_meta.meta,
        &store_meta.blocks,
        &(offset as u32),
    )?;

    let results = serde_json::to_string(&entries)?;
    Ok(results)
}

#[op2]
#[string]
/// Expose setting up Spotlight parser to `Deno`
pub(crate) fn setup_spotlight_parser(#[string] glob_path: String) -> Result<String, AnyError> {
    let meta = setup_spotlight_reader(&glob_path)?;

    let results = serde_json::to_string(&meta)?;
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
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        }
    }

    #[test]
    fn test_get_spotlight() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21haW4vc3JjL3V0aWxzL2Vycm9yLnRzCnZhciBFcnJvckJhc2UgPSBjbGFzcyBleHRlbmRzIEVycm9yIHsKICBjb25zdHJ1Y3RvcihuYW1lLCBtZXNzYWdlKSB7CiAgICBzdXBlcigpOwogICAgdGhpcy5uYW1lID0gbmFtZTsKICAgIHRoaXMubWVzc2FnZSA9IG1lc3NhZ2U7CiAgfQp9OwoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21haW4vc3JjL2ZpbGVzeXN0ZW0vZXJyb3JzLnRzCnZhciBGaWxlRXJyb3IgPSBjbGFzcyBleHRlbmRzIEVycm9yQmFzZSB7Cn07CgovLyAuLi8uLi9Qcm9qZWN0cy9EZW5vL2FydGVtaXMtYXBpL3NyYy91dGlscy9lcnJvci50cwp2YXIgRXJyb3JCYXNlMiA9IGNsYXNzIGV4dGVuZHMgRXJyb3IgewogIGNvbnN0cnVjdG9yKG5hbWUsIG1lc3NhZ2UpIHsKICAgIHN1cGVyKCk7CiAgICB0aGlzLm5hbWUgPSBuYW1lOwogICAgdGhpcy5tZXNzYWdlID0gbWVzc2FnZTsKICB9Cn07CgovLyAuLi8uLi9Qcm9qZWN0cy9EZW5vL2FydGVtaXMtYXBpL3NyYy9tYWNvcy9lcnJvcnMudHMKdmFyIE1hY29zRXJyb3IgPSBjbGFzcyBleHRlbmRzIEVycm9yQmFzZTIgewp9OwoKLy8gLi4vLi4vUHJvamVjdHMvRGVuby9hcnRlbWlzLWFwaS9zcmMvbWFjb3Mvc3BvdGxpZ2h0LnRzCmZ1bmN0aW9uIHNldHVwX3Nwb3RsaWdodF9wYXJzZXIoZ2xvYl9wYXRoKSB7CiAgdHJ5IHsKICAgIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLnNldHVwX3Nwb3RsaWdodF9wYXJzZXIoZ2xvYl9wYXRoKTsKICAgIGNvbnN0IG1ldGEgPSBKU09OLnBhcnNlKGRhdGEpOwogICAgcmV0dXJuIG1ldGE7CiAgfSBjYXRjaCAoZXJyKSB7CiAgICByZXR1cm4gbmV3IE1hY29zRXJyb3IoCiAgICAgICJTUE9UTElHSFQiLAogICAgICBgZmFpbGVkIHRvIHNldHVwIHNwb3RsaWdodCBwYXJzZXIgZm9yICR7Z2xvYl9wYXRofTogJHtlcnJ9YAogICAgKTsKICB9Cn0KZnVuY3Rpb24gZ2V0X3Nwb3RsaWdodChtZXRhLCBzdG9yZV9maWxlLCBvZmZzZXQpIHsKICB0cnkgewogICAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X3Nwb3RsaWdodChzdG9yZV9maWxlLCBKU09OLnN0cmluZ2lmeShtZXRhKSwgb2Zmc2V0KTsKICAgIGNvbnN0IGVudHJpZXMgPSBKU09OLnBhcnNlKGRhdGEpOwogICAgcmV0dXJuIGVudHJpZXM7CiAgfSBjYXRjaCAoZXJyKSB7CiAgICByZXR1cm4gbmV3IE1hY29zRXJyb3IoCiAgICAgICJTUE9UTElHSFQiLAogICAgICBgZmFpbGVkIHRvIGdldCBzcG90bGlnaHQgZW50cmllcyBmb3IgJHtzdG9yZV9maWxlfTogJHtlcnJ9YAogICAgKTsKICB9Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYWluL3NyYy9maWxlc3lzdGVtL2ZpbGVzLnRzCmZ1bmN0aW9uIGdsb2IocGF0dGVybikgewogIHRyeSB7CiAgICBjb25zdCByZXN1bHQgPSBmcy5nbG9iKHBhdHRlcm4pOwogICAgY29uc3QgZGF0YSA9IEpTT04ucGFyc2UocmVzdWx0KTsKICAgIHJldHVybiBkYXRhOwogIH0gY2F0Y2ggKGVycikgewogICAgcmV0dXJuIG5ldyBGaWxlRXJyb3IoIkdMT0IiLCBgZmFpbGVkIHRvIGdsb2IgcGF0dGVybiAke3BhdHRlcm59IiAke2Vycn1gKTsKICB9Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBwYXRoID0gIi9Vc2Vycy8qL0xpYnJhcnkvQ2FjaGVzL2NvbS5hcHBsZS5oZWxwZC9pbmRleC5zcG90bGlnaHRWMy8qIjsKICBjb25zdCBtZXRhID0gc2V0dXBfc3BvdGxpZ2h0X3BhcnNlcihwYXRoKTsKICBpZiAobWV0YSBpbnN0YW5jZW9mIE1hY29zRXJyb3IpIHsKICAgIGNvbnNvbGUuZXJyb3IoYENvdWxkIG5vdCBzZXR1cCBzcG90bGlnaHQgcGFyc2VyOiAke21ldGF9YCk7CiAgICByZXR1cm4gW107CiAgfQogIGNvbnN0IHN0b3JlX3BhdGggPSBnbG9iKCIvVXNlcnMvKi9MaWJyYXJ5L0NhY2hlcy9jb20uYXBwbGUuaGVscGQvaW5kZXguc3BvdGxpZ2h0VjMvc3RvcmUuZGIiKTsKICBpZiAoc3RvcmVfcGF0aCBpbnN0YW5jZW9mIEZpbGVFcnJvcikgewogICAgY29uc29sZS5lcnJvcihgQ291bGQgZ2xvYiBzcG90bGlnaHQgc3RvcmUuZGI6ICR7c3RvcmVfcGF0aH1gKTsKICAgIHJldHVybiBbXTsKICB9CiAgZm9yIChjb25zdCBwYXRoMiBvZiBzdG9yZV9wYXRoKSB7CiAgICBjb25zdCByZXN1bHRzID0gZ2V0X3Nwb3RsaWdodChtZXRhLCBwYXRoMi5mdWxsX3BhdGgsIDApOwogICAgaWYgKHJlc3VsdHMgaW5zdGFuY2VvZiBNYWNvc0Vycm9yKSB7CiAgICAgIGNvbnNvbGUuZXJyb3IoYENvdWxkIG5vdCBwYXJzZSBzcG90bGlnaHQgZGIgJHtwYXRoMi5mdWxsX3BhdGh9OiAke3Jlc3VsdHN9YCk7CiAgICAgIHJldHVybiBbXTsKICAgIH0KICAgIGNvbnNvbGUubG9nKHJlc3VsdHNbMF0pOwogICAgcmV0dXJuIHJlc3VsdHM7CiAgfQp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);

        let script = JSScript {
            name: String::from("spotlight_script"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_setup_spotlight_parser() {
        let test = "Ly8gLi4vLi4vUHJvamVjdHMvRGVuby9hcnRlbWlzLWFwaS9zcmMvdXRpbHMvZXJyb3IudHMKdmFyIEVycm9yQmFzZSA9IGNsYXNzIGV4dGVuZHMgRXJyb3IgewogIGNvbnN0cnVjdG9yKG5hbWUsIG1lc3NhZ2UpIHsKICAgIHN1cGVyKCk7CiAgICB0aGlzLm5hbWUgPSBuYW1lOwogICAgdGhpcy5tZXNzYWdlID0gbWVzc2FnZTsKICB9Cn07CgovLyAuLi8uLi9Qcm9qZWN0cy9EZW5vL2FydGVtaXMtYXBpL3NyYy9tYWNvcy9lcnJvcnMudHMKdmFyIE1hY29zRXJyb3IgPSBjbGFzcyBleHRlbmRzIEVycm9yQmFzZSB7Cn07CgovLyAuLi8uLi9Qcm9qZWN0cy9EZW5vL2FydGVtaXMtYXBpL3NyYy9tYWNvcy9zcG90bGlnaHQudHMKZnVuY3Rpb24gc2V0dXBfc3BvdGxpZ2h0X3BhcnNlcihnbG9iX3BhdGgpIHsKICB0cnkgewogICAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuc2V0dXBfc3BvdGxpZ2h0X3BhcnNlcihnbG9iX3BhdGgpOwogICAgY29uc3QgbWV0YSA9IEpTT04ucGFyc2UoZGF0YSk7CiAgICByZXR1cm4gbWV0YTsKICB9IGNhdGNoIChlcnIpIHsKICAgIHJldHVybiBuZXcgTWFjb3NFcnJvcigKICAgICAgIlNQT1RMSUdIVCIsCiAgICAgIGBmYWlsZWQgdG8gc2V0dXAgc3BvdGxpZ2h0IHBhcnNlciBmb3IgJHtnbG9iX3BhdGh9OiAke2Vycn1gCiAgICApOwogIH0KfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHBhdGggPSAiL1VzZXJzLyovTGlicmFyeS9DYWNoZXMvY29tLmFwcGxlLmhlbHBkL2luZGV4LnNwb3RsaWdodFYzLyoiOwogIGNvbnN0IG1ldGEgPSBzZXR1cF9zcG90bGlnaHRfcGFyc2VyKHBhdGgpOwogIGlmIChtZXRhIGluc3RhbmNlb2YgTWFjb3NFcnJvcikgewogICAgY29uc29sZS5lcnJvcihgQ291bGQgbm90IHNldHVwIHNwb3RsaWdodCBwYXJzZXI6ICR7bWV0YX1gKTsKICAgIHJldHVybiBbXTsKICB9Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);

        let script = JSScript {
            name: String::from("spotlight_setup_script"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
