use super::error::RemoteError;
use crate::structs::toml::Output;
use crate::utils::encoding::base64_decode_standard;
use log::{error, warn};
use reqwest::header::ETAG;
use reqwest::{StatusCode, Url, blocking::Client};
use rusty_s3::actions::{
    CompleteMultipartUpload, CreateMultipartUpload, CreateMultipartUploadResponse, S3Action,
    UploadPart,
};
use rusty_s3::{Bucket, Credentials, UrlStyle};
use serde::Deserialize;
use std::collections::HashMap;
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

    let aws_filename = if filename.ends_with(".log") {
        format!("{}/{}/{filename}", output.directory, output.name)
    } else {
        format!(
            "{}/{}/{filename}.{}",
            output.directory, output.name, output.format
        )
    };

    let aws_endpoint_url_result = aws_url.parse();
    let aws_endpoint_url: Url = match aws_endpoint_url_result {
        Ok(result) => result,
        Err(err) => {
            error!("[core] Could not parse AWS URL: {err:?}");
            return Err(RemoteError::RemoteUrl);
        }
    };

    let aws_info = aws_creds(api_key)?;

    let setup = setup_upload(aws_info, aws_endpoint_url, &aws_filename, &HashMap::new())?;

    aws_start_upload(setup, data)
}

pub(crate) struct AwsSetup {
    pub(crate) bucket: Bucket,
    pub(crate) creds: Credentials,
    pub(crate) session: CreateMultipartUploadResponse,
    pub(crate) filename: String,
}

/// Setup requirements to establish an upload session to S3 bucket
pub(crate) fn setup_upload(
    info: AwsInfo,
    url: Url,
    filename: &str,
    headers: &HashMap<String, String>,
) -> Result<AwsSetup, RemoteError> {
    let bucket_result = Bucket::new(url, UrlStyle::VirtualHost, info.bucket, info.region);
    let bucket = match bucket_result {
        Ok(result) => result,
        Err(err) => {
            error!("[core] Could not create bucket request: {err:?}");
            return Err(RemoteError::RemoteUpload);
        }
    };

    let creds = Credentials::new(info.key, info.secret);
    // Valid for one (1) hour
    let duration = Duration::from_secs(3600);

    let mut action = CreateMultipartUpload::new(&bucket, Some(&creds), filename);
    for (key, value) in headers {
        action.headers_mut().insert(key, value);
    }

    let mut url = action.sign(duration);
    // This is used for our test to ensure we hit the mock server
    if url.as_str().starts_with("http://blah.replacemeduh.com") {
        url.set_host(Some("127.0.0.1")).unwrap();
    }

    let response = aws_create_multipart(url.as_str(), headers)?;

    let multipart_res = CreateMultipartUpload::parse_response(&response);
    let session = match multipart_res {
        Ok(result) => result,
        Err(err) => {
            error!("[core] Could not parse session for multipart upload: {err:?}");
            return Err(RemoteError::BadResponse);
        }
    };

    let setup = AwsSetup {
        bucket,
        creds,
        session,
        filename: filename.to_string(),
    };

    Ok(setup)
}

/// Start the AWS data upload
fn aws_start_upload(setup: AwsSetup, output_data: &[u8]) -> Result<(), RemoteError> {
    let first_upload = 1;
    let etag_res = aws_multipart_upload(
        output_data,
        setup.session.upload_id(),
        &setup.bucket,
        &setup.creds,
        &setup.filename,
        first_upload,
    );
    let etag = if let Ok(result) = etag_res {
        result
    } else {
        error!("[core] Could not finish AWS S3 upload");
        return Err(RemoteError::RemoteUpload);
    };

    let etags: Vec<&str> = etag.iter().map(|tag| tag as &str).collect();
    aws_complete_multipart(
        &setup.bucket,
        &setup.creds,
        &setup.filename,
        setup.session.upload_id(),
        etags,
    )
}

/// Create the AWS multipart upload session
fn aws_create_multipart(
    url: &str,
    headers: &HashMap<String, String>,
) -> Result<String, RemoteError> {
    let max_attempts = 15;
    let mut attempts = 0;
    let client = Client::new();

    while attempts < max_attempts {
        let mut builder = client.post(url);

        for (key, value) in headers {
            builder = builder.header(key, value);
        }

        let session_result = builder.send();
        let session = match session_result {
            Ok(result) => result,
            Err(err) => {
                error!("[core] Could not create session for multipart upload: {err:?}");
                return Err(RemoteError::RemoteUpload);
            }
        };

        if session.status() != StatusCode::OK && attempts < max_attempts {
            warn!(
                "[core] Non-200 response create on attempt {attempts} out of {max_attempts}. Response: {session:?}"
            );
            attempts += 1;
            continue;
        }

        let res_result = session.text();
        let response = match res_result {
            Ok(result) => result,
            Err(err) => {
                error!("[core] Could not read response for multipart upload start: {err:?}");
                return Err(RemoteError::BadResponse);
            }
        };
        return Ok(response);
    }

    Err(RemoteError::RemoteUpload)
}

