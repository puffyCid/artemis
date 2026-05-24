use crate::output2::{
    context::ArtifactContext,
    encoder::artifact_encoder::ArtifactEncoder,
    error::OutputResult,
    record::{Record, RecordStream},
};
use csv::{Writer, WriterBuilder};
use serde_json::{Map, Value};
use std::io::Write;

/// Encoder for CSV files
#[derive(Debug, PartialEq)]
pub(crate) struct CsvEncoder;

impl ArtifactEncoder for CsvEncoder {
    fn mime_type(&self) -> &str {
        "text/csv"
    }
    fn extension(&self) -> &str {
        "csv"
    }
    fn encode(
        &self,
        records: &mut dyn RecordStream,
        writer: &mut dyn Write,
        _context: &ArtifactContext,
    ) -> OutputResult<usize> {
        let mut csv_writer = WriterBuilder::new().from_writer(writer);
        let Some(record) = records.next_record()? else {
            return Ok(0);
        };

        // Need headers first
        let Record::Json(record) = record;
        let fields = record.fields;
        let headers: Vec<String> = fields.keys().cloned().collect();
        csv_writer.write_record(&headers)?;
        write_row(&mut csv_writer, &headers, &fields)?;

        let mut count = 1;

        // Now write each row
        while let Some(record) = records.next_record()? {
            let Record::Json(record) = record;
            write_row(&mut csv_writer, &headers, &record.fields)?;

            count += 1;
        }

        csv_writer.flush()?;
        Ok(count)
    }
}

/// Writes one artifact record as a CSV row.
fn write_row<W: Write>(
    writer: &mut Writer<W>,
    headers: &[String],
    fields: &Map<String, Value>,
) -> OutputResult<()> {
    let row = headers
        .iter()
        .map(|header| fields.get(header).map(value_to_cell).unwrap_or_default())
        .collect::<Vec<_>>();

    writer.write_record(row)?;

    Ok(())
}

/// Converts a JSON value into a CSV cell string.
fn value_to_cell(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}
