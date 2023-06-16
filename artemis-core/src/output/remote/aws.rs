use super::error::RemoteError;
use crate::utils::{
    artemis_toml::Output, compression::compress_gzip_data, encoding::base64_decode_standard,
};
use log::{error, info};
use reqwest::{blocking::Client, StatusCode, Url};
use rusty_s3::{actions::PutObject, Bucket, Credentials, S3Action, UrlStyle};
use serde::Deserialize;
use std::time::Duration;

/// Upload data to AWS S3 Bucket using a signed URL signature
pub(crate) fn aws_upload(data: &[u8], output: &Output, filename: &str) -> Result<(), RemoteError> {
    let aws_url = if let Some(url) = &output.url {
        url
    } else {
        return Err(RemoteError::RemoteUrl);
    };

    // Grab service account key info (base64 encoded)
    let api_key = if let Some(key) = &output.api_key {
        key
    } else {
        return Err(RemoteError::RemoteApiKey);
    };

    let mut aws_filename = format!(
        "{}/{}/{filename}.{}",
        output.directory, output.name, output.format
    );

    let aws_endpoint_url_result = aws_url.parse();
    let aws_endpoint_url: Url = match aws_endpoint_url_result {
        Ok(result) => result,
        Err(err) => {
            error!("[artemis-core] Could not parse AWS URL: {err:?}");
            return Err(RemoteError::RemoteUrl);
        }
    };

    let mut header_value = "application/json-seq";

    let output_data = if output.compress {
        aws_filename = format!("{aws_filename}.gz");
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

    let aws_info = aws_creds(api_key)?;

    let bucket_result = Bucket::new(
        aws_endpoint_url,
        UrlStyle::VirtualHost,
        aws_info.bucket,
        aws_info.region,
    );
    let bucket = match bucket_result {
        Ok(result) => result,
        Err(err) => {
            error!("[artemis-core] Could not create bucket request: {err:?}");
            return Err(RemoteError::RemoteUpload);
        }
    };

    let creds = Credentials::new(aws_info.key, aws_info.secret);
    // Valid for one hour
    let duration = Duration::from_secs(3600);

    let action = PutObject::new(&bucket, Some(&creds), &aws_filename);
    let mut signed_url = action.sign(duration);

    // This is used for our test to ensure we hit the mock server
    if signed_url
        .as_str()
        .starts_with("http://blah.replacemeduh.com")
    {
        signed_url.set_host(Some("127.0.0.1")).unwrap();
    }

    let client = Client::new();

    let mut builder = client.put(signed_url);
    builder = builder.header("Content-Type", header_value);
    builder = builder.header("Content-Length", output_data.len());

    let res_result = builder.body(output_data.clone()).send();
    let res = match res_result {
        Ok(result) => result,
        Err(err) => {
            error!("[artemis-core] Failed to upload data to AWS S3 bucket: {err:?}");
            return Err(RemoteError::RemoteUpload);
        }
    };
    if res.status() != StatusCode::OK && res.status() != StatusCode::CREATED {
        error!(
            "[artemis-core] Non-200 response from AWS S3 bucket: {:?}",
            res.text()
        );
        return Err(RemoteError::RemoteUpload);
    }

    info!(
        "[artemis-core] Uploaded {} bytes to AWS S3 bucket",
        output_data.len()
    );
    Ok(())
}

#[derive(Deserialize)]
struct AwsInfo {
    bucket: String,
    secret: String,
    key: String,
    region: String,
}

/// Base64 decode the AWS key info
fn aws_creds(keys: &str) -> Result<AwsInfo, RemoteError> {
    let aws_info_result = base64_decode_standard(keys);
    let aws_info = match aws_info_result {
        Ok(result) => result,
        Err(err) => {
            error!("[artemis-core] Could not base64 decode AWS API key info: {err:?}");
            return Err(RemoteError::RemoteApiKey);
        }
    };
    let aws_key_result = serde_json::from_slice(&aws_info);
    let aws_key: AwsInfo = match aws_key_result {
        Ok(result) => result,
        Err(err) => {
            error!("[artemis-core] Could not parse AWS API key json: {err:?}");
            return Err(RemoteError::RemoteApiKey);
        }
    };

    Ok(aws_key)
}

#[cfg(test)]
mod tests {
    use super::{aws_creds, aws_upload};
    use crate::utils::artemis_toml::Output;
    use httpmock::{Method::PUT, MockServer};

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
            url: Some(format!("http://replacemeduh.com:{port}")),
            port: Some(port),
            // Fake keys created at https://canarytokens.org/generate
            api_key: Some(String::from("ewogICAgImJ1Y2tldCI6ICJibGFoIiwKICAgICJzZWNyZXQiOiAicGtsNkFpQWFrL2JQcEdPenlGVW9DTC96SW1hSEoyTzVtR3ZzVWxSTCIsCiAgICAia2V5IjogIkFLSUEyT0dZQkFINlRPSUFVSk1SIiwKICAgICJyZWdpb24iOiAidXMtZWFzdC0yIgp9")),
            username: Some(String::from("foo")),
            password: Some(String::from("pass")),
            generic_keys: Some(Vec::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
        }
    }

    #[test]
    fn test_aws_upload() {
        let server = MockServer::start();
        let port = server.port();
        let output = output_options("aws_upload_test", "aws", "tmp", false, port);

        let test = "A rust program";
        let name = "output";
        let mock_me = server.mock(|when, then| {
            when.method(PUT);
            then.status(200);
        });

        aws_upload(test.as_bytes(), &output, name).unwrap();
        mock_me.assert();
    }

    #[test]
    fn test_aws_upload_compress() {
        let server = MockServer::start();
        let port = server.port();
        let output = output_options("aws_upload_test", "aws", "tmp", true, port);

        let test = "A rust program";
        let name = "output";
        let mock_me = server.mock(|when, then| {
            when.method(PUT);
            then.status(200);
        });

        aws_upload(test.as_bytes(), &output, name).unwrap();
        mock_me.assert();
    }

    #[test]
    fn test_aws_keys() {
        let test = "ewogICAgImJ1Y2tldCI6ICJibGFoIiwKICAgICJzZWNyZXQiOiAicGtsNkFpQWFrL2JQcEdPenlGVW9DTC96SW1hSEoyTzVtR3ZzVWxSTCIsCiAgICAia2V5IjogIkFLSUEyT0dZQkFINlRPSUFVSk1SIiwKICAgICJyZWdpb24iOiAidXMtZWFzdC0yIgp9";

        let results = aws_creds(test).unwrap();
        assert_eq!(results.bucket, "blah");
        assert_eq!(results.region, "us-east-2");
        assert_eq!(results.key, "AKIA2OGYBAH6TOIAUJMR");
        assert_eq!(results.secret, "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");
    }
}
