use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum EndpointOS {
    Windows,
    Darwin,
    Linux,
    All,
}
