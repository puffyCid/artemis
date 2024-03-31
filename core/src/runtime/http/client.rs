use crate::utils::strings::extract_utf8_string;
use deno_core::{error::AnyError, op2, JsBuffer};
use log::error;
use nom::AsBytes;
use reqwest::{redirect::Policy, ClientBuilder};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Serialize)]
pub(crate) struct ClientResponse {
    url: String,
    status: u16,
    headers: HashMap<String, String>,
    content_length: u64,
    body: Vec<u8>,
}

#[derive(Deserialize)]
struct ClientRequest {
    url: String,
    protocol: String,
    headers: HashMap<String, String>,
    body_type: String,
    follow_redirects: bool,
    verify_ssl: bool,
}

#[op2(async)]
#[string]
/// Make a HTTP request to target URL using specified protocol, headers, and body
pub(crate) async fn js_request(
    #[string] js_request: String,
    #[buffer] body: JsBuffer,
) -> Result<String, AnyError> {
    let request_result = serde_json::from_str(&js_request);
    let request: ClientRequest = match request_result {
        Ok(result) => result,
        Err(err) => {
            error!("[runtime] Could not parse request: {err:?}");
            return Err(AnyError::msg(format!("Failed to parse request {err:?}")));
        }
    };

    let client_res = if !request.follow_redirects {
        ClientBuilder::new()
            .redirect(Policy::none())
            .danger_accept_invalid_certs(!request.verify_ssl)
            .build()
    } else {
        ClientBuilder::new()
            .danger_accept_invalid_certs(!request.verify_ssl)
            .build()
    };

    let client = match client_res {
        Ok(result) => result,
        Err(err) => {
            error!("[runtime] Could not create client: {err:?}");
            return Err(AnyError::msg("Failed to create HTTP client"));
        }
    };

    let mut builder = match request.protocol.as_str() {
        "GET" => client.get(request.url),
        "POST" => client.post(request.url),
        _ => {
            error!(
                "[runtime] Unsupported protocol selected: {}",
                request.protocol
            );
            return Err(AnyError::msg("unsupported protocol"));
        }
    };

    // Add the headers
    for (key, value) in request.headers {
        builder = builder.header(key, value);
    }

    // Add User-Agent
    builder = builder.header(
        "User-Agent",
        format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
    );

    if request.body_type == "form" {
        let form_data_result = serde_json::from_slice(body.as_bytes());
        let form_data: HashMap<String, Value> = match form_data_result {
            Ok(result) => result,
            Err(err) => {
                error!("[runtime] Could not deserialize form data: {err}");
                return Err(AnyError::msg("failed to extract form data"));
            }
        };
        builder = builder.form(&form_data);
    } else {
        builder = builder.body(body.to_vec());
    }

    let res_result = builder.send().await?;

    let mut res_headers = HashMap::new();
    for (key, value) in res_result.headers() {
        // Header values can technically be bytes. Try to extract the string if any
        res_headers.insert(key.to_string(), extract_utf8_string(value.as_bytes()));
    }

    let res = ClientResponse {
        url: res_result.url().to_string(),
        status: res_result.status().as_u16(),
        headers: res_headers,
        content_length: res_result.content_length().unwrap_or(0),
        body: res_result.bytes().await?.to_vec(),
    };

    let results = serde_json::to_string(&res)?;
    Ok(results)
}

#[cfg(test)]
mod tests {
    use crate::{
        runtime::deno::execute_script,
        structs::artifacts::runtime::script::JSScript,
        structs::toml::Output,
        utils::{
            encoding::{base64_decode_standard, base64_encode_standard},
            strings::extract_utf8_string,
        },
    };
    use httpmock::{Method::GET, MockServer};

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("json"),
            compress,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: None,
            filter_script: None,
            logging: None,
        }
    }

    #[test]
    fn test_js_request() {
        let server = MockServer::start();
        let port = server.port();

        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL2h0dHAvY2xpZW50LnRzCmFzeW5jIGZ1bmN0aW9uIHJlcXVlc3QoCiAgcmVxdWVzdCwKICBib2R5ID0gbmV3IEFycmF5QnVmZmVyKDApLAopIHsKICBjb25zdCByZXN1bHQgPSBhd2FpdCBodHRwLnNlbmQoSlNPTi5zdHJpbmdpZnkocmVxdWVzdCksIGJvZHkpOwogIGlmIChyZXN1bHQgaW5zdGFuY2VvZiBFcnJvcikgewogICAgcmV0dXJuIHJlc3VsdDsKICB9CiAgY29uc3QgcmVzID0gSlNPTi5wYXJzZShyZXN1bHQpOwogIHJldHVybiByZXM7Cn0KCi8vIC4uLy4uL2FydGVtaXMtYXBpL3NyYy9lbmNvZGluZy9ieXRlcy50cwpmdW5jdGlvbiBlbmNvZGVCeXRlcyhkYXRhKSB7CiAgY29uc3QgcmVzdWx0ID0gZW5jb2RpbmcuYnl0ZXNfZW5jb2RlKGRhdGEpOwogIHJldHVybiByZXN1bHQ7Cn0KCi8vIC4uLy4uL2FydGVtaXMtYXBpL3NyYy9lbmNvZGluZy9zdHJpbmdzLnRzCmZ1bmN0aW9uIGV4dHJhY3RVdGY4U3RyaW5nKGRhdGEpIHsKICBjb25zdCByZXN1bHQgPSBlbmNvZGluZy5leHRyYWN0X3V0Zjhfc3RyaW5nKGRhdGEpOwogIHJldHVybiByZXN1bHQ7Cn0KCi8vIG1haW4udHMKYXN5bmMgZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCB1cmwgPSAiaHR0cDovLzEyNy4wLjAuMTpSRVBMQUNFUE9SVC91c2VyLWFnZW50IjsKICBjb25zdCBib2R5ID0gIiI7CiAgY29uc3QganNfcmVxdWVzdCA9IHsKICAgIHVybDogdXJsLAogICAgcHJvdG9jb2w6ICJHRVQiLAogICAgaGVhZGVyczogeyAiQ29udGVudC1UeXBlIjogImFwcGxpY2F0aW9uL2pzb24iIH0sCiAgICBib2R5X3R5cGU6ICIiLAogICAgZm9sbG93X3JlZGlyZWN0czogdHJ1ZSwKICAgIHZlcmlmeV9zc2w6IHRydWUsCiAgfQogIGNvbnN0IHJlcyA9IGF3YWl0IHJlcXVlc3QoanNfcmVxdWVzdCwgZW5jb2RlQnl0ZXMoYm9keSkpOwogIGNvbnNvbGUubG9nKEpTT04ucGFyc2UoZXh0cmFjdFV0ZjhTdHJpbmcobmV3IFVpbnQ4QXJyYXkocmVzLmJvZHkpKSkpOwogIHJldHVybiByZXM7Cn0KbWFpbigpOwo=";
        let data = base64_decode_standard(&test).unwrap();
        let temp_script = extract_utf8_string(&data).replace("REPLACEPORT", &format!("{port}"));
        let update_script = base64_encode_standard(temp_script.as_bytes());

        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("network_request"),
            script: update_script,
        };
        let mock_me = server.mock(|when, then| {
            when.method(GET);
            then.status(200)
                .body("{\"data\": \"A mock response\"}")
                .header("Last-Modified", "2023-06-14 12:00:00")
                .header("Content-MD5", "sQqNsWTgdUEFt6mb5y4/5Q==");
        });
        execute_script(&mut output, &script).unwrap();
        mock_me.assert();
    }
}
