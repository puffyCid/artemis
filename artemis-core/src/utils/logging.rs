use super::{
    artemis_toml::Output, error::ArtemisError, output::output_artifact, uuid::generate_uuid,
};
use crate::filesystem::files::{get_filename, list_files, read_file};
use log::{error, warn, LevelFilter};
use std::{
    fs::{create_dir_all, remove_dir, remove_file, File, OpenOptions},
    io::Write,
};

/// Create log output file and logging level based on TOML `Output` configuration
pub(crate) fn create_log_file(output: &Output) -> Result<(File, LevelFilter), ArtemisError> {
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

    let level = if let Some(log_level) = &output.logging {
        match log_level.to_lowercase().as_str() {
            "warn" => LevelFilter::Warn,
            "error" => LevelFilter::Error,
            "info" => LevelFilter::Info,
            "debug" => LevelFilter::Debug,
            _ => LevelFilter::Warn,
        }
    } else {
        LevelFilter::Warn
    };

    Ok((log_file, level))
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

/// Upload artemis logs
pub(crate) fn upload_logs(output_dir: &str, output: &Output) -> Result<(), ArtemisError> {
    let files_res = list_files(output_dir);
    let log_files = match files_res {
        Ok(results) => results,
        Err(err) => {
            warn!("[artemis-core] Could not get list of logs to upload: {err:?}");
            return Ok(());
        }
    };

    for log in log_files {
        if !log.ends_with(".log") {
            continue;
        }

        let read_res = read_file(&log);
        let log_data = match read_res {
            Ok(result) => result,
            Err(err) => {
                warn!("[artemis-core] Could not read log file {log}: {err:?}");
                continue;
            }
        };
        output_artifact(&log_data, output, &get_filename(&log))?;
        let _ = remove_file(&log);
    }

    // Now remove directory if its empty
    let remove_status = remove_dir(output_dir);
    match remove_status {
        Ok(_) => {}
        Err(err) => {
            error!("[artemis-core] Failed to remove output directory: {err:?}");
            return Err(ArtemisError::Cleanup);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{collection_status, create_log_file, upload_logs};
    use crate::utils::artemis_toml::Output;
    use httpmock::{
        Method::{POST, PUT},
        MockServer,
    };
    use log::{warn, LevelFilter};
    use serde_json::json;
    use simplelog::{Config, WriteLogger};
    use std::{fs::File, io::Write, path::PathBuf};

    #[test]
    fn test_create_log_file() {
        let test = Output {
            name: String::from("logging"),
            directory: String::from("tmp"),
            format: String::from("json"),
            compress: false,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: String::from("local"),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        };

        let (result, level) = create_log_file(&test).unwrap();
        let _ = WriteLogger::init(log::LevelFilter::Warn, Config::default(), result);
        warn!("A simple fancy logger!");
        assert_eq!(level, LevelFilter::Warn);
    }

    #[test]
    fn test_collection_status() {
        let test = Output {
            name: String::from("logging"),
            directory: String::from("tmp"),
            format: String::from("json"),
            compress: false,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: String::from("local"),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        };

        collection_status("test", &test, "c639679b-40ec-4aca-9ed1-dc740c38731c").unwrap();
    }

    #[test]
    fn test_upload_logs() {
        let server = MockServer::start();
        let port = server.port();

        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/system");
        let mut test_log = File::create(format!(
            "{}/files/test.log",
            test_location.display().to_string()
        ))
        .unwrap();
        test_log.write_all(b"testing!").unwrap();

        let output = Output {
            name: String::from("files"),
            directory: test_location.display().to_string(),
            format: String::from("json"),
            compress: false,
            url: Some(format!("http://127.0.0.1:{port}")),
            api_key: Some(String::from("ewogICJ0eXBlIjogInNlcnZpY2VfYWNjb3VudCIsCiAgInByb2plY3RfaWQiOiAiZmFrZW1lIiwKICAicHJpdmF0ZV9rZXlfaWQiOiAiZmFrZW1lIiwKICAicHJpdmF0ZV9rZXkiOiAiLS0tLS1CRUdJTiBQUklWQVRFIEtFWS0tLS0tXG5NSUlFdndJQkFEQU5CZ2txaGtpRzl3MEJBUUVGQUFTQ0JLa3dnZ1NsQWdFQUFvSUJBUUM3VkpUVXQ5VXM4Y0tqTXpFZll5amlXQTRSNC9NMmJTMUdCNHQ3TlhwOThDM1NDNmRWTXZEdWljdEdldXJUOGpOYnZKWkh0Q1N1WUV2dU5Nb1NmbTc2b3FGdkFwOEd5MGl6NXN4alptU25YeUNkUEVvdkdoTGEwVnpNYVE4cytDTE95UzU2WXlDRkdlSlpxZ3R6SjZHUjNlcW9ZU1c5YjlVTXZrQnBaT0RTY3RXU05HajNQN2pSRkRPNVZvVHdDUUFXYkZuT2pEZkg1VWxncDJQS1NRblNKUDNBSkxRTkZOZTdicjFYYnJoVi8vZU8rdDUxbUlwR1NEQ1V2M0UwRERGY1dEVEg5Y1hEVFRsUlpWRWlSMkJ3cFpPT2tFL1owL0JWbmhaWUw3MW9aVjM0YktmV2pRSXQ2Vi9pc1NNYWhkc0FBU0FDcDRaVEd0d2lWdU5kOXR5YkFnTUJBQUVDZ2dFQkFLVG1qYVM2dGtLOEJsUFhDbFRRMnZwei9ONnV4RGVTMzVtWHBxYXNxc2tWbGFBaWRnZy9zV3FwalhEYlhyOTNvdElNTGxXc00rWDBDcU1EZ1NYS2VqTFMyang0R0RqSTFaVFhnKyswQU1KOHNKNzRwV3pWRE9mbUNFUS83d1hzMytjYm5YaEtyaU84WjAzNnE5MlFjMStOODdTSTM4bmtHYTBBQkg5Q044M0htUXF0NGZCN1VkSHp1SVJlL21lMlBHaElxNVpCemo2aDNCcG9QR3pFUCt4M2w5WW1LOHQvMWNOMHBxSStkUXdZZGdmR2phY2tMdS8ycUg4ME1DRjdJeVFhc2VaVU9KeUtyQ0x0U0QvSWl4di9oekRFVVBmT0NqRkRnVHB6ZjNjd3RhOCtvRTR3SENvMWlJMS80VGxQa3dtWHg0cVNYdG13NGFRUHo3SURRdkVDZ1lFQThLTlRoQ08yZ3NDMkk5UFFETS84Q3cwTzk4M1dDRFkrb2krN0pQaU5BSnd2NURZQnFFWkIxUVlkajA2WUQxNlhsQy9IQVpNc01rdTFuYTJUTjBkcml3ZW5RUVd6b2V2M2cyUzdnUkRvUy9GQ0pTSTNqSitramd0YUE3UW16bGdrMVR4T0ROK0cxSDkxSFc3dDBsN1ZuTDI3SVd5WW8ycVJSSzNqenhxVWlQVUNnWUVBeDBvUXMycmVCUUdNVlpuQXBEMWplcTduNE12TkxjUHZ0OGIvZVU5aVV2Nlk0TWowU3VvL0FVOGxZWlhtOHViYnFBbHd6MlZTVnVuRDJ0T3BsSHlNVXJ0Q3RPYkFmVkRVQWhDbmRLYUE5Z0FwZ2ZiM3h3MUlLYnVRMXU0SUYxRkpsM1Z0dW1mUW4vL0xpSDFCM3JYaGNkeW8zL3ZJdHRFazQ4UmFrVUtDbFU4Q2dZRUF6VjdXM0NPT2xERGNRZDkzNURkdEtCRlJBUFJQQWxzcFFVbnpNaTVlU0hNRC9JU0xEWTVJaVFIYklIODNENGJ2WHEwWDdxUW9TQlNOUDdEdnYzSFl1cU1oZjBEYWVncmxCdUpsbEZWVnE5cVBWUm5LeHQxSWwySGd4T0J2YmhPVCs5aW4xQnpBK1lKOTlVekM4NU8wUXowNkErQ210SEV5NGFaMmtqNWhIakVDZ1lFQW1OUzQrQThGa3NzOEpzMVJpZUsyTG5pQnhNZ21ZbWwzcGZWTEtHbnptbmc3SDIrY3dQTGhQSXpJdXd5dFh5d2gyYnpic1lFZll4M0VvRVZnTUVwUGhvYXJRbllQdWtySk80Z3dFMm81VGU2VDVtSlNaR2xRSlFqOXE0WkIyRGZ6ZXQ2SU5zSzBvRzhYVkdYU3BRdlFoM1JVWWVrQ1pRa0JCRmNwcVdwYklFc0NnWUFuTTNEUWYzRkpvU25YYU1oclZCSW92aWM1bDB4RmtFSHNrQWpGVGV2Tzg2RnN6MUMyYVNlUktTcUdGb09RMHRtSnpCRXMxUjZLcW5ISW5pY0RUUXJLaEFyZ0xYWDR2M0NkZGpmVFJKa0ZXRGJFL0NrdktaTk9yY2YxbmhhR0NQc3BSSmoyS1VrajFGaGw5Q25jZG4vUnNZRU9OYndRU2pJZk1Qa3Z4Ris4SFE9PVxuLS0tLS1FTkQgUFJJVkFURSBLRVktLS0tLVxuIiwKICAiY2xpZW50X2VtYWlsIjogImZha2VAZ3NlcnZpY2VhY2NvdW50LmNvbSIsCiAgImNsaWVudF9pZCI6ICJmYWtlbWUiLAogICJhdXRoX3VyaSI6ICJodHRwczovL2FjY291bnRzLmdvb2dsZS5jb20vby9vYXV0aDIvYXV0aCIsCiAgInRva2VuX3VyaSI6ICJodHRwczovL29hdXRoMi5nb29nbGVhcGlzLmNvbS90b2tlbiIsCiAgImF1dGhfcHJvdmlkZXJfeDUwOV9jZXJ0X3VybCI6ICJodHRwczovL3d3dy5nb29nbGVhcGlzLmNvbS9vYXV0aDIvdjEvY2VydHMiLAogICJjbGllbnRfeDUwOV9jZXJ0X3VybCI6ICJodHRwczovL3d3dy5nb29nbGVhcGlzLmNvbS9yb2JvdC92MS9tZXRhZGF0YS94NTA5L2Zha2VtZSIsCiAgInVuaXZlcnNlX2RvbWFpbiI6ICJnb29nbGVhcGlzLmNvbSIKfQo=")),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: String::from("gcp"),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
logging: Some(String::new())
        };

        let mock_me = server.mock(|when, then| {
            when.method(POST);
            then.status(200)
                .header("content-type", "application/json")
                .header("Location", format!("http://127.0.0.1:{port}"))
                .json_body(json!({ "timeCreated": "whatever", "name":"mockme" }));
        });

        let mock_me_put = server.mock(|when, then| {
            when.method(PUT);
            then.status(200)
                .header("content-type", "application/json")
                .header("Location", format!("http://127.0.0.1:{port}"))
                .json_body(json!({ "timeCreated": "whatever", "name":"mockme" }));
        });

        let output_dir = format!("{}/{}", output.directory, output.name);

        let _ = upload_logs(&output_dir, &output);
        mock_me.assert();
        mock_me_put.assert();
    }
}
