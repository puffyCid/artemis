use crate::output2::{
    config::{OutputConfig, OutputFormat},
    encoder::{artifact_encoder::Encoder, csv::CsvEncoder, json::JsonEncoder, jsonl::JsonlEncoder},
};

pub(crate) fn build_encoder(config: &OutputConfig) -> Encoder {
    match config.format {
        OutputFormat::Json => Encoder::Json(JsonEncoder),
        OutputFormat::Jsonl => Encoder::Jsonl(JsonlEncoder),
        OutputFormat::Csv => Encoder::Csv(CsvEncoder),
    }
}
