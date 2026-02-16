use super::error::RemoteError;
use crate::structs::toml::Output;
use log::{error, info, warn};
use reqwest::{StatusCode, blocking::Client, header::HeaderMap};
use std::time::Duration;

/// Upload data to Azure Blob Storage using a shared access signature (SAS) URI
pub(crate) fn azure_upload(
    data: &[u8],
    output: &mut Output,
    filename: &str,
) -> Result<(), RemoteError> {
    let azure_url = if let Some(url) = &output.url {
        url
    } else {
        return Err(RemoteError::RemoteUrl);
    };

    // Log files are not compressed
    let azure_filename = if filename.ends_with(".log") {
        format!("{}%2F{}%2F{filename}", output.directory, output.name)
    } else {
        let mut compression_extension = "";
        if output.compress {
            compression_extension = ".gz";
        }

        format!(
            "{}%2F{}%2F{filename}.{}{compression_extension}",
            output.directory, output.name, output.format
        )
    };

    let azure_full_url = compose_azure_url(azure_url, &azure_filename)?;

    azure_url_upload(&azure_full_url, &HeaderMap::new(), data, data.len())?;

    info!(
        "[forensics] Uploaded {} bytes to Azure blob storage",
        data.len()
    );

    // Track output files
    output.output_count += 1;
    Ok(())
}

/// Upload bytes to Azure
pub(crate) fn azure_url_upload(
    url: &str,
    headers: &HeaderMap,
    data: &[u8],
    size: usize,
) -> Result<(), RemoteError> {
    let client = Client::new();
    let max_attempts = 15;
    let mut attempts = 0;

    while attempts < max_attempts {
        let mut builder = client.put(url);
        builder = builder.header("Content-Type", "application/json-seq");
        builder = builder.header("Content-Length", size);
        builder = builder.header("x-ms-version", "2019-12-12");

        if !url.contains("&comp=") {
            builder = builder.header("x-ms-blob-type", "Blockblob");
        }

        for (key, value) in headers {
            builder = builder.header(key, value);
        }

        builder = builder.timeout(Duration::from_secs(300));

        builder = builder.body(data.to_vec());
        let res_result = builder.send();
        let res = match res_result {
            Ok(result) => result,
            Err(err) => {
                error!("[forensics] Failed to upload data to Azure blob storage: {err:?}");
                return Err(RemoteError::RemoteUpload);
            }
        };

        if res.status() != StatusCode::OK && res.status() != StatusCode::CREATED {
            if attempts < max_attempts {
                warn!(
                    "[forensics] Non-200 response on attempt {attempts} out of {max_attempts}. Response: {res:?}"
                );

                attempts += 1;
                continue;
            }
            error!(
                "[forensics] Non-200 response from Azure blob storage: {:?}",
                res.text()
            );
            return Err(RemoteError::RemoteUpload);
        }
        break;
    }

    Ok(())
}

/// Compose the final URL to upload data to Azure
pub(crate) fn compose_azure_url(azure_url: &str, filename: &str) -> Result<String, RemoteError> {
    let azure_uris: Vec<&str> = azure_url.split('?').collect();
    let expected_len = 2;
    if azure_uris.len() < expected_len {
        error!("[forensics] Unexpected Azure URL provided: {azure_url}");
        return Err(RemoteError::RemoteUrl);
    }

    Ok(format!("{}/{filename}?{}", azure_uris[0], azure_uris[1]))
}

#[cfg(test)]
mod tests {
    use super::{azure_upload, compose_azure_url};
    use crate::structs::toml::Output;
    use httpmock::{Method::PUT, MockServer};

    fn output_options(
        name: &str,
        output: &str,
        directory: &str,
        compress: bool,
        full_url: &str,
    ) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
            compress,
            timeline: false,
            url: Some(full_url.to_string()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            output: output.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_azure_upload() {
        let server = MockServer::start();
        let port = server.port();
        let mut output = output_options(
            "azure_upload_test",
            "azure",
            "tmp",
            false,
            &format!(
                "http://127.0.0.1:{port}/mycontainername?sp=rcw&st=2023-06-14T03:00:40Z&se=2023-06-14T11:00:40Z&skoid=asdfasdfas-asdfasdfsadf-asdfsfd-sadf"
            ),
        );

        let test = "A rust program";
        let name = "output";
        let mock_me = server.mock(|when, then| {
            when.method(PUT);
            then.status(200)
                .header("Last-Modified", "2023-06-14 12:00:00")
                .header("Content-MD5", "sQqNsWTgdUEFt6mb5y4/5Q==");
        });
        azure_upload(test.as_bytes(), &mut output, name).unwrap();
        mock_me.assert();
    }

    #[test]
    fn test_compose_azure_url() {
        let name = "output";

        let result =  compose_azure_url("http://127.0.0.1/mycontainername?sp=rcw&st=2023-06-14T03:00:40Z&se=2023-06-14T11:00:40Z&skoid=asdfasdfas-asdfasdfsadf-asdfsfd-sadf", name).unwrap();
        assert_eq!(
            result,
            "http://127.0.0.1/mycontainername/output?sp=rcw&st=2023-06-14T03:00:40Z&se=2023-06-14T11:00:40Z&skoid=asdfasdfas-asdfasdfsadf-asdfsfd-sadf"
        );
    }

    #[test]
    fn test_azure_upload_compress() {
        let server = MockServer::start();
        let port = server.port();
        let mut output = output_options(
            "azure_upload_test",
            "azure",
            "tmp",
            true,
            &format!(
                "http://127.0.0.1:{port}/mycontainername?sp=rcw&st=2023-06-14T03:00:40Z&se=2023-06-14T11:00:40Z&skoid=asdfasdfas-asdfasdfsadf-asdfsfd-sadf"
            ),
        );

        let test = "A rust program";
        let name = "output";
        let mock_me = server.mock(|when, then| {
            when.method(PUT);
            then.status(200)
                .header("Last-Modified", "2023-06-14 12:00:00")
                .header("Content-MD5", "sQqNsWTgdUEFt6mb5y4/5Q==");
        });
        azure_upload(test.as_bytes(), &mut output, name).unwrap();
        mock_me.assert();
    }

    #[test]
    #[should_panic(expected = "RemoteUrl")]
    fn test_azure_upload_bad_url() {
        let server = MockServer::start();
        let port = server.port();
        let mut output = output_options(
            "azure_upload_test",
            "azure",
            "tmp",
            false,
            &format!(
                "http://127.0.0.1:{port}/mycontainernamesp=rcw&st=2023-06-14T03:00:40Z&se=2023-06-14T11:00:40Z&skoid=asdfasdfas-asdfasdfsadf-asdfsfd-sadf"
            ),
        );

        let test = "A rust program";
        let name = "output";
        let mock_me = server.mock(|when, then| {
            when.method(PUT);
            then.status(200)
                .header("Last-Modified", "2023-06-14 12:00:00")
                .header("Content-MD5", "sQqNsWTgdUEFt6mb5y4/5Q==");
        });
        azure_upload(test.as_bytes(), &mut output, name).unwrap();
        mock_me.assert();
    }
}
