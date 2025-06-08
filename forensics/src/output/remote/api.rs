use super::error::RemoteError;
use crate::{
    structs::toml::Output,
    utils::time::{time_now, unixepoch_to_iso},
};
use log::error;
use reqwest::{StatusCode, blocking::Client};
use std::{thread::sleep, time::Duration};

/// Upload data to a remote server. We use our unique endpoint ID for authentication
/// It should have been obtained from our initial enrollment when running in deamon mode
/// Inspired by osquery approach to remote uploads <https://osquery.readthedocs.io/en/stable/deployment/remote/>
pub(crate) fn api_upload(data: &[u8], output: &Output, complete: &bool) -> Result<(), RemoteError> {
    let api_url = if let Some(url) = &output.url {
        url
    } else {
        return Err(RemoteError::RemoteUrl);
    };

    let client = Client::new();

    let mut count = 0;
    let max_attempts = 8;
    let pause = 6;
    while count < max_attempts {
        let mut builder = client.post(api_url);
        builder = builder.header("x-artemis-endpoint_id", &output.endpoint_id);
        builder = builder.header("x-artemis-collection_id", &output.collection_id.to_string());
        builder = builder.header("x-artemis-collection_name", &output.name);

        if *complete {
            // This is the last upload associated with the collection
            builder = builder.header(
                "x-artemis-collection-complete",
                unixepoch_to_iso(&(time_now() as i64)),
            );
            // The final upload is just plaintext log files
            builder = builder.header("Content-Type", "application/text");
        } else {
            builder = builder.header("Content-Encoding", "gzip");
            builder = builder.header("Content-Type", "application/jsonl");
        }

        let status = match builder.body(data.to_vec()).send() {
            Ok(result) => result,
            Err(err) => {
                error!(
                    "[core] Failed to upload data to {api_url}. Attempt {count}. Error: {err:?}"
                );
                // Pause for 6 seconds between each attempt
                sleep(Duration::from_secs(pause));
                count += 1;
                continue;
            }
        };
        if status.status() == StatusCode::OK {
            break;
        }

        // Pause for 6 seconds between each attempt
        sleep(Duration::from_secs(pause));
        count += 1;
    }

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
            collection_id: 0,
            output: output.to_string(),
            filter_name: None,
            filter_script: None,
            logging: None,
        }
    }

    #[test]
    fn test_api_upload() {
        let server = MockServer::start();
        let port = server.port();
        let output = output_options("api_upload_test", "api", "tmp", false, port);

        let mock_me = server.mock(|when, then| {
            when.method(POST).header("x-artemis-endpoint_id", "abcd");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({ "message": "ok" }));
        });

        let test = "A rust program";
        api_upload(test.as_bytes(), &output, &true).unwrap();
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
        api_upload(test.as_bytes(), &output, &false).unwrap();
        mock_me.assert();
    }
}
