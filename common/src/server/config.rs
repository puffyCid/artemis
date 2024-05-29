use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ArtemisConfig {
    pub metadata: ArtemisInfo,
    pub enroll_key: String,
    pub endpoint_id: String,
    pub endpoint_server: EndpointServer,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ArtemisInfo {
    pub version: String,
    pub name: String,
    pub target: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EndpointServer {
    pub address: String,
    pub port: u16,
    pub cert: String,
    pub storage: String,
    pub verify_ssl: bool,
    pub version: u8,
}
