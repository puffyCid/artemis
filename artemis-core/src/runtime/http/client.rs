use crate::utils::strings::extract_utf8_string;
use deno_core::{error::AnyError, op2, JsBuffer};
use log::error;
use reqwest::Client;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
pub(crate) struct ClientResponse {
    url: String,
    status: u16,
    headers: HashMap<String, String>,
    content_length: u64,
    body: Vec<u8>,
}

#[op2(async)]
#[string]
/// Make a HTTP request to target URL using specified protocol, headers, and body
pub(crate) async fn js_request(
    #[string] url: String,
    #[string] protocol: String,
    #[serde] headers: HashMap<String, String>,
    #[buffer] body: JsBuffer,
) -> Result<String, AnyError> {
    let client = Client::new();
    let mut builder = match protocol.as_str() {
        "GET" => client.get(url),
        "POST" => client.post(url),
        _ => {
            error!("[runtime] Unsupported protocol selected: {protocol}");
            return Err(AnyError::msg("unsupported protocol"));
        }
    };

    // Add the headers
    for (key, value) in headers {
        builder = builder.header(key, value);
    }

    // Add User-Agent
    builder = builder.header(
        "User-Agent",
        format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
    );

    builder = builder.body(body.to_vec());

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

        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL2h0dHAvY2xpZW50LnRzCmFzeW5jIGZ1bmN0aW9uIHJlcXVlc3QoCiAgdXJsLAogIHByb3RvY29sLAogIGJvZHkgPSBuZXcgQXJyYXlCdWZmZXIoMCksCiAgaGVhZGVycyA9IHsgIkNvbnRlbnQtVHlwZSI6ICJhcHBsaWNhdGlvbi9qc29uIiB9LAopIHsKICBjb25zdCByZXN1bHQgPSBhd2FpdCBodHRwLnNlbmQodXJsLCBwcm90b2NvbCwgaGVhZGVycywgYm9keSk7CiAgaWYgKHJlc3VsdCBpbnN0YW5jZW9mIEVycm9yKSB7CiAgICByZXR1cm4gcmVzdWx0OwogIH0KICBjb25zdCByZXMgPSBKU09OLnBhcnNlKHJlc3VsdCk7CiAgcmV0dXJuIHJlczsKfQoKLy8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL2VuY29kaW5nL2J5dGVzLnRzCmZ1bmN0aW9uIGVuY29kZUJ5dGVzKGRhdGEpIHsKICBjb25zdCByZXN1bHQgPSBlbmNvZGluZy5ieXRlc19lbmNvZGUoZGF0YSk7CiAgcmV0dXJuIHJlc3VsdDsKfQoKLy8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL2VuY29kaW5nL3N0cmluZ3MudHMKZnVuY3Rpb24gZXh0cmFjdFV0ZjhTdHJpbmcoZGF0YSkgewogIGNvbnN0IHJlc3VsdCA9IGVuY29kaW5nLmV4dHJhY3RfdXRmOF9zdHJpbmcoZGF0YSk7CiAgcmV0dXJuIHJlc3VsdDsKfQoKLy8gbWFpbi50cwphc3luYyBmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHVybCA9ICJodHRwOi8vMTI3LjAuMC4xOlJFUExBQ0VQT1JUL3VzZXItYWdlbnQiOwogIGNvbnN0IGJvZHkgPSAiIjsKICBjb25zdCByZXMgPSBhd2FpdCByZXF1ZXN0KHVybCwgIkdFVCIsIC8qIEdFVCAqLyBlbmNvZGVCeXRlcyhib2R5KSk7CiAgY29uc29sZS5sb2coSlNPTi5wYXJzZShleHRyYWN0VXRmOFN0cmluZyhuZXcgVWludDhBcnJheShyZXMuYm9keSkpKSk7CiAgcmV0dXJuIHJlczsKfQptYWluKCk7Cg==";
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
