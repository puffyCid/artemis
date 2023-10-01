use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct JSScript {
    pub name: String,
    pub script: String, // Base64 encoded js script
}
