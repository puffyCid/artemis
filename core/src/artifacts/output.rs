use crate::{
    artifacts::error::CollectionError,
    output::formats::{csv::csv_format, json::json_format, jsonl::jsonl_format},
    runtime::deno::filter_script,
    structs::toml::Output,
};
use log::error;
use serde_json::Value;

/// Output forensic artifacts
pub(crate) fn output_artifact(
    serde_data: &mut Value,
    output_name: &str,
    output: &mut Output,
    start_time: &u64,
    filter: &bool,
) -> Result<(), CollectionError> {
    if *filter {
        if let Some(script) = &output.filter_script.clone() {
            let args = vec![serde_data.to_string(), output_name.to_string()];
            if let Some(name) = &output.filter_name.clone() {
                let filter_result = filter_script(output, &args, name, script);
                return match filter_result {
                    Ok(_) => Ok(()),
                    Err(err) => {
                        error!(
                            "[artemis-core] Could not apply filter script to windows data: {err:?}"
                        );
                        Err(CollectionError::FilterOutput)
                    }
                };
            }
            let filter_result = filter_script(output, &args, "UnknownFilterName", script);
            return match filter_result {
                Ok(_) => Ok(()),
                Err(err) => {
                    error!(
                        "[artemis-core] Could not apply unknown filter script to windows data: {err:?}"
                    );
                    Err(CollectionError::FilterOutput)
                }
            };
        }
    }

    let output_status = if output.format.to_lowercase() == "json" {
        json_format(serde_data, output_name, output, start_time)
    } else if output.format.to_lowercase() == "jsonl" {
        jsonl_format(serde_data, output_name, output, start_time)
    } else if output.format.to_lowercase() == "csv" {
        csv_format(serde_data, output_name, output)
    } else {
        error!(
            "[artemis-core] Unknown formatter provided: {}",
            output.format
        );
        return Err(CollectionError::Format);
    };
    match output_status {
        Ok(_) => {}
        Err(err) => {
            error!("[artemis-core] Could not output data: {err:?}");
            return Err(CollectionError::Output);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{artifacts::output::output_artifact, structs::toml::Output, utils::time};

    fn output_options(name: &str, format: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: format.to_string(),
            compress,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: String::from("local"),
            filter_name: None,
            filter_script: None,
            logging: None,
        }
    }

    #[test]
    fn test_output_artifact() {
        let mut output = output_options("output_test", "json", "./tmp", false);
        let start_time = time::time_now();

        let name = "test";
        let mut data = serde_json::Value::String(String::from("test"));
        let status = output_artifact(&mut data, name, &mut output, &start_time, &false).unwrap();
        assert_eq!(status, ());
    }
}
