use deno_core::{error::AnyError, op2};
use serde::Serialize;
use url::Url;

#[derive(Serialize)]
pub(crate) struct UrlInfo {
    authority: String,
    username: String,
    password: String,
    host: String,
    domain: String,
    port: Option<u16>,
    path: String,
    segments: Vec<String>,
    query: String,
    query_pairs: Vec<String>,
    fragment: String,
    scheme: String,
}

#[op2]
#[string]
pub(crate) fn url_parse(#[string] url_string: String) -> Result<String, AnyError> {
    let res = Url::parse(&url_string)?;

    let mut info = UrlInfo {
        authority: res.authority().to_string(),
        username: res.username().to_string(),
        password: res.password().unwrap_or_default().to_string(),
        host: res.host_str().unwrap_or_default().to_string(),
        domain: res.domain().unwrap_or_default().to_string(),
        port: res.port(),
        path: res.path().to_string(),
        segments: Vec::new(),
        query: res.query().unwrap_or_default().to_string(),
        fragment: res.fragment().unwrap_or_default().to_string(),
        scheme: res.scheme().to_string(),
        query_pairs: Vec::new(),
    };

    if let Some(segs) = res.path_segments() {
        for seg in segs {
            if seg.is_empty() {
                continue;
            }
            info.segments.push(seg.to_string());
        }
    }

    for (key, value) in res.query_pairs() {
        info.query_pairs.push(format!("{key}={value}"))
    }

    let results = serde_json::to_string(&info)?;
    Ok(results)
}
