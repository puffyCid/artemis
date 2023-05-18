use super::{artemis_toml::Output, error::ArtemisError, uuid::generate_uuid};
use log::error;
use std::{
    fs::{create_dir_all, File, OpenOptions},
    io::Write,
};

/// Create log output file based on TOML `Output` configuration
pub(crate) fn create_log_file(output: &Output) -> Result<File, ArtemisError> {
    let path = format!("{}/{}", output.directory, output.name);
    let result = create_dir_all(&path);
    match result {
        Ok(_) => {}
        Err(err) => {
            error!("[artemis-core] Failed to create logging output directory for {path}. Error: {err:?}");
            return Err(ArtemisError::CreateDirectory);
        }
    }

    let output_result = File::create(format!("{path}/{}.log", generate_uuid()));
    let log_file = match output_result {
        Ok(result) => result,
        Err(err) => {
            error!("[artemis-core] Failed to create log file at {path}. Error: {err:?}");
            return Err(ArtemisError::LogFile);
        }
    };

    Ok(log_file)
}

/// Create and update a simple `status.log` file to track our output data
pub(crate) fn collection_status(
    artifact_name: &str,
    output: &Output,
    output_name: &str,
) -> Result<(), ArtemisError> {
    let path = format!("{}/{}", output.directory, output.name);
    let result = create_dir_all(&path);
    match result {
        Ok(_) => {}
        Err(err) => {
            error!("[artemis-core] Failed to create status output directory for {path}. Error: {err:?}");
            return Err(ArtemisError::CreateDirectory);
        }
    }

    let status_log = format!("{path}/status.log");
    let status_result = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(status_log);

    let mut status = match status_result {
        Ok(result) => result,
        Err(err) => {
            error!("[artemis-core] Failed to open or create status.log at  {path}. Error: {err:?}");
            return Err(ArtemisError::LogFile);
        }
    };

    /*
     * This is a simple log file that maps artifact names to the uuid filename
     * Ex: amcache:c639679b-40ec-4aca-9ed1-dc740c38731c.json
     * The JSON file also contains the artifact name, but this provides a single file to quickly check where each artifact was saved to
     */
    let status_message = format!("{artifact_name}:{output_name}.{}\n", output.format);
    let write_result = status.write_all(status_message.as_bytes());
    match write_result {
        Ok(_) => {}
        Err(err) => {
            error!("[artemis-core] Failed to update status.log at  {path}. Error: {err:?}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{collection_status, create_log_file};
    use crate::utils::artemis_toml::Output;
    use log::warn;
    use simplelog::{Config, WriteLogger};

    #[test]
    fn test_create_log_file() {
        let test = Output {
            name: String::from("logging"),
            directory: String::from("tmp"),
            format: String::from("json"),
            compress: false,
            // url: Some(String::new()),
            // port: Some(0),
            // api_key: Some(String::new()),
            // username: Some(String::new()),
            // password: Some(String::new()),
            // generic_keys: Some(Vec::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: String::from("local"),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
        };

        let result = create_log_file(&test).unwrap();
        let _ = WriteLogger::init(log::LevelFilter::Warn, Config::default(), result);
        warn!("A simple fancy logger!");
    }

    #[test]
    fn test_collection_status() {
        let test = Output {
            name: String::from("logging"),
            directory: String::from("tmp"),
            format: String::from("json"),
            compress: false,
            // url: Some(String::new()),
            // port: Some(0),
            // api_key: Some(String::new()),
            // username: Some(String::new()),
            // password: Some(String::new()),
            // generic_keys: Some(Vec::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: String::from("local"),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
        };

        collection_status("test", &test, "c639679b-40ec-4aca-9ed1-dc740c38731c").unwrap();
    }
}
