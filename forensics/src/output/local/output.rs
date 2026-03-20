use crate::{output::local::error::LocalError, structs::toml::Output};
use csv::WriterBuilder;
use flate2::{Compression, write::GzEncoder};
use log::error;
use serde_json::Value;
use std::{
    fs::{File, create_dir_all},
    io::{BufWriter, Write},
};

/// A write for outputting results locally
/// Supports gzip compression if compression is enabled
pub(crate) enum LocalWrite<W: Write> {
    /// Write data
    Raw(W),
    /// Write data with gzip compression
    Gzip(Box<GzEncoder<W>>),
}

impl<W: Write> Write for LocalWrite<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            LocalWrite::Gzip(w) => w.write(buf),
            LocalWrite::Raw(w) => w.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            LocalWrite::Gzip(w) => w.flush(),
            LocalWrite::Raw(w) => w.flush(),
        }
    }
}

/// Output to local directory provided by TOML input
pub(crate) fn local_output(
    data: &Value,
    output: &mut Output,
    output_name: &str,
) -> Result<(), LocalError> {
    let output_path = format!("{}/{}", output.directory, output.name);

    let result = create_dir_all(&output_path);
    match result {
        Ok(_) => {}
        Err(err) => {
            error!(
                "[forensics] Failed to create output directory for {output_path}. Error: {err:?}"
            );
            return Err(LocalError::CreateDirectory);
        }
    }

    let mut compression_extension = "";
    if output.compress {
        compression_extension = ".gz";
    }
    let extension = &output.format;
    let output_file = format!("{output_path}/{output_name}.{extension}{compression_extension}");

    let file = match File::create(output_file) {
        Ok(results) => results,
        Err(err) => {
            error!(
                "[forensics] Failed to create output file {output_name} at {output_path}. Error: {err:?}"
            );
            return Err(LocalError::CreateFile);
        }
    };

    let file_buf = BufWriter::new(file);

    let mut writer = if output.compress {
        LocalWrite::Gzip(Box::new(GzEncoder::new(file_buf, Compression::default())))
    } else {
        LocalWrite::Raw(file_buf)
    };

    if output.format.to_lowercase() == "csv" {
        // Track output files
        output.output_count += 1;
        return csv_writer(&mut writer, data);
    }

    // Write serde data as newline json
    if data.is_array() && output.format.to_lowercase() == "jsonl" {
        let value = data.as_array().unwrap();
        for entry in value {
            let mut line = serde_json::to_vec(&entry).unwrap_or_default();

            line.push(b'\n');
            if let Err(err) = writer.write_all(&line) {
                error!("[forensics] Could not write all bytes to jsonl: {err:?}");
            }
        }

        // Track output files
        output.output_count += 1;
        return Ok(());
    }

    // Write as normal json object
    let mut line = serde_json::to_vec(&data).unwrap_or_default();

    line.push(b'\n');
    if let Err(err) = writer.write_all(&line) {
        error!("[forensics] Could not write all bytes to json: {err:?}");
    }

    // Track output files
    output.output_count += 1;

    Ok(())
}

/// Write to CSV instead of JSON/JSONL
fn csv_writer<W: Write>(writer: &mut LocalWrite<W>, serde_data: &Value) -> Result<(), LocalError> {
    let mut csv_writer = WriterBuilder::new().from_writer(writer);

    let mut header = Vec::new();
    if serde_data.is_object() {
        for key in serde_data.as_object().unwrap().keys() {
            header.push(key);
        }
    } else if serde_data.is_array() {
        let row = serde_data.as_array().unwrap().first();
        if let Some(value) = row
            && value.is_object()
        {
            for key in value.as_object().unwrap().keys() {
                header.push(key);
            }
        }
    }

    if header.is_empty() {
        return Err(LocalError::WriteCsv);
    }
    if let Err(err) = csv_writer.write_record(&header) {
        error!("[forensics] Could not write csv header: {err:?}");
        return Err(LocalError::WriteCsv);
    }

    if serde_data.is_object() {
        let mut rows = Vec::new();

        for value in serde_data.as_object().unwrap().values() {
            let cell = cell_to_string(value);
            rows.push(cell);
        }
        if let Err(err) = csv_writer.write_record(&rows) {
            error!("[forensics] Could not write csv row: {err:?}");
            return Err(LocalError::WriteCsv);
        }
    } else if serde_data.is_array() {
        for values in serde_data.as_array().unwrap() {
            let mut rows = Vec::new();

            if values.is_object() {
                for value in values.as_object().unwrap().values() {
                    let cell = cell_to_string(value);
                    rows.push(cell);
                }
                if let Err(err) = csv_writer.write_record(&rows) {
                    error!("[forensics] Could not write csv row: {err:?}");
                    return Err(LocalError::WriteCsv);
                }
            }
        }
    }
    Ok(())
}

/// Clean serde values to look nice in csv
fn cell_to_string(v: &Value) -> String {
    match v {
        Value::Null => "".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        _ => serde_json::to_string(v).unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use crate::{
        artifacts::os::systeminfo::info::get_info,
        output::local::output::{cell_to_string, local_output},
        structs::toml::Output,
    };

    #[test]
    fn test_output_json() {
        let mut output = Output {
            name: String::from("test_output"),
            directory: String::from("./tmp"),
            format: String::from("json"),
            compress: false,
            endpoint_id: String::from("abcd"),
            output: String::from("local"),
            ..Default::default()
        };

        let test = "A rust program";
        let name = "output";
        local_output(&serde_json::to_value(test).unwrap(), &mut output, name).unwrap();
    }

    #[test]
    fn test_output_json_compress() {
        let mut output = Output {
            name: String::from("test_output"),
            directory: String::from("./tmp"),
            format: String::from("json"),
            compress: true,
            url: Some(String::new()),
            timeline: false,
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            output: String::from("local"),
            ..Default::default()
        };

        let test = "A rust program";
        let name = "output";
        local_output(&serde_json::to_value(test).unwrap(), &mut output, name).unwrap();
    }

    #[test]
    fn test_output_csv() {
        let mut output = Output {
            name: String::from("test_output"),
            directory: String::from("./tmp"),
            format: String::from("csv"),
            compress: false,
            url: Some(String::new()),
            timeline: false,
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            output: String::from("local"),
            ..Default::default()
        };

        let info = get_info();
        let value = serde_json::to_value(&info).unwrap();
        local_output(&value, &mut output, "csv_info").unwrap();
    }

    #[test]
    fn test_cell_to_string() {
        let test = vec![
            Value::Null,
            Value::Number(serde_json::Number::from(0)),
            Value::String(String::from("test")),
        ];
        for entry in test {
            if entry == Value::Null {
                assert!(cell_to_string(&entry).is_empty());
                continue;
            }
            assert!(!cell_to_string(&entry).is_empty());
        }
    }
}
