use super::error::RemoteError;
use crate::utils::{
    artemis_toml::Output, compression::compress_gzip_data, encoding::base64_decode_standard,
    time::time_now,
};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use log::{error, info, warn};
use reqwest::{blocking::Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Error;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadResponse {
    time_created: String,
    name: String,
}

/// Upload data to Google Cloud Storage Bucket using signed JWT tokens
pub(crate) fn gcp_upload(data: &[u8], output: &Output, filename: &str) -> Result<(), RemoteError> {
    // Grab URL which should include the target bucket
    let gcp_url = if let Some(url) = &output.url {
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

    let mut gcp_output = format!(
        "{}%2F{}%2F{filename}.{}",
        output.directory, output.name, output.format
    );
    let mut header_value = "application/json-seq";
    let output_data = if output.compress {
        gcp_output = format!("{gcp_output}.gz");
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

    let client = Client::new();
    // Full URL to target bucket and make upload resumable
    let session = &format!("{gcp_url}/o?uploadType=resumable&name={gcp_output}");

    // Create the signed JWT token
    let token = create_jwt_gcp(api_key)?;
    // Create the upload session
    let session_uri = gcp_session(session, &token)?;

    let mut builder = client.put(&session_uri);
    builder = builder.header("Content-Type", header_value);
    builder = builder.header("Content-Length", output_data.len());

    let res_result = builder.body(output_data.clone()).send();
    let res = match res_result {
        Ok(result) => result,
        Err(err) => {
            error!("[artemis-core] Failed to upload data to GCP storage: {err:?}");
            let attempt = 0;
            return gcp_resume_upload(&session_uri, &output_data, attempt);
        }
    };
    if res.status() != StatusCode::OK && res.status() != StatusCode::CREATED {
        error!(
            "[artemis-core] Non-200 response from GCP storage: {:?}",
            res.text()
        );
        let attempt = 0;
        return gcp_resume_upload(&session_uri, &output_data, attempt);
    }

    match res.bytes() {
        Ok(result) => {
            let upload_status: Result<UploadResponse, Error> = serde_json::from_slice(&result);
            match upload_status {
                Ok(status) => {
                    info!(
                        "[artemis-core] Uploaded {} at {}",
                        status.name, status.time_created
                    );
                }
                Err(err) => {
                    warn!("[artemis-core] Got non-standard upload response: {err:?}");
                }
            }
        }
        Err(err) => {
            warn!("[artemis-core] Could not get bytes of OK response: {err:?}");
        }
    }

    Ok(())
}

/// Create a resumable upload session
fn gcp_session(url: &str, token: &str) -> Result<String, RemoteError> {
    let client = Client::new();
    let mut builder = client.post(url).bearer_auth(token);
    builder = builder.header("Content-Length", 0);
    let res_result = builder.send();
    let res = match res_result {
        Ok(result) => result,
        Err(err) => {
            error!("[artemis-core] Failed to establish Google Cloud Session: {err:?}");
            return Err(RemoteError::RemoteUpload);
        }
    };
    if res.status() != StatusCode::OK {
        error!(
            "[artemis-core] Non-200 response from Google Cloud Session: {:?}",
            res.text()
        );
        return Err(RemoteError::BadResponse);
    }
    if let Some(location) = res.headers().get("Location") {
        let session_res = location.to_str();
        let session = match session_res {
            Ok(result) => result.to_string(),
            Err(err) => {
                error!("[artemis-core] Could not get Session URI string: {err:?}");
                return Err(RemoteError::BadResponse);
            }
        };
        return Ok(session);
    }

    error!("[artemis-core] No Location header in response");
    Err(RemoteError::BadResponse)
}

/// Attempt to resume a GCP upload. Will attempt to resume an upload 15 times
fn gcp_resume_upload(
    session_uri: &str,
    output_data: &[u8],
    max_attempts: u8,
) -> Result<(), RemoteError> {
    let max = 15;

    if max_attempts > max {
        error!("[artemis-core] Max attempts reached for uploading to Google Cloud");
        return Err(RemoteError::MaxAttempts);
    }
    let client = Client::new();
    let status = gcp_get_upload_status(session_uri, output_data.len())?;
    let complete = -1;
    if status == complete {
        return Ok(());
    }

    let data_remaining = output_data.len() - status as usize;

    let mut builder = client.put(session_uri);
    builder = builder.header("Content-Length", data_remaining);

    let range_adjust = 1;
    builder = builder.header(
        "Content-Range",
        format!(
            "bytes {}-{}/{}",
            (status + range_adjust),
            (output_data.len() - range_adjust as usize),
            output_data.len()
        ),
    );

    let output_left = output_data[status as usize + 1..output_data.len()].to_vec();

    let res_result = builder.body(output_left).send();
    let res = match res_result {
        Ok(result) => result,
        Err(err) => {
            error!("[artemis-core] Could not upload to GCP storage: {err:?}. Attempting again");
            let try_again: u8 = 1;
            let attempt = try_again + max_attempts;
            return gcp_resume_upload(session_uri, output_data, attempt);
        }
    };
    if res.status() != StatusCode::OK && res.status() != StatusCode::CREATED {
        error!(
            "[artemis-core] Non-200 response from GCP storage: {:?}. Attempting again",
            res.text()
        );
        let try_again: u8 = 1;
        let attempt = try_again + max_attempts;
        return gcp_resume_upload(session_uri, output_data, attempt);
    }

    Ok(())
}

/// Check the GCP upload status. A value of -1 means we are done
fn gcp_get_upload_status(url: &str, upload_size: usize) -> Result<isize, RemoteError> {
    let client = Client::new();
    let mut builder = client.put(url);
    builder = builder.header("Content-Length", 0);
    builder = builder.header("Content-Range", format!("bytes */{upload_size}"));

    let res_result = builder.send();
    let res = match res_result {
        Ok(result) => result,
        Err(err) => {
            error!("[artemis-core] Failed to check upload status: {err:?}");
            return Err(RemoteError::RemoteUpload);
        }
    };
    // Upload is done
    if res.status() == StatusCode::OK || res.status() == StatusCode::CREATED {
        let upload_ok = -1;
        return Ok(upload_ok);
    }

    if res.status() != StatusCode::PERMANENT_REDIRECT {
        error!(
            "[artemis-core] Unknown response received from Google Cloud when checking status: {:?}",
            res.text()
        );
        return Err(RemoteError::BadResponse);
    }

    if let Some(location) = res.headers().get("Range") {
        let session_res = location.to_str();
        let session = match session_res {
            Ok(result) => result.to_string(),
            Err(err) => {
                error!("[artemis-core] Could not get Session URI string: {err:?}");
                return Err(RemoteError::BadResponse);
            }
        };

        let upper_bytes: Vec<&str> = session.split('-').collect();
        let expected_len = 2;
        if upper_bytes.len() != expected_len {
            error!("[artemis-core] Unexpected Range header response: {session}");
            return Err(RemoteError::BadResponse);
        }

        let bytes_res = upper_bytes[1].parse::<isize>();
        let bytes = match bytes_res {
            Ok(results) => results,
            Err(err) => {
                error!("[artemis-core] Could not parse uploaded bytes status: {err:?}");
                return Err(RemoteError::BadResponse);
            }
        };
        return Ok(bytes);
    }

    // If range is not in the Header response then no bytes have been uploaded yet
    let no_bytes = 0;
    Ok(no_bytes)
}

#[derive(Deserialize)]
struct GcpKey {
    private_key_id: String,
    private_key: String,
    client_email: String,
}

#[derive(Serialize)]
struct JwtToken {
    iss: String,
    sub: String,
    scope: String,
    iat: u64,
    exp: u64,
}

/// Create a signed JWT token for remote uploads using service account
fn create_jwt_gcp(key: &str) -> Result<String, RemoteError> {
    let priv_key_result = base64_decode_standard(key);
    let priv_key = match priv_key_result {
        Ok(result) => result,
        Err(err) => {
            error!("[artemis-core] Could not base64 decode GCP key: {err:?}");
            return Err(RemoteError::RemoteApiKey);
        }
    };
    let gcp_key_result = serde_json::from_slice(&priv_key);
    let gcp_key: GcpKey = match gcp_key_result {
        Ok(result) => result,
        Err(err) => {
            error!("[artemis-core] Could not parse GCP key json: {err:?}");
            return Err(RemoteError::RemoteApiKey);
        }
    };

    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(gcp_key.private_key_id);

    let expire = 3600;
    let start = time_now();
    // We only want write permissions
    let payload = JwtToken {
        iss: gcp_key.client_email.clone(),
        sub: gcp_key.client_email,
        scope: String::from("https://www.googleapis.com/auth/devstorage.write_only"),
        iat: start,
        exp: start + expire,
    };

    let encoding_result = EncodingKey::from_rsa_pem(gcp_key.private_key.as_bytes());
    let encoding = match encoding_result {
        Ok(result) => result,
        Err(err) => {
            error!("[artemis-core] Could not creating encoding from Private Key: {err:?}");
            return Err(RemoteError::RemoteApiKey);
        }
    };
    let token_result = encode(&header, &payload, &encoding);
    let token = match token_result {
        Ok(result) => result,
        Err(err) => {
            error!("[artemis-core] Could not create token from encoding: {err:?}");
            return Err(RemoteError::RemoteApiKey);
        }
    };
    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::{create_jwt_gcp, gcp_get_upload_status, gcp_resume_upload};
    use crate::{
        output::remote::gcp::{gcp_session, gcp_upload},
        utils::artemis_toml::Output,
    };
    use httpmock::{
        Method::{POST, PUT},
        MockServer,
    };
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
            url: Some(format!("http://127.0.0.1:{port}")),
            api_key: Some(String::from("ewogICJ0eXBlIjogInNlcnZpY2VfYWNjb3VudCIsCiAgInByb2plY3RfaWQiOiAiZmFrZW1lIiwKICAicHJpdmF0ZV9rZXlfaWQiOiAiZmFrZW1lIiwKICAicHJpdmF0ZV9rZXkiOiAiLS0tLS1CRUdJTiBQUklWQVRFIEtFWS0tLS0tXG5NSUlFdndJQkFEQU5CZ2txaGtpRzl3MEJBUUVGQUFTQ0JLa3dnZ1NsQWdFQUFvSUJBUUM3VkpUVXQ5VXM4Y0tqTXpFZll5amlXQTRSNC9NMmJTMUdCNHQ3TlhwOThDM1NDNmRWTXZEdWljdEdldXJUOGpOYnZKWkh0Q1N1WUV2dU5Nb1NmbTc2b3FGdkFwOEd5MGl6NXN4alptU25YeUNkUEVvdkdoTGEwVnpNYVE4cytDTE95UzU2WXlDRkdlSlpxZ3R6SjZHUjNlcW9ZU1c5YjlVTXZrQnBaT0RTY3RXU05HajNQN2pSRkRPNVZvVHdDUUFXYkZuT2pEZkg1VWxncDJQS1NRblNKUDNBSkxRTkZOZTdicjFYYnJoVi8vZU8rdDUxbUlwR1NEQ1V2M0UwRERGY1dEVEg5Y1hEVFRsUlpWRWlSMkJ3cFpPT2tFL1owL0JWbmhaWUw3MW9aVjM0YktmV2pRSXQ2Vi9pc1NNYWhkc0FBU0FDcDRaVEd0d2lWdU5kOXR5YkFnTUJBQUVDZ2dFQkFLVG1qYVM2dGtLOEJsUFhDbFRRMnZwei9ONnV4RGVTMzVtWHBxYXNxc2tWbGFBaWRnZy9zV3FwalhEYlhyOTNvdElNTGxXc00rWDBDcU1EZ1NYS2VqTFMyang0R0RqSTFaVFhnKyswQU1KOHNKNzRwV3pWRE9mbUNFUS83d1hzMytjYm5YaEtyaU84WjAzNnE5MlFjMStOODdTSTM4bmtHYTBBQkg5Q044M0htUXF0NGZCN1VkSHp1SVJlL21lMlBHaElxNVpCemo2aDNCcG9QR3pFUCt4M2w5WW1LOHQvMWNOMHBxSStkUXdZZGdmR2phY2tMdS8ycUg4ME1DRjdJeVFhc2VaVU9KeUtyQ0x0U0QvSWl4di9oekRFVVBmT0NqRkRnVHB6ZjNjd3RhOCtvRTR3SENvMWlJMS80VGxQa3dtWHg0cVNYdG13NGFRUHo3SURRdkVDZ1lFQThLTlRoQ08yZ3NDMkk5UFFETS84Q3cwTzk4M1dDRFkrb2krN0pQaU5BSnd2NURZQnFFWkIxUVlkajA2WUQxNlhsQy9IQVpNc01rdTFuYTJUTjBkcml3ZW5RUVd6b2V2M2cyUzdnUkRvUy9GQ0pTSTNqSitramd0YUE3UW16bGdrMVR4T0ROK0cxSDkxSFc3dDBsN1ZuTDI3SVd5WW8ycVJSSzNqenhxVWlQVUNnWUVBeDBvUXMycmVCUUdNVlpuQXBEMWplcTduNE12TkxjUHZ0OGIvZVU5aVV2Nlk0TWowU3VvL0FVOGxZWlhtOHViYnFBbHd6MlZTVnVuRDJ0T3BsSHlNVXJ0Q3RPYkFmVkRVQWhDbmRLYUE5Z0FwZ2ZiM3h3MUlLYnVRMXU0SUYxRkpsM1Z0dW1mUW4vL0xpSDFCM3JYaGNkeW8zL3ZJdHRFazQ4UmFrVUtDbFU4Q2dZRUF6VjdXM0NPT2xERGNRZDkzNURkdEtCRlJBUFJQQWxzcFFVbnpNaTVlU0hNRC9JU0xEWTVJaVFIYklIODNENGJ2WHEwWDdxUW9TQlNOUDdEdnYzSFl1cU1oZjBEYWVncmxCdUpsbEZWVnE5cVBWUm5LeHQxSWwySGd4T0J2YmhPVCs5aW4xQnpBK1lKOTlVekM4NU8wUXowNkErQ210SEV5NGFaMmtqNWhIakVDZ1lFQW1OUzQrQThGa3NzOEpzMVJpZUsyTG5pQnhNZ21ZbWwzcGZWTEtHbnptbmc3SDIrY3dQTGhQSXpJdXd5dFh5d2gyYnpic1lFZll4M0VvRVZnTUVwUGhvYXJRbllQdWtySk80Z3dFMm81VGU2VDVtSlNaR2xRSlFqOXE0WkIyRGZ6ZXQ2SU5zSzBvRzhYVkdYU3BRdlFoM1JVWWVrQ1pRa0JCRmNwcVdwYklFc0NnWUFuTTNEUWYzRkpvU25YYU1oclZCSW92aWM1bDB4RmtFSHNrQWpGVGV2Tzg2RnN6MUMyYVNlUktTcUdGb09RMHRtSnpCRXMxUjZLcW5ISW5pY0RUUXJLaEFyZ0xYWDR2M0NkZGpmVFJKa0ZXRGJFL0NrdktaTk9yY2YxbmhhR0NQc3BSSmoyS1VrajFGaGw5Q25jZG4vUnNZRU9OYndRU2pJZk1Qa3Z4Ris4SFE9PVxuLS0tLS1FTkQgUFJJVkFURSBLRVktLS0tLVxuIiwKICAiY2xpZW50X2VtYWlsIjogImZha2VAZ3NlcnZpY2VhY2NvdW50LmNvbSIsCiAgImNsaWVudF9pZCI6ICJmYWtlbWUiLAogICJhdXRoX3VyaSI6ICJodHRwczovL2FjY291bnRzLmdvb2dsZS5jb20vby9vYXV0aDIvYXV0aCIsCiAgInRva2VuX3VyaSI6ICJodHRwczovL29hdXRoMi5nb29nbGVhcGlzLmNvbS90b2tlbiIsCiAgImF1dGhfcHJvdmlkZXJfeDUwOV9jZXJ0X3VybCI6ICJodHRwczovL3d3dy5nb29nbGVhcGlzLmNvbS9vYXV0aDIvdjEvY2VydHMiLAogICJjbGllbnRfeDUwOV9jZXJ0X3VybCI6ICJodHRwczovL3d3dy5nb29nbGVhcGlzLmNvbS9yb2JvdC92MS9tZXRhZGF0YS94NTA5L2Zha2VtZSIsCiAgInVuaXZlcnNlX2RvbWFpbiI6ICJnb29nbGVhcGlzLmNvbSIKfQo=")),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
        }
    }

    #[test]
    fn test_upload_gcp() {
        let server = MockServer::start();
        let port = server.port();
        let output = output_options("gcp_upload_test", "gcp", "tmp", false, port);

        let mock_me = server.mock(|when, then| {
            when.method(POST);
            then.status(200)
                .header("content-type", "application/json")
                .header("Location", format!("http://127.0.0.1:{port}"))
                .json_body(json!({ "timeCreated": "whatever", "name":"mockme" }));
        });
        let test = "A rust program";
        let name = "output";
        let mock_me_put = server.mock(|when, then| {
            when.method(PUT);
            then.status(200)
                .header("content-type", "application/json")
                .header("Location", format!("http://127.0.0.1:{port}"))
                .json_body(json!({ "timeCreated": "whatever", "name":"mockme" }));
        });
        gcp_upload(test.as_bytes(), &output, name).unwrap();
        mock_me.assert();
        mock_me_put.assert();
    }

    #[test]
    fn test_gcp_session() {
        // This is a "real" key made at jwt.io with fake GCP data
        let test = "ewogICJ0eXBlIjogInNlcnZpY2VfYWNjb3VudCIsCiAgInByb2plY3RfaWQiOiAiZmFrZW1lIiwKICAicHJpdmF0ZV9rZXlfaWQiOiAiZmFrZW1lIiwKICAicHJpdmF0ZV9rZXkiOiAiLS0tLS1CRUdJTiBQUklWQVRFIEtFWS0tLS0tXG5NSUlFdndJQkFEQU5CZ2txaGtpRzl3MEJBUUVGQUFTQ0JLa3dnZ1NsQWdFQUFvSUJBUUM3VkpUVXQ5VXM4Y0tqTXpFZll5amlXQTRSNC9NMmJTMUdCNHQ3TlhwOThDM1NDNmRWTXZEdWljdEdldXJUOGpOYnZKWkh0Q1N1WUV2dU5Nb1NmbTc2b3FGdkFwOEd5MGl6NXN4alptU25YeUNkUEVvdkdoTGEwVnpNYVE4cytDTE95UzU2WXlDRkdlSlpxZ3R6SjZHUjNlcW9ZU1c5YjlVTXZrQnBaT0RTY3RXU05HajNQN2pSRkRPNVZvVHdDUUFXYkZuT2pEZkg1VWxncDJQS1NRblNKUDNBSkxRTkZOZTdicjFYYnJoVi8vZU8rdDUxbUlwR1NEQ1V2M0UwRERGY1dEVEg5Y1hEVFRsUlpWRWlSMkJ3cFpPT2tFL1owL0JWbmhaWUw3MW9aVjM0YktmV2pRSXQ2Vi9pc1NNYWhkc0FBU0FDcDRaVEd0d2lWdU5kOXR5YkFnTUJBQUVDZ2dFQkFLVG1qYVM2dGtLOEJsUFhDbFRRMnZwei9ONnV4RGVTMzVtWHBxYXNxc2tWbGFBaWRnZy9zV3FwalhEYlhyOTNvdElNTGxXc00rWDBDcU1EZ1NYS2VqTFMyang0R0RqSTFaVFhnKyswQU1KOHNKNzRwV3pWRE9mbUNFUS83d1hzMytjYm5YaEtyaU84WjAzNnE5MlFjMStOODdTSTM4bmtHYTBBQkg5Q044M0htUXF0NGZCN1VkSHp1SVJlL21lMlBHaElxNVpCemo2aDNCcG9QR3pFUCt4M2w5WW1LOHQvMWNOMHBxSStkUXdZZGdmR2phY2tMdS8ycUg4ME1DRjdJeVFhc2VaVU9KeUtyQ0x0U0QvSWl4di9oekRFVVBmT0NqRkRnVHB6ZjNjd3RhOCtvRTR3SENvMWlJMS80VGxQa3dtWHg0cVNYdG13NGFRUHo3SURRdkVDZ1lFQThLTlRoQ08yZ3NDMkk5UFFETS84Q3cwTzk4M1dDRFkrb2krN0pQaU5BSnd2NURZQnFFWkIxUVlkajA2WUQxNlhsQy9IQVpNc01rdTFuYTJUTjBkcml3ZW5RUVd6b2V2M2cyUzdnUkRvUy9GQ0pTSTNqSitramd0YUE3UW16bGdrMVR4T0ROK0cxSDkxSFc3dDBsN1ZuTDI3SVd5WW8ycVJSSzNqenhxVWlQVUNnWUVBeDBvUXMycmVCUUdNVlpuQXBEMWplcTduNE12TkxjUHZ0OGIvZVU5aVV2Nlk0TWowU3VvL0FVOGxZWlhtOHViYnFBbHd6MlZTVnVuRDJ0T3BsSHlNVXJ0Q3RPYkFmVkRVQWhDbmRLYUE5Z0FwZ2ZiM3h3MUlLYnVRMXU0SUYxRkpsM1Z0dW1mUW4vL0xpSDFCM3JYaGNkeW8zL3ZJdHRFazQ4UmFrVUtDbFU4Q2dZRUF6VjdXM0NPT2xERGNRZDkzNURkdEtCRlJBUFJQQWxzcFFVbnpNaTVlU0hNRC9JU0xEWTVJaVFIYklIODNENGJ2WHEwWDdxUW9TQlNOUDdEdnYzSFl1cU1oZjBEYWVncmxCdUpsbEZWVnE5cVBWUm5LeHQxSWwySGd4T0J2YmhPVCs5aW4xQnpBK1lKOTlVekM4NU8wUXowNkErQ210SEV5NGFaMmtqNWhIakVDZ1lFQW1OUzQrQThGa3NzOEpzMVJpZUsyTG5pQnhNZ21ZbWwzcGZWTEtHbnptbmc3SDIrY3dQTGhQSXpJdXd5dFh5d2gyYnpic1lFZll4M0VvRVZnTUVwUGhvYXJRbllQdWtySk80Z3dFMm81VGU2VDVtSlNaR2xRSlFqOXE0WkIyRGZ6ZXQ2SU5zSzBvRzhYVkdYU3BRdlFoM1JVWWVrQ1pRa0JCRmNwcVdwYklFc0NnWUFuTTNEUWYzRkpvU25YYU1oclZCSW92aWM1bDB4RmtFSHNrQWpGVGV2Tzg2RnN6MUMyYVNlUktTcUdGb09RMHRtSnpCRXMxUjZLcW5ISW5pY0RUUXJLaEFyZ0xYWDR2M0NkZGpmVFJKa0ZXRGJFL0NrdktaTk9yY2YxbmhhR0NQc3BSSmoyS1VrajFGaGw5Q25jZG4vUnNZRU9OYndRU2pJZk1Qa3Z4Ris4SFE9PVxuLS0tLS1FTkQgUFJJVkFURSBLRVktLS0tLVxuIiwKICAiY2xpZW50X2VtYWlsIjogImZha2VAZ3NlcnZpY2VhY2NvdW50LmNvbSIsCiAgImNsaWVudF9pZCI6ICJmYWtlbWUiLAogICJhdXRoX3VyaSI6ICJodHRwczovL2FjY291bnRzLmdvb2dsZS5jb20vby9vYXV0aDIvYXV0aCIsCiAgInRva2VuX3VyaSI6ICJodHRwczovL29hdXRoMi5nb29nbGVhcGlzLmNvbS90b2tlbiIsCiAgImF1dGhfcHJvdmlkZXJfeDUwOV9jZXJ0X3VybCI6ICJodHRwczovL3d3dy5nb29nbGVhcGlzLmNvbS9vYXV0aDIvdjEvY2VydHMiLAogICJjbGllbnRfeDUwOV9jZXJ0X3VybCI6ICJodHRwczovL3d3dy5nb29nbGVhcGlzLmNvbS9yb2JvdC92MS9tZXRhZGF0YS94NTA5L2Zha2VtZSIsCiAgInVuaXZlcnNlX2RvbWFpbiI6ICJnb29nbGVhcGlzLmNvbSIKfQo=";
        let result = create_jwt_gcp(test).unwrap();
        assert!(!result.is_empty());

        let server = MockServer::start();
        let port = server.port();
        let mock_me = server.mock(|when, then| {
            when.method(POST);
            then.status(200)
                .header("content-type", "application/json")
                .header("Location", format!("http://127.0.0.1:{port}"))
                .json_body(json!({ "timeCreated": "whatever", "name":"mockme" }));
        });

        let session = gcp_session(&format!("http://127.0.0.1:{port}"), &result).unwrap();
        mock_me.assert();

        assert_eq!(session, format!("http://127.0.0.1:{port}"))
    }

    #[test]
    fn test_gcp_resume_upload() {
        let server = MockServer::start();
        let port = server.port();
        let mock_me_resume = server.mock(|when, then| {
            when.method(PUT)
                .header_exists("Content-Length")
                .header("Content-Range", "bytes 3-4/5");
            then.status(200)
                .json_body(json!({ "timeCreated": "whatever", "name":"mockme" }));
        });
        let mock_me = server.mock(|when, then| {
            when.method(PUT)
                .header("Content-Range", "bytes */5")
                .header("Content-Length", "0");
            then.status(308)
                .header("Range", "0-2")
                .json_body(json!({ "timeCreated": "whatever", "name":"mockme" }));
        });

        let data = [0, 1, 2, 3, 4];

        gcp_resume_upload(&format!("http://127.0.0.1:{port}"), &data, 0).unwrap();
        mock_me.assert();
        mock_me_resume.assert();
    }

    #[test]
    #[should_panic(expected = "MaxAttempts")]
    fn test_gcp_resume_upload_max_attempts() {
        let server = MockServer::start();
        let port = server.port();
        let mock_me = server.mock(|when, then| {
            when.method(PUT);
            then.status(308)
                .header("Range", "0-2")
                .json_body(json!({ "timeCreated": "whatever", "name":"mockme" }));
        });
        let data = [0, 1, 2, 3, 4];

        gcp_resume_upload(&format!("http://127.0.0.1:{port}"), &data, 0).unwrap();
        mock_me.assert();
    }

    #[test]
    fn test_gcp_get_upload_status() {
        let server = MockServer::start();
        let port = server.port();
        let mock_me = server.mock(|when, then| {
            when.method(PUT);
            then.status(308)
                .header("Range", "0-5")
                .json_body(json!({ "timeCreated": "whatever", "name":"mockme" }));
        });

        let size = gcp_get_upload_status(&format!("http://127.0.0.1:{port}"), 10).unwrap();
        mock_me.assert();

        assert_eq!(size, 5);
    }

    #[test]
    fn test_gcp_get_upload_status_done() {
        let server = MockServer::start();
        let port = server.port();
        let mock_me = server.mock(|when, then| {
            when.method(PUT);
            then.status(200)
                .json_body(json!({ "timeCreated": "whatever", "name":"mockme" }));
        });

        let size = gcp_get_upload_status(&format!("http://127.0.0.1:{port}"), 10).unwrap();
        mock_me.assert();

        assert_eq!(size, -1);
    }

    #[test]
    fn test_create_jwt_gcp() {
        // This is a "real" key made at jwt.io with fake GCP data
        let test = "ewogICJ0eXBlIjogInNlcnZpY2VfYWNjb3VudCIsCiAgInByb2plY3RfaWQiOiAiZmFrZW1lIiwKICAicHJpdmF0ZV9rZXlfaWQiOiAiZmFrZW1lIiwKICAicHJpdmF0ZV9rZXkiOiAiLS0tLS1CRUdJTiBQUklWQVRFIEtFWS0tLS0tXG5NSUlFdndJQkFEQU5CZ2txaGtpRzl3MEJBUUVGQUFTQ0JLa3dnZ1NsQWdFQUFvSUJBUUM3VkpUVXQ5VXM4Y0tqTXpFZll5amlXQTRSNC9NMmJTMUdCNHQ3TlhwOThDM1NDNmRWTXZEdWljdEdldXJUOGpOYnZKWkh0Q1N1WUV2dU5Nb1NmbTc2b3FGdkFwOEd5MGl6NXN4alptU25YeUNkUEVvdkdoTGEwVnpNYVE4cytDTE95UzU2WXlDRkdlSlpxZ3R6SjZHUjNlcW9ZU1c5YjlVTXZrQnBaT0RTY3RXU05HajNQN2pSRkRPNVZvVHdDUUFXYkZuT2pEZkg1VWxncDJQS1NRblNKUDNBSkxRTkZOZTdicjFYYnJoVi8vZU8rdDUxbUlwR1NEQ1V2M0UwRERGY1dEVEg5Y1hEVFRsUlpWRWlSMkJ3cFpPT2tFL1owL0JWbmhaWUw3MW9aVjM0YktmV2pRSXQ2Vi9pc1NNYWhkc0FBU0FDcDRaVEd0d2lWdU5kOXR5YkFnTUJBQUVDZ2dFQkFLVG1qYVM2dGtLOEJsUFhDbFRRMnZwei9ONnV4RGVTMzVtWHBxYXNxc2tWbGFBaWRnZy9zV3FwalhEYlhyOTNvdElNTGxXc00rWDBDcU1EZ1NYS2VqTFMyang0R0RqSTFaVFhnKyswQU1KOHNKNzRwV3pWRE9mbUNFUS83d1hzMytjYm5YaEtyaU84WjAzNnE5MlFjMStOODdTSTM4bmtHYTBBQkg5Q044M0htUXF0NGZCN1VkSHp1SVJlL21lMlBHaElxNVpCemo2aDNCcG9QR3pFUCt4M2w5WW1LOHQvMWNOMHBxSStkUXdZZGdmR2phY2tMdS8ycUg4ME1DRjdJeVFhc2VaVU9KeUtyQ0x0U0QvSWl4di9oekRFVVBmT0NqRkRnVHB6ZjNjd3RhOCtvRTR3SENvMWlJMS80VGxQa3dtWHg0cVNYdG13NGFRUHo3SURRdkVDZ1lFQThLTlRoQ08yZ3NDMkk5UFFETS84Q3cwTzk4M1dDRFkrb2krN0pQaU5BSnd2NURZQnFFWkIxUVlkajA2WUQxNlhsQy9IQVpNc01rdTFuYTJUTjBkcml3ZW5RUVd6b2V2M2cyUzdnUkRvUy9GQ0pTSTNqSitramd0YUE3UW16bGdrMVR4T0ROK0cxSDkxSFc3dDBsN1ZuTDI3SVd5WW8ycVJSSzNqenhxVWlQVUNnWUVBeDBvUXMycmVCUUdNVlpuQXBEMWplcTduNE12TkxjUHZ0OGIvZVU5aVV2Nlk0TWowU3VvL0FVOGxZWlhtOHViYnFBbHd6MlZTVnVuRDJ0T3BsSHlNVXJ0Q3RPYkFmVkRVQWhDbmRLYUE5Z0FwZ2ZiM3h3MUlLYnVRMXU0SUYxRkpsM1Z0dW1mUW4vL0xpSDFCM3JYaGNkeW8zL3ZJdHRFazQ4UmFrVUtDbFU4Q2dZRUF6VjdXM0NPT2xERGNRZDkzNURkdEtCRlJBUFJQQWxzcFFVbnpNaTVlU0hNRC9JU0xEWTVJaVFIYklIODNENGJ2WHEwWDdxUW9TQlNOUDdEdnYzSFl1cU1oZjBEYWVncmxCdUpsbEZWVnE5cVBWUm5LeHQxSWwySGd4T0J2YmhPVCs5aW4xQnpBK1lKOTlVekM4NU8wUXowNkErQ210SEV5NGFaMmtqNWhIakVDZ1lFQW1OUzQrQThGa3NzOEpzMVJpZUsyTG5pQnhNZ21ZbWwzcGZWTEtHbnptbmc3SDIrY3dQTGhQSXpJdXd5dFh5d2gyYnpic1lFZll4M0VvRVZnTUVwUGhvYXJRbllQdWtySk80Z3dFMm81VGU2VDVtSlNaR2xRSlFqOXE0WkIyRGZ6ZXQ2SU5zSzBvRzhYVkdYU3BRdlFoM1JVWWVrQ1pRa0JCRmNwcVdwYklFc0NnWUFuTTNEUWYzRkpvU25YYU1oclZCSW92aWM1bDB4RmtFSHNrQWpGVGV2Tzg2RnN6MUMyYVNlUktTcUdGb09RMHRtSnpCRXMxUjZLcW5ISW5pY0RUUXJLaEFyZ0xYWDR2M0NkZGpmVFJKa0ZXRGJFL0NrdktaTk9yY2YxbmhhR0NQc3BSSmoyS1VrajFGaGw5Q25jZG4vUnNZRU9OYndRU2pJZk1Qa3Z4Ris4SFE9PVxuLS0tLS1FTkQgUFJJVkFURSBLRVktLS0tLVxuIiwKICAiY2xpZW50X2VtYWlsIjogImZha2VAZ3NlcnZpY2VhY2NvdW50LmNvbSIsCiAgImNsaWVudF9pZCI6ICJmYWtlbWUiLAogICJhdXRoX3VyaSI6ICJodHRwczovL2FjY291bnRzLmdvb2dsZS5jb20vby9vYXV0aDIvYXV0aCIsCiAgInRva2VuX3VyaSI6ICJodHRwczovL29hdXRoMi5nb29nbGVhcGlzLmNvbS90b2tlbiIsCiAgImF1dGhfcHJvdmlkZXJfeDUwOV9jZXJ0X3VybCI6ICJodHRwczovL3d3dy5nb29nbGVhcGlzLmNvbS9vYXV0aDIvdjEvY2VydHMiLAogICJjbGllbnRfeDUwOV9jZXJ0X3VybCI6ICJodHRwczovL3d3dy5nb29nbGVhcGlzLmNvbS9yb2JvdC92MS9tZXRhZGF0YS94NTA5L2Zha2VtZSIsCiAgInVuaXZlcnNlX2RvbWFpbiI6ICJnb29nbGVhcGlzLmNvbSIKfQo=";
        let result = create_jwt_gcp(test).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_upload_gcp_compress() {
        let server = MockServer::start();
        let port = server.port();

        let output = output_options("gcp_upload_test", "gcp", "tmp", false, port);

        let mock_me = server.mock(|when, then| {
            when.method(POST);
            then.status(200)
                .header("content-type", "application/json")
                .header("Location", format!("http://127.0.0.1:{port}"))
                .json_body(json!({ "timeCreated": "whatever", "name":"mockme" }));
        });
        let test = "A rust program";
        let name = "output";
        let mock_me_put = server.mock(|when, then| {
            when.method(PUT);
            then.status(200)
                .header("content-type", "application/json")
                .header("Location", format!("http://127.0.0.1:{port}"))
                .json_body(json!({ "timeCreated": "whatever", "name":"mockme" }));
        });
        gcp_upload(test.as_bytes(), &output, name).unwrap();
        mock_me.assert();
        mock_me_put.assert();
    }

    #[test]
    #[should_panic(expected = "RemoteUpload")]
    fn test_bad_upload_gcp() {
        let output = Output {
            name: String::from("test_output"),
            directory: String::from("upload"),
            format: String::from("gcp"),
            compress: false,
            url: Some(String::from("http://127.0.0.1:2223")),
            api_key: Some(String::from("ewogICJ0eXBlIjogInNlcnZpY2VfYWNjb3VudCIsCiAgInByb2plY3RfaWQiOiAiZmFrZW1lIiwKICAicHJpdmF0ZV9rZXlfaWQiOiAiZmFrZW1lIiwKICAicHJpdmF0ZV9rZXkiOiAiLS0tLS1CRUdJTiBQUklWQVRFIEtFWS0tLS0tXG5NSUlFdndJQkFEQU5CZ2txaGtpRzl3MEJBUUVGQUFTQ0JLa3dnZ1NsQWdFQUFvSUJBUUM3VkpUVXQ5VXM4Y0tqTXpFZll5amlXQTRSNC9NMmJTMUdCNHQ3TlhwOThDM1NDNmRWTXZEdWljdEdldXJUOGpOYnZKWkh0Q1N1WUV2dU5Nb1NmbTc2b3FGdkFwOEd5MGl6NXN4alptU25YeUNkUEVvdkdoTGEwVnpNYVE4cytDTE95UzU2WXlDRkdlSlpxZ3R6SjZHUjNlcW9ZU1c5YjlVTXZrQnBaT0RTY3RXU05HajNQN2pSRkRPNVZvVHdDUUFXYkZuT2pEZkg1VWxncDJQS1NRblNKUDNBSkxRTkZOZTdicjFYYnJoVi8vZU8rdDUxbUlwR1NEQ1V2M0UwRERGY1dEVEg5Y1hEVFRsUlpWRWlSMkJ3cFpPT2tFL1owL0JWbmhaWUw3MW9aVjM0YktmV2pRSXQ2Vi9pc1NNYWhkc0FBU0FDcDRaVEd0d2lWdU5kOXR5YkFnTUJBQUVDZ2dFQkFLVG1qYVM2dGtLOEJsUFhDbFRRMnZwei9ONnV4RGVTMzVtWHBxYXNxc2tWbGFBaWRnZy9zV3FwalhEYlhyOTNvdElNTGxXc00rWDBDcU1EZ1NYS2VqTFMyang0R0RqSTFaVFhnKyswQU1KOHNKNzRwV3pWRE9mbUNFUS83d1hzMytjYm5YaEtyaU84WjAzNnE5MlFjMStOODdTSTM4bmtHYTBBQkg5Q044M0htUXF0NGZCN1VkSHp1SVJlL21lMlBHaElxNVpCemo2aDNCcG9QR3pFUCt4M2w5WW1LOHQvMWNOMHBxSStkUXdZZGdmR2phY2tMdS8ycUg4ME1DRjdJeVFhc2VaVU9KeUtyQ0x0U0QvSWl4di9oekRFVVBmT0NqRkRnVHB6ZjNjd3RhOCtvRTR3SENvMWlJMS80VGxQa3dtWHg0cVNYdG13NGFRUHo3SURRdkVDZ1lFQThLTlRoQ08yZ3NDMkk5UFFETS84Q3cwTzk4M1dDRFkrb2krN0pQaU5BSnd2NURZQnFFWkIxUVlkajA2WUQxNlhsQy9IQVpNc01rdTFuYTJUTjBkcml3ZW5RUVd6b2V2M2cyUzdnUkRvUy9GQ0pTSTNqSitramd0YUE3UW16bGdrMVR4T0ROK0cxSDkxSFc3dDBsN1ZuTDI3SVd5WW8ycVJSSzNqenhxVWlQVUNnWUVBeDBvUXMycmVCUUdNVlpuQXBEMWplcTduNE12TkxjUHZ0OGIvZVU5aVV2Nlk0TWowU3VvL0FVOGxZWlhtOHViYnFBbHd6MlZTVnVuRDJ0T3BsSHlNVXJ0Q3RPYkFmVkRVQWhDbmRLYUE5Z0FwZ2ZiM3h3MUlLYnVRMXU0SUYxRkpsM1Z0dW1mUW4vL0xpSDFCM3JYaGNkeW8zL3ZJdHRFazQ4UmFrVUtDbFU4Q2dZRUF6VjdXM0NPT2xERGNRZDkzNURkdEtCRlJBUFJQQWxzcFFVbnpNaTVlU0hNRC9JU0xEWTVJaVFIYklIODNENGJ2WHEwWDdxUW9TQlNOUDdEdnYzSFl1cU1oZjBEYWVncmxCdUpsbEZWVnE5cVBWUm5LeHQxSWwySGd4T0J2YmhPVCs5aW4xQnpBK1lKOTlVekM4NU8wUXowNkErQ210SEV5NGFaMmtqNWhIakVDZ1lFQW1OUzQrQThGa3NzOEpzMVJpZUsyTG5pQnhNZ21ZbWwzcGZWTEtHbnptbmc3SDIrY3dQTGhQSXpJdXd5dFh5d2gyYnpic1lFZll4M0VvRVZnTUVwUGhvYXJRbllQdWtySk80Z3dFMm81VGU2VDVtSlNaR2xRSlFqOXE0WkIyRGZ6ZXQ2SU5zSzBvRzhYVkdYU3BRdlFoM1JVWWVrQ1pRa0JCRmNwcVdwYklFc0NnWUFuTTNEUWYzRkpvU25YYU1oclZCSW92aWM1bDB4RmtFSHNrQWpGVGV2Tzg2RnN6MUMyYVNlUktTcUdGb09RMHRtSnpCRXMxUjZLcW5ISW5pY0RUUXJLaEFyZ0xYWDR2M0NkZGpmVFJKa0ZXRGJFL0NrdktaTk9yY2YxbmhhR0NQc3BSSmoyS1VrajFGaGw5Q25jZG4vUnNZRU9OYndRU2pJZk1Qa3Z4Ris4SFE9PVxuLS0tLS1FTkQgUFJJVkFURSBLRVktLS0tLVxuIiwKICAiY2xpZW50X2VtYWlsIjogImZha2VAZ3NlcnZpY2VhY2NvdW50LmNvbSIsCiAgImNsaWVudF9pZCI6ICJmYWtlbWUiLAogICJhdXRoX3VyaSI6ICJodHRwczovL2FjY291bnRzLmdvb2dsZS5jb20vby9vYXV0aDIvYXV0aCIsCiAgInRva2VuX3VyaSI6ICJodHRwczovL29hdXRoMi5nb29nbGVhcGlzLmNvbS90b2tlbiIsCiAgImF1dGhfcHJvdmlkZXJfeDUwOV9jZXJ0X3VybCI6ICJodHRwczovL3d3dy5nb29nbGVhcGlzLmNvbS9vYXV0aDIvdjEvY2VydHMiLAogICJjbGllbnRfeDUwOV9jZXJ0X3VybCI6ICJodHRwczovL3d3dy5nb29nbGVhcGlzLmNvbS9yb2JvdC92MS9tZXRhZGF0YS94NTA5L2Zha2VtZSIsCiAgInVuaXZlcnNlX2RvbWFpbiI6ICJnb29nbGVhcGlzLmNvbSIKfQo=")),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: String::from("local"),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
        };

        let test = "A rust program";
        let name = "output";
        gcp_upload(test.as_bytes(), &output, name).unwrap();
    }

    #[test]
    #[should_panic(expected = "BadResponse")]
    fn test_upload_gcp_non_ok() {
        let server = MockServer::start();
        let output = output_options("gcp_upload_test", "gcp", "tmp", false, server.port());

        let mock_me = server.mock(|when, then| {
            when.method(POST);
            then.status(500)
                .header("content-type", "application/json")
                .json_body(json!({ "bad": "rust" }));
        });
        let test = "A rust program";
        let name = "output";
        gcp_upload(test.as_bytes(), &output, name).unwrap();
        mock_me.assert();
    }
}
