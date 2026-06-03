use crate::output2::{
    config::{OutputConfig, OutputFormat},
    encoder::{
        artifact_encoder::Encoder, csv::CsvEncoder, json::JsonEncoder, jsonl::JsonlEncoder,
        text::TextEncoder, timeline::TimelineEncoder,
    },
};

/// Create an `Encoder` to output forensic data
pub(crate) fn build_encoder(config: &OutputConfig) -> Encoder {
    match config.format {
        OutputFormat::Json => Encoder::Json(JsonEncoder),
        OutputFormat::Jsonl => Encoder::Jsonl(JsonlEncoder),
        OutputFormat::Csv => Encoder::Csv(CsvEncoder),
        OutputFormat::Timeline => Encoder::Timeline(TimelineEncoder),
        OutputFormat::Text => Encoder::Text(TextEncoder),
    }
}

#[cfg(test)]
mod tests {
    use crate::output2::{
        config::{OutputConfig, OutputFormat},
        encoder::{
            artifact_encoder::Encoder, csv::CsvEncoder, factory::build_encoder, json::JsonEncoder,
            jsonl::JsonlEncoder, text::TextEncoder,
        },
    };

    #[test]
    fn test_build_encoder() {
        let mut output = OutputConfig::default();
        assert_eq!(build_encoder(&output), Encoder::Jsonl(JsonlEncoder));
        output.format = OutputFormat::Csv;
        assert_eq!(build_encoder(&output), Encoder::Csv(CsvEncoder));
        output.format = OutputFormat::Json;
        assert_eq!(build_encoder(&output), Encoder::Json(JsonEncoder));
    }

    #[test]
    fn test_text_encoder() {
        let mut output = OutputConfig::default();
        output.format = OutputFormat::Text;
        assert_eq!(build_encoder(&output), Encoder::Text(TextEncoder));
    }
}
