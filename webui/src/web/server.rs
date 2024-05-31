use leptos::logging::error;
use reqwest::{Client, Error, Method, Response};
use web_sys::{wasm_bindgen::JsValue, Window};

/// Get server IP and Port
pub async fn server_info() -> Result<(String, String), JsValue> {
    let win_option = web_sys::window();
    let win = match win_option {
        Some(result) => result,
        None => return Err(JsValue::UNDEFINED),
    };

    let ip = get_ip(&win).await?;
    let port = get_port(&win).await?;

    Ok((ip, port))
}

/// Get server IP
pub async fn get_ip(win: &Window) -> Result<String, JsValue> {
    let server_ip = win.location().hostname()?;
    Ok(server_ip)
}

/// Get server port
pub async fn get_port(win: &Window) -> Result<String, JsValue> {
    let server_port = win.location().port()?;
    Ok(server_port)
}

/// Compose and send a request to the server
pub async fn request_server(uri: &str, body: String, method: Method) -> Result<Response, Error> {
    let server_result = server_info().await;
    let (server, port) = match server_result {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to get server IP: {err:?}");
            (String::new(), String::new())
        }
    };
    let uri = format!("http://{server}:{port}/ui/v1/{uri}");
    if method == Method::POST {
        Client::new()
            .post(uri)
            .body(body)
            .header("Content-Type", "application/json")
            .send()
            .await
    } else {
        Client::new().get(uri).send().await
    }
}
