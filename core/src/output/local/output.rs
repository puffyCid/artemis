use super::error::LocalError;
use crate::structs::toml::Output;
use log::error;
use std::{
    fs::{OpenOptions, create_dir_all},
    io::Write,
};

/// Output to local directory provided by TOML input
pub(crate) fn local_output(
    data: &[u8],
    output: &Output,
    output_name: &str,
    extension: &str,
) -> Result<(), LocalError> {
    let output_path = format!("{}/{}", output.directory, output.name);

    let result = create_dir_all(&output_path);
    match result {
        Ok(_) => {}
        Err(err) => {
            error!(
                "[artemis-core] Failed to create output directory for {output_path}. Error: {err:?}"
            );
            return Err(LocalError::CreateDirectory);
        }
    }

    let file_extension = if output.compress {
        format!("{extension}.gz")
    } else {
        extension.to_string()
    };

    let json_file_result = OpenOptions::new()
        .append(true)
        .create(true)
        .open(format!("{output_path}/{output_name}.{file_extension}"));

    let mut json_file = match json_file_result {
        Ok(results) => results,
        Err(err) => {
            error!(
                "[artemis-core] Failed to create output file {output_name} at {output_path}. Error: {err:?}"
            );
            return Err(LocalError::CreateFile);
        }
    };

    let write_result = json_file.write_all(data);
    match write_result {
        Ok(_) => {}
        Err(err) => {
            error!(
                "[artemis-core] Failed to write output to file {output_name} at {output_path}. Error: {err:?}",
            );
            return Err(LocalError::WriteJson);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{output::local::output::local_output, structs::toml::Output};

    #[test]
    fn test_output_json() {
        let output = Output {
            name: String::from("test_output"),
            directory: String::from("./tmp"),
            format: String::from("json"),
            compress: false,
            timeline: false,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: String::from("local"),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        };

        let test = "A rust program";
        let name = "output";
        local_output(test.as_bytes(), &output, name, &output.format).unwrap();
    }

    #[test]
    fn test_output_json_compress() {
        let output = Output {
            name: String::from("test_output"),
            directory: String::from("./tmp"),
            format: String::from("json"),
            compress: true,
            url: Some(String::new()),
            timeline: false,
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: String::from("local"),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        };

        let test = "A rust program";
        let name = "output";
        local_output(test.as_bytes(), &output, name, &output.format).unwrap();
    }
}
