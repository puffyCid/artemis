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
