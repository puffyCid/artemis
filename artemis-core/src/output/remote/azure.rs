use super::error::RemoteError;
use crate::utils::{artemis_toml::Output, compression::compress_gzip_data};
use log::{error, info, warn};
use reqwest::{blocking::Client, StatusCode};

/// Upload data to Azure Blob Storage using a shared access signature (SAS) URI
pub(crate) fn azure_upload(
    data: &[u8],
    output: &Output,
    filename: &str,
) -> Result<(), RemoteError> {
    let azure_url = if let Some(url) = &output.url {
        url
    } else {
        return Err(RemoteError::RemoteUrl);
    };

    let mut header_value = "application/json-seq";
    let mut azure_filename = format!(
        "{}%2F{}%2F{filename}.{}",
        output.directory, output.name, output.format
    );
    let output_data = if output.compress {
        azure_filename = format!("{azure_filename}.gz");
        header_value = "application/gzip";
        let compressed_results = compress_gzip_data(data);
        match compressed_results {
            Ok(result) => result,
            Err(err) => {
                error!("[artemis-core] Failed to compress data: {err:?}");
                return Err(RemoteError::CompressFailed);
            }
        }
    } else {
        data.to_vec()
    };

    let azure_uris: Vec<&str> = azure_url.split('?').collect();
    let expected_len = 2;
    if azure_uris.len() < expected_len {
        error!("[artemis-core] Unexpected Azure URL provided: {azure_url}");
        return Err(RemoteError::RemoteUrl);
    }

    let client = Client::new();
    let max_attempts = 15;
    let mut attempts = 0;

    while attempts < max_attempts {
        let azure_full_url = format!("{}/{azure_filename}?{}", azure_uris[0], azure_uris[1]);

        let mut builder = client.put(azure_full_url);
        builder = builder.header("Content-Type", header_value);
        builder = builder.header("Content-Length", output_data.len());
        builder = builder.header("x-ms-version", "2019-12-12");
        builder = builder.header("x-ms-blob-type", "Blockblob");
        builder = builder.body(output_data.clone());

        let res_result = builder.send();
        let res = match res_result {
            Ok(result) => result,
            Err(err) => {
                error!("[artemis-core] Failed to upload data to Azure blob storage: {err:?}");
                return Err(RemoteError::RemoteUpload);
            }
        };

        if res.status() != StatusCode::OK && res.status() != StatusCode::CREATED {
            if attempts < max_attempts {
                warn!("[artemis-core] Non-200 response on attempt {attempts} out of {max_attempts}. Response: {res:?}");
                attempts += 1;
                continue;
            }
            error!(
                "[artemis-core] Non-200 response from Azure blob storage: {:?}",
                res.text()
            );
            return Err(RemoteError::RemoteUpload);
        }
        break;
    }

    info!(
        "[artemis-core] Uploaded {} bytes to Azure blob storage",
        output_data.len()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::azure_upload;
    use crate::utils::artemis_toml::Output;
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
            url: Some(full_url.to_string()),
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
    fn test_azure_upload() {
        let server = MockServer::start();
        let port = server.port();
        let output = output_options("azure_upload_test", "azure", "tmp", false, &format!("http://127.0.0.1:{port}/mycontainername?sp=rcw&st=2023-06-14T03:00:40Z&se=2023-06-14T11:00:40Z&skoid=asdfasdfas-asdfasdfsadf-asdfsfd-sadf"));

        let test = "A rust program";
        let name = "output";
        let mock_me = server.mock(|when, then| {
            when.method(PUT);
            then.status(200)
                .header("Last-Modified", "2023-06-14 12:00:00")
                .header("Content-MD5", "sQqNsWTgdUEFt6mb5y4/5Q==");
        });
        azure_upload(test.as_bytes(), &output, name).unwrap();
        mock_me.assert();
    }

    #[test]
    fn test_azure_upload_compress() {
        let server = MockServer::start();
        let port = server.port();
        let output = output_options("azure_upload_test", "azure", "tmp", true, &format!("http://127.0.0.1:{port}/mycontainername?sp=rcw&st=2023-06-14T03:00:40Z&se=2023-06-14T11:00:40Z&skoid=asdfasdfas-asdfasdfsadf-asdfsfd-sadf"));

        let test = "A rust program";
        let name = "output";
        let mock_me = server.mock(|when, then| {
            when.method(PUT);
            then.status(200)
                .header("Last-Modified", "2023-06-14 12:00:00")
                .header("Content-MD5", "sQqNsWTgdUEFt6mb5y4/5Q==");
        });
        azure_upload(test.as_bytes(), &output, name).unwrap();
        mock_me.assert();
    }

    #[test]
    #[should_panic(expected = "RemoteUrl")]
    fn test_azure_upload_bad_url() {
        let server = MockServer::start();
        let port = server.port();
        let output = output_options("azure_upload_test", "azure", "tmp", false, &format!("http://127.0.0.1:{port}/mycontainernamesp=rcw&st=2023-06-14T03:00:40Z&se=2023-06-14T11:00:40Z&skoid=asdfasdfas-asdfasdfsadf-asdfsfd-sadf"));

        let test = "A rust program";
        let name = "output";
        let mock_me = server.mock(|when, then| {
            when.method(PUT);
            then.status(200)
                .header("Last-Modified", "2023-06-14 12:00:00")
                .header("Content-MD5", "sQqNsWTgdUEFt6mb5y4/5Q==");
        });
        azure_upload(test.as_bytes(), &output, name).unwrap();
        mock_me.assert();
    }
}
