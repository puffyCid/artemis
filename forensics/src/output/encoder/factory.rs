use crate::{
    output::encoder::{
        artifact_encoder::Encoder, csv::CsvEncoder, json::JsonEncoder, jsonl::JsonlEncoder,
        text::TextEncoder, timeline::TimelineEncoder, xml::XmlEncoder,
    },
    structs::toml::{OutputConfig, OutputFormat},
};

/// Create an `Encoder` to output forensic data
pub(crate) fn build_encoder(config: &OutputConfig) -> Encoder {
    match config.format {
        OutputFormat::Json => Encoder::Json(JsonEncoder),
        OutputFormat::Jsonl => Encoder::Jsonl(JsonlEncoder),
        OutputFormat::Csv => Encoder::Csv(CsvEncoder),
        OutputFormat::Timeline => Encoder::Timeline(TimelineEncoder),
        OutputFormat::Text => Encoder::Text(TextEncoder),
        OutputFormat::Xml => Encoder::Xml(XmlEncoder),
    }
}

#[cfg(test)]
mod tests {
    use crate::output::encoder::{
        artifact_encoder::Encoder, csv::CsvEncoder, factory::build_encoder, json::JsonEncoder,
        jsonl::JsonlEncoder, text::TextEncoder,
    };
    use crate::structs::toml::{OutputConfig, OutputFormat};

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
