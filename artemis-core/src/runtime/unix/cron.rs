use crate::{artifacts::os::unix::cron::crontab::Cron, runtime::error::RuntimeError};
use deno_core::{error::AnyError, op};
use log::error;

#[op]
/// Get `Cron` data
fn get_cron() -> Result<String, AnyError> {
    let history_results = Cron::parse_cron();
    let history = match history_results {
        Ok(results) => results,
        Err(err) => {
            error!("[runtime] Failed to get cron data: {err:?}");
            return Err(RuntimeError::ExecuteScript.into());
        }
    };
    let results = serde_json::to_string_pretty(&history)?;
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
    fn test_get_cron() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3VuaXgvY3Jvbi50cwpmdW5jdGlvbiBnZXRfY3JvbigpIHsKICBjb25zdCBkYXRhID0gRGVub1tEZW5vLmludGVybmFsXS5jb3JlLm9wcy5nZXRfY3JvbigpOwogIGNvbnN0IGhpc3RvcnkgPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiBoaXN0b3J5Owp9CgovLyAuLi8uLi9hcnRlbWlzLWFwaS9tb2QudHMKZnVuY3Rpb24gZ2V0Q3JvbigpIHsKICByZXR1cm4gZ2V0X2Nyb24oKTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IGRhdGEgPSBnZXRDcm9uKCk7CiAgcmV0dXJuIGRhdGE7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("cron_script"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