/// Complete and close the multipart upload session
pub(crate) fn aws_complete_multipart(
    bucket: &Bucket,
    creds: &Credentials,
    aws_filename: &str,
    upload_id: &str,
    etags: Vec<&str>,
) -> Result<(), RemoteError> {
    let action = CompleteMultipartUpload::new(
        bucket,
        Some(creds),
        aws_filename,
        upload_id,
        etags.into_iter(),
    );
    // Valid for one (1) hour
    let duration = Duration::from_secs(3600);
    let mut url = action.sign(duration);

    // This is used for our test to ensure we hit the mock server
    if url.as_str().starts_with("http://blah.replacemeduh.com") {
        url.set_host(Some("127.0.0.1")).unwrap();
    }
    let max_attempts = 15;
    let mut attempts = 0;
    let client = Client::new();

    while attempts < max_attempts {
        let complete_builder = client.post(url.as_str());
        let complete_result = complete_builder.body(action.clone().body()).send();
        let complete = match complete_result {
            Ok(result) => result,
            Err(err) => {
                error!("[core] Could not complete multipart upload: {err:?}");
                return Err(RemoteError::RemoteUpload);
            }
        };
        let status = complete.status();

        if status != StatusCode::OK {
            if attempts < max_attempts {
                warn!(
                    "[core] Non-200 response on complete attempt {attempts} out of {max_attempts}. Response: {complete:?}"
                );
                attempts += 1;
                continue;
            }
            error!(
                "[core] Non-200 response when trying to complete upload: {:?}",
                complete.text()
            );
            return Err(RemoteError::RemoteUpload);
        }
        let body = complete.text().unwrap_or_default();
        if status == StatusCode::OK && body.contains("Internal Error") && attempts < max_attempts {
            error!(
                "[core] 200 response on attempt {attempts} out of {max_attempts} but the response contained an error. Response body: {body:?}"
            );
            attempts += 1;
            continue;
        }
        break;
    }

    Ok(())
}

/// Upload data using multipart uploads
pub(crate) fn aws_multipart_upload(
    output_data: &[u8],
    upload_id: &str,
    bucket: &Bucket,
    creds: &Credentials,
    aws_filename: &str,
    id: u16,
) -> Result<Vec<String>, RemoteError> {
    // Valid for one (1) hour
    let duration = Duration::from_secs(3600);

    let part_upload = UploadPart::new(bucket, Some(creds), aws_filename, id, upload_id);

    //let client = Client::new();
    let client = Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .unwrap();
    let max_attempts = 15;
    let mut attempts = 0;

    while attempts < max_attempts {
        let mut signed_url = part_upload.sign(duration);

        // This is used for our test to ensure we hit the mock server
        if signed_url
            .as_str()
            .starts_with("http://blah.replacemeduh.com")
        {
            signed_url.set_host(Some("127.0.0.1")).unwrap();
        }

        let mut builder = client.put(signed_url);
        let header_value = "application/json-seq";

        builder = builder.header("Content-Type", header_value);

        let mut etags: Vec<String> = Vec::new();
        builder = builder.header("Content-Length", output_data.len());

        let res_result = builder.body(output_data.to_vec()).send();
        let response = match res_result {
            Ok(result) => result,
            Err(err) => {
                error!("[core] Could not upload data for multipart upload: {err:?}");
                return Err(RemoteError::RemoteUpload);
            }
        };

        if response.status() != StatusCode::OK {
            if attempts < max_attempts {
                warn!(
                    "[core] Non-200 response on upload attempt {attempts} out of {max_attempts}. Response: {response:?}"
                );
                attempts += 1;
                continue;
            }

            error!(
                "[core] Non-200 response from AWS S3 bucket: {:?}",
                response.text()
            );
            return Err(RemoteError::RemoteUpload);
        }

        if let Some(etag_header) = response.headers().get(ETAG) {
            let etag = etag_header.to_str().unwrap_or_default();
            if etag.is_empty() {
                error!("[core] Got empty ETAG");
                return Err(RemoteError::RemoteUpload);
            }
            etags.push(etag.to_string());
            return Ok(etags);
        }
        error!("[core] Missing ETAG header in response");
        return Err(RemoteError::RemoteUpload);
    }
    error!("[core] Failed to upload to S3 bucket max retries reached");
    Err(RemoteError::RemoteUpload)
}

