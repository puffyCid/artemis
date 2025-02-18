use crate::{
    runtimev2::helper::{bytes_arg, string_arg, value_arg},
    utils::strings::extract_utf8_string,
};
use boa_engine::{
    js_string, object::builtins::JsPromise, Context, JsError, JsResult, JsValue, NativeFunction,
};

use reqwest::{redirect::Policy, ClientBuilder, RequestBuilder};
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

/// Make a HTTP request to target URL using specified protocol, headers, and body
pub(crate) fn js_request(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let js_request = value_arg(args, &0, context)?;
    let request_result = serde_json::from_value(js_request);
    let request: ClientRequest = match request_result {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Could not parse request: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
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
            let issue = format!("Could not create client: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let mut builder = match request.protocol.as_str() {
        "GET" => client.get(request.url),
        "POST" => client.post(request.url),
        _ => {
            let issue = String::from("Unsupported protocol selected");
            return Err(JsError::from_opaque(js_string!(issue).into()));
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

    let body = bytes_arg(args, &1, context)?;

    if request.body_type == "form" {
        let form_data_result = serde_json::from_slice(&body);
        let form_data: HashMap<String, Value> = match form_data_result {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Could not deserialize form data: {err}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };
        builder = builder.form(&form_data);
    } else {
        builder = builder.body(body.clone());
    }

    // Create a promise to execute our async script
    let promise = JsPromise::from_future(send(builder), context).then(
        Some(
            NativeFunction::from_fn_ptr(|_, args, ctx| {
                // Get the value from the script
                let script_value = string_arg(args, &0)?;
                let serde_value = serde_json::from_str(&script_value).unwrap_or_default();
                // Return the JavaScript object
                let value = JsValue::from_json(&serde_value, ctx)?;
                Ok(value)
            })
            .to_js_function(context.realm()),
        ),
        None,
        context,
    );

    Ok(promise.into())
}

async fn send(builder: RequestBuilder) -> JsResult<JsValue> {
    let res_result = match builder.send().await {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to send request: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let mut res_headers = HashMap::new();
    for (key, value) in res_result.headers() {
        // Header values can technically be bytes. Try to extract the string if any
        res_headers.insert(key.to_string(), extract_utf8_string(value.as_bytes()));
    }
    let url = res_result.url().to_string();
    let status = res_result.status().as_u16();
    let content_length = res_result.content_length().unwrap_or_default();
    let res_body = match res_result.bytes().await {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get response bytes: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let res = ClientResponse {
        url,
        status,
        headers: res_headers,
        content_length,
        body: res_body.to_vec(),
    };
    // We have to serialize to string for now
    let data = serde_json::to_string(&res).unwrap_or_default();
    Ok(js_string!(data).into())
}

#[cfg(test)]
mod tests {
    use crate::{
        runtimev2::run::execute_script,
        structs::{artifacts::runtime::script::JSScript, toml::Output},
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

        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL2h0dHAvY2xpZW50LnRzCmFzeW5jIGZ1bmN0aW9uIHJlcXVlc3QoCiAgcmVxdWVzdCwKICBib2R5ID0gbmV3IFVpbnQ4QXJyYXkoWzBdKSwKKSB7CiAgY29uc3QgcmVzdWx0ID0gYXdhaXQganNfcmVxdWVzdChyZXF1ZXN0LCBib2R5KTsKICBpZiAocmVzdWx0IGluc3RhbmNlb2YgRXJyb3IpIHsKICAgIHJldHVybiByZXN1bHQ7CiAgfQogIHJldHVybiByZXN1bHQ7Cn0KCi8vIC4uLy4uL2FydGVtaXMtYXBpL3NyYy9lbmNvZGluZy9ieXRlcy50cwpmdW5jdGlvbiBlbmNvZGVCeXRlcyhkYXRhKSB7CiAgY29uc3QgcmVzdWx0ID0ganNfZW5jb2RlX2J5dGVzKGRhdGEpOwogIHJldHVybiByZXN1bHQ7Cn0KCi8vIC4uLy4uL2FydGVtaXMtYXBpL3NyYy9lbmNvZGluZy9zdHJpbmdzLnRzCmZ1bmN0aW9uIGV4dHJhY3RVdGY4U3RyaW5nKGRhdGEpIHsKICBjb25zdCByZXN1bHQgPSBqc19leHRyYWN0X3V0Zjhfc3RyaW5nKGRhdGEpOwogIHJldHVybiByZXN1bHQ7Cn0KCi8vIG1haW4udHMKYXN5bmMgZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCB1cmwgPSAiaHR0cDovLzEyNy4wLjAuMTpSRVBMQUNFUE9SVC91c2VyLWFnZW50IjsKICBjb25zdCBib2R5ID0gIiI7CiAgY29uc3QganNfcmVxdWVzdCA9IHsKICAgIHVybDogdXJsLAogICAgcHJvdG9jb2w6ICJHRVQiLAogICAgaGVhZGVyczogeyAiQ29udGVudC1UeXBlIjogImFwcGxpY2F0aW9uL2pzb24iIH0sCiAgICBib2R5X3R5cGU6ICIiLAogICAgZm9sbG93X3JlZGlyZWN0czogdHJ1ZSwKICAgIHZlcmlmeV9zc2w6IHRydWUsCiAgfQogIGNvbnN0IHJlcyA9IGF3YWl0IHJlcXVlc3QoanNfcmVxdWVzdCwgZW5jb2RlQnl0ZXMoYm9keSkpOwogIGNvbnNvbGUubG9nKGV4dHJhY3RVdGY4U3RyaW5nKG5ldyBVaW50OEFycmF5KHJlcy5ib2R5KSkpOwogIHJldHVybiByZXM7Cn0KbWFpbigpOwo=";
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
