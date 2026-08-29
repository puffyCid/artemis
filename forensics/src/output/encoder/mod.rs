pub(crate) mod artifact_encoder;
mod csv;
#[cfg(feature = "duck")]
mod duckdb;
pub(crate) mod factory;
mod helper;
mod json;
mod jsonl;
mod metadata;
mod parquet;
mod sqlite;
mod text;
mod timeline;
mod xml;
