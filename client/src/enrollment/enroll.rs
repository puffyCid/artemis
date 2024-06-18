use super::{error::EnrollError, info::gather_info};
use crate::filesystem::config::create_layout;
use common::server::config::ArtemisConfig;
use common::server::enrollment::{EnrollSystem, EnrollmentResponse};
use log::error;
use reqwest::{
    header::{HeaderMap, CONTENT_TYPE, USER_AGENT},
    ClientBuilder, Response, StatusCode,
};
use serde_json::Value;

/// Enroll the system into the server defined in the config
pub(crate) async fn enroll_client(config: &mut ArtemisConfig) -> Result<(), EnrollError> {
    let client_result = ClientBuilder::new()
        .danger_accept_invalid_certs(!config.endpoint_server.verify_ssl)
        .build();

    let client = match client_result {
        Ok(result) => result,
        Err(err) => {
            error!("[client] Could not create enroll client: {err:?}");
            return Err(EnrollError::Enroll);
        }
    };

    let endpoint_info = gather_info();
    let enroll_info = EnrollSystem {
        enroll_key: config.enroll_key.clone(),
        enrollment_info: endpoint_info,
    };

    let data_result = serde_json::to_vec(&enroll_info);
    let data = match data_result {
        Ok(result) => result,
        Err(err) => {
            error!("[client] Could not serialize enroll client: {err:?}");
            return Err(EnrollError::EnrollSerialize);
        }
    };

    let mut builder = client.post(format!(
        "http://{}:{}/endpoint/v{}/enroll",
        config.endpoint_server.address, config.endpoint_server.port, config.endpoint_server.version
    ));

    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
            .parse()
            .unwrap(),
    );
    headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());

    // Add User-Agent
    builder = builder.headers(headers);

    builder = builder.body(data);
    let res_result = builder.send().await;
    let response = match res_result {
        Ok(result) => result,
        Err(err) => {
            error!("[client] Could not send enroll request: {err:?}");
            return Err(EnrollError::EnrollRequest);
        }
    };

    config.endpoint_id = enroll_response(response).await?;

    let status = create_layout(config).await;
    if status.is_err() {
        error!(
            "[client] Could not create client from enrollment: {:?}",
            status.unwrap_err()
        );
        return Err(EnrollError::CreateLayout);
    }
    Ok(())
}

/// Parse the enrollment response
async fn enroll_response(response: Response) -> Result<String, EnrollError> {
    if response.status() != StatusCode::OK {
        error!(
            "[client] Non-200 response {}: {:?}",
            response.status(),
            response.json::<Value>().await.unwrap_or_default()
        );
        return Err(EnrollError::EnrollBadResponse);
    }

    let data_result = response.json::<EnrollmentResponse>().await;
    let data = match data_result {
        Ok(result) => result,
        Err(err) => {
            error!("[client] Could not deserialize enroll response: {err:?}");
            return Err(EnrollError::EnrollDeserialize);
        }
    };

    Ok(data.endpoint_id)
}

#[cfg(test)]
mod tests {
    use crate::{
        enrollment::{
            enroll::{enroll_client, enroll_response},
            info::gather_info,
        },
        filesystem::config::read_config,
    };
    use common::server::enrollment::EnrollSystem;
    use httpmock::{Method::POST, MockServer};
    use reqwest::{
        header::{HeaderMap, CONTENT_TYPE, USER_AGENT},
        ClientBuilder,
    };
    use serde_json::json;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_enroll_client() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/client.toml");

        let mut config = read_config(&test_location.display().to_string())
            .await
            .unwrap();

        let server = MockServer::start();
        config.endpoint_server.port = server.port();

        let mock_me = server.mock(|when, then| {
            when.method(POST);
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({ "endpoint_id": "whatever" }));
        });
        enroll_client(&mut config).await.unwrap();
        mock_me.assert();
    }

    #[tokio::test]
    #[should_panic = "EnrollBadResponse"]
    async fn test_enroll_client_bad_response() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/client.toml");

        let mut config = read_config(&test_location.display().to_string())
            .await
            .unwrap();

        let server = MockServer::start();
        config.endpoint_server.port = server.port();

        let mock_me = server.mock(|when, then| {
            when.method(POST);
            then.status(500)
                .header("content-type", "application/json")
                .json_body(json!({ "endpoint_id": "whatever" }));
        });
        enroll_client(&mut config).await.unwrap();
        mock_me.assert();
    }

    #[tokio::test]
    #[should_panic = "EnrollDeserialize"]
    async fn test_enroll_client_bad_body() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/client.toml");

        let mut config = read_config(&test_location.display().to_string())
            .await
            .unwrap();

        let server = MockServer::start();
        config.endpoint_server.port = server.port();

        let mock_me = server.mock(|when, then| {
            when.method(POST);
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({ "endpoint_idasdfasdfa": "whatever" }));
        });
        enroll_client(&mut config).await.unwrap();
        mock_me.assert();
    }

    #[tokio::test]
    async fn test_enroll_response() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/client.toml");

        let mut config = read_config(&test_location.display().to_string())
            .await
            .unwrap();

        let server = MockServer::start();
        config.endpoint_server.port = server.port();

        let mock_me = server.mock(|when, then| {
            when.method(POST);
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({ "endpoint_id": "whatever" }));
        });

        let client = ClientBuilder::new()
            .danger_accept_invalid_certs(!config.endpoint_server.verify_ssl)
            .build()
            .unwrap();

        let endpoint_info = gather_info();
        let enroll_info = EnrollSystem {
            enroll_key: config.enroll_key.clone(),
            enrollment_info: endpoint_info,
        };

        let data = serde_json::to_vec(&enroll_info).unwrap();

        let mut builder = client.post(format!(
            "{}:{}/endpoint/v{}/enroll",
            config.endpoint_server.address,
            config.endpoint_server.port,
            config.endpoint_server.version
        ));

        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
                .parse()
                .unwrap(),
        );
        headers.insert(CONTENT_TYPE, "application/json".parse().unwrap());

        // Add User-Agent
        builder = builder.headers(headers);

        builder = builder.body(data);
        let response = builder.send().await.unwrap();

        config.endpoint_id = enroll_response(response).await.unwrap();
        assert_eq!(config.endpoint_id, "whatever");
        mock_me.assert();
    }
}
