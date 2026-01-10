use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct JSScript {
    pub name: String,
    pub script: String, // Base64 encoded js script
}