#[derive(Deserialize)]
pub(crate) struct AwsInfo {
    pub(crate) bucket: String,
    pub(crate) secret: String,
    pub(crate) key: String,
    pub(crate) region: String,
}

/// Base64 decode the AWS key info
pub(crate) fn aws_creds(keys: &str) -> Result<AwsInfo, RemoteError> {
    let aws_info_result = base64_decode_standard(keys);
    let aws_info = match aws_info_result {
        Ok(result) => result,
        Err(err) => {
            error!("[core] Could not base64 decode AWS API key info: {err:?}");
            return Err(RemoteError::RemoteApiKey);
        }
    };
    let aws_key_result = serde_json::from_slice(&aws_info);
    let aws_key: AwsInfo = match aws_key_result {
        Ok(result) => result,
        Err(err) => {
            error!("[core] Could not parse AWS API key json: {err:?}");
            return Err(RemoteError::RemoteApiKey);
        }
    };

    Ok(aws_key)
}

#[cfg(test)]
mod tests {
    use super::{
        aws_complete_multipart, aws_create_multipart, aws_creds, aws_multipart_upload,
        aws_start_upload, aws_upload, setup_upload,
    };
    use crate::structs::toml::Output;
    use httpmock::{
        Method::{POST, PUT},
        MockServer,
    };
    use reqwest::Url;
    use rusty_s3::{Bucket, Credentials, UrlStyle};
    use std::collections::HashMap;

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
            url: Some(format!("http://replacemeduh.com:{port}")),
            // Fake keys created at https://canarytokens.org/generate
            api_key: Some(String::from(
                "ewogICAgImJ1Y2tldCI6ICJibGFoIiwKICAgICJzZWNyZXQiOiAicGtsNkFpQWFrL2JQcEdPenlGVW9DTC96SW1hSEoyTzVtR3ZzVWxSTCIsCiAgICAia2V5IjogIkFLSUEyT0dZQkFINlRPSUFVSk1SIiwKICAgICJyZWdpb24iOiAidXMtZWFzdC0yIgp9",
            )),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
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
            when.method(POST);
            then.status(200).body(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>
            <InitiateMultipartUploadResult>
            <Bucket>mybucket</Bucket>
            <Key>mykey</Key>
            <UploadId>whatever</UploadId>
         </InitiateMultipartUploadResult>",
            );
        });
        let mock_me_put = server.mock(|when, then| {
            when.method(PUT);
            then.status(200).header("ETAG", "whatever");
        });
        aws_upload(test.as_bytes(), &output, name).unwrap();
        mock_me.assert_hits(2);
        mock_me_put.assert();
    }

    #[test]
    fn test_setup_upload() {
        let server = MockServer::start();
        let port = server.port();
        let output = output_options("aws_upload_test", "aws", "tmp", false, port);

        let name = "output";
        let mock_me = server.mock(|when, then| {
            when.method(POST);
            then.status(200).body(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>
            <InitiateMultipartUploadResult>
            <Bucket>mybucket</Bucket>
            <Key>mykey</Key>
            <UploadId>whatever</UploadId>
         </InitiateMultipartUploadResult>",
            );
        });

        let info = aws_creds(output.api_key.as_ref().unwrap()).unwrap();
        let result = setup_upload(
            info,
            Url::parse(output.url.as_ref().unwrap()).unwrap(),
            name,
            &HashMap::new(),
        )
        .unwrap();
        mock_me.assert();

        // This is fake key and data
        assert_eq!(result.creds.key(), "AKIA2OGYBAH6TOIAUJMR")
    }

    #[test]
    fn test_aws_upload_compress() {
        let server = MockServer::start();
        let port = server.port();
        let output = output_options("aws_upload_test", "aws", "tmp", true, port);

        let test = "A rust program";
        let name = "output";
        let mock_me = server.mock(|when, then| {
            when.method(POST);
            then.status(200).body(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>
            <InitiateMultipartUploadResult>
            <Bucket>mybucket</Bucket>
            <Key>mykey</Key>
            <UploadId>whatever</UploadId>
         </InitiateMultipartUploadResult>",
            );
        });
        let mock_me_put = server.mock(|when, then| {
            when.method(PUT);
            then.status(200).header("ETAG", "whatever");
        });
        aws_upload(test.as_bytes(), &output, name).unwrap();
        mock_me.assert_hits(2);
        mock_me_put.assert();
    }

    #[test]
    fn test_aws_start_upload() {
        let server = MockServer::start();
        let port = server.port();
        let output = output_options("aws_upload_test", "aws", "tmp", false, port);

        let test = "A rust program";
        let key = "ewogICAgImJ1Y2tldCI6ICJibGFoIiwKICAgICJzZWNyZXQiOiAicGtsNkFpQWFrL2JQcEdPenlGVW9DTC96SW1hSEoyTzVtR3ZzVWxSTCIsCiAgICAia2V5IjogIkFLSUEyT0dZQkFINlRPSUFVSk1SIiwKICAgICJyZWdpb24iOiAidXMtZWFzdC0yIgp9";
        let name = "output";

        let aws_info = aws_creds(key).unwrap();
        let url: Url = output.url.unwrap().parse().unwrap();

        let mock_me = server.mock(|when, then| {
            when.method(POST);
            then.status(200).body(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>
            <InitiateMultipartUploadResult>
            <Bucket>mybucket</Bucket>
            <Key>mykey</Key>
            <UploadId>whatever</UploadId>
         </InitiateMultipartUploadResult>",
            );
        });
        let mock_me_put = server.mock(|when, then| {
            when.method(PUT);
            then.status(200).header("ETAG", "whatever");
        });

        let setup = setup_upload(aws_info, url, name, &HashMap::new()).unwrap();

        aws_start_upload(setup, test.as_bytes()).unwrap();
        mock_me.assert_hits(2);
        mock_me_put.assert();
    }

    #[test]
    fn test_aws_create_multipart() {
        let server = MockServer::start();
        let port = server.port();
        let url = format!("http://127.0.0.1:{port}");
        let mock_me = server.mock(|when, then| {
            when.method(POST);
            then.status(200).body("hi");
        });

        let result = aws_create_multipart(&url, &HashMap::new()).unwrap();
        assert_eq!(result, "hi");
        mock_me.assert();
    }

    #[test]
    fn test_aws_multipart_upload() {
        let server = MockServer::start();
        let port = server.port();

        let first_upload = 1;
        let test = "hello";
        let url: Url = format!("http://replacemeduh.com:{port}").parse().unwrap();
        let key = "ewogICAgImJ1Y2tldCI6ICJibGFoIiwKICAgICJzZWNyZXQiOiAicGtsNkFpQWFrL2JQcEdPenlGVW9DTC96SW1hSEoyTzVtR3ZzVWxSTCIsCiAgICAia2V5IjogIkFLSUEyT0dZQkFINlRPSUFVSk1SIiwKICAgICJyZWdpb24iOiAidXMtZWFzdC0yIgp9";
        let name = "output";

        let aws_info = aws_creds(key).unwrap();
        let creds = Credentials::new(aws_info.key, aws_info.secret);
        let mock_me_put = server.mock(|when, then| {
            when.method(PUT);
            then.status(200).header("ETAG", "whatever");
        });

        let bucket = Bucket::new(url, UrlStyle::VirtualHost, "blah", "us-east-1").unwrap();
        let etag_res = aws_multipart_upload(
            test.as_bytes(),
            "an id",
            &bucket,
            &creds,
            name,
            first_upload,
        )
        .unwrap();
        assert_eq!(etag_res.len(), 1);
        mock_me_put.assert();
    }

    #[test]
    fn test_aws_complete_multipart() {
        let server = MockServer::start();
        let port = server.port();

        let url: Url = format!("http://replacemeduh.com:{port}").parse().unwrap();
        let key = "ewogICAgImJ1Y2tldCI6ICJibGFoIiwKICAgICJzZWNyZXQiOiAicGtsNkFpQWFrL2JQcEdPenlGVW9DTC96SW1hSEoyTzVtR3ZzVWxSTCIsCiAgICAia2V5IjogIkFLSUEyT0dZQkFINlRPSUFVSk1SIiwKICAgICJyZWdpb24iOiAidXMtZWFzdC0yIgp9";
        let name = "output";

        let aws_info = aws_creds(key).unwrap();
        let creds = Credentials::new(aws_info.key, aws_info.secret);
        let mock_me = server.mock(|when, then| {
            when.method(POST);
            then.status(200);
        });

        let bucket = Bucket::new(url, UrlStyle::VirtualHost, "blah", "us-east-1").unwrap();
        aws_complete_multipart(&bucket, &creds, name, "myid", Vec::new()).unwrap();
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
