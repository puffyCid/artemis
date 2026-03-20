use super::error::RemoteError;
use crate::{
    output::remote::data::prep_data_upload, structs::toml::Output, utils::uuid::generate_uuid,
};
use log::error;
use reqwest::{
    StatusCode,
    blocking::{Client, multipart},
};
use serde_json::Value;
use std::{thread::sleep, time::Duration};

/// Upload data to a remote server. For now we use our unique endpoint ID for authentication
/// It should have been obtained from our initial enrollment when running in deamon mode
/// Inspired by osquery approach to remote uploads <https://osquery.readthedocs.io/en/stable/deployment/remote/>
pub(crate) fn api_upload(
    serde_data: &mut Value,
    output: &mut Output,
    artifact_name: &str,
    start_time: u64,
) -> Result<(), RemoteError> {
    let uuid = generate_uuid();
    let filename = format!("{artifact_name}_{uuid}");
    let data = prep_data_upload(serde_data, output, "api", artifact_name, start_time)?;

    let api_url = if let Some(url) = &output.url {
        url
    } else {
        return Err(RemoteError::RemoteUrl);
    };

    let client = Client::new();

    let mut attempt = 1;
    let max_attempts = 6;
    let pause = 8;

    loop {
        let mut builder = client.post(api_url);
        builder = builder.header("x-artemis-endpoint_id", &output.endpoint_id);
        builder = builder.header("x-artemis-collection_id", &output.collection_id.to_string());
        builder = builder.header("x-artemis-collection_name", &output.name);
        builder = builder.header("accept", "application/json");

        let mut part = multipart::Part::bytes(data.clone());
        part = part.file_name(filename.clone());

        if filename.ends_with(".log") {
            // The last two uploads for collections are just plaintext log files
            part = part.mime_str("text/plain").unwrap();
        } else {
            builder = builder.header("Content-Encoding", "gzip");
            // Should be safe to unwrap?
            part = part.mime_str("application/jsonl").unwrap();
        }

        let form = multipart::Form::new().part("artemis-upload", part);
        builder = builder.multipart(form);

        let jitter = fastrand::usize(..11);

        let backoff = if attempt <= max_attempts {
            pause * attempt + jitter
        } else {
            // If 6 attempts fail. Then backoff for 5 mins
            300 + jitter
        };
        let status = match builder.send() {
            Ok(result) => result,
            Err(err) => {
                error!(
                    "[forensics] Failed to upload data to {api_url}. Attempt {attempt}. Error: {err:?}"
                );

                // Pause between each attempt
                sleep(Duration::from_secs(backoff as u64));
                attempt += 1;
                continue;
            }
        };
        if status.status() == StatusCode::OK {
            break;
        }

        // Pause between each attempt
        sleep(Duration::from_secs(backoff as u64));
        attempt += 1;
    }

    // Track output files
    output.output_count += 1;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::api_upload;
    use crate::structs::toml::Output;
    use httpmock::{Method::POST, MockServer};
    use serde_json::json;

    fn output_options(
        name: &str,
        output: &str,
        directory: &str,
        compress: bool,
        port: u16,
    ) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
            compress,
            timeline: false,
            url: Some(format!("http://127.0.0.1:{port}")),
            api_key: None,
            endpoint_id: String::from("abcd"),
            output: output.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_api_upload() {
        let server = MockServer::start();
        let port = server.port();
        let mut output = output_options("api_upload_test", "api", "tmp", false, port);

        let mock_me = server.mock(|when, then| {
            when.method(POST).header("x-artemis-endpoint_id", "abcd");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({ "message": "ok" }));
        });

        let test = "A rust program";
        api_upload(
            &mut serde_json::to_value(&test).unwrap(),
            &mut output,
            "uuid",
            0,
        )
        .unwrap();
        mock_me.assert();
    }

    #[test]
    #[should_panic(expected = "RemoteUrl")]
    fn test_api_bad_upload() {
        let server = MockServer::start();
        let port = server.port();
        let mut output = output_options("api_upload_test", "api", "tmp", false, port);
        output.url = None;

        let mock_me = server.mock(|when, then| {
            when.method(POST).header("x-artemis-endpoint_id", "abcd");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({ "message": "ok" }));
        });

        let test = "A rust program";
        api_upload(
            &mut serde_json::to_value(&test).unwrap(),
            &mut output,
            "uuid",
            1,
        )
        .unwrap();
        mock_me.assert();
    }
}
