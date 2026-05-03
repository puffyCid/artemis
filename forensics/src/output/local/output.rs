use crate::{
    artifacts::os::systeminfo::info::get_info_metadata,
    output::local::error::LocalError,
    structs::toml::Output,
    utils::{
        time::{time_now, unixepoch_to_iso},
        uuid::generate_uuid,
    },
};
use csv::WriterBuilder;
use flate2::{Compression, write::GzEncoder};
use log::error;
use serde_json::{Value, json};
use std::{
    fs::{File, create_dir_all},
    io::{BufWriter, Write},
};

/// A writer for outputting results locally
/// Supports gzip if compression is enabled
enum LocalWrite<W: Write> {
    /// Write data
    Raw(W),
    /// Write data with gzip compression. Wrap in box due "larger" `GzEncoder` state.
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
    data: &mut Value,
    output: &mut Output,
    filename: &str,
    start_time: u64,
    artifact_name: &str,
) -> Result<(), LocalError> {
    // If we have empty array return now
    if data.as_array().is_some_and(|v| v.is_empty()) {
        return Ok(());
    }

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
    let uuid = generate_uuid();

    let output_file = format!("{output_path}/{filename}.{extension}{compression_extension}");

    let file = match File::create(&output_file) {
        Ok(results) => results,
        Err(err) => {
            error!(
                "[forensics] Failed to create output file {filename} at {output_path}. Error: {err:?}"
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
        output.output_files.push(output_file);
        csv_writer(&mut writer, data)?;
        return finish_writer(writer);
    }

    // Get small amount of system metadata
    let info = get_info_metadata();
    let complete = unixepoch_to_iso(time_now() as i64);
    let disable_meta = 0;

    // Write serde data as newline json
    if data.is_array() {
        let value = data.as_array_mut().unwrap();

        for entry in value {
            // Append metadata row
            if entry.is_object() && start_time != disable_meta {
                entry["collection_metadata"] = json![{
                        "endpoint_id": output.endpoint_id,
                        "uuid": uuid,
                        "id": output.collection_id,
                        "artifact_name": artifact_name,
                        "complete_time": complete,
                        "start_time": unixepoch_to_iso(start_time as i64),
                        "hostname": info.hostname,
                        "os_version": info.os_version,
                        "platform": info.platform,
                        "kernel_version": info.kernel_version,
                        "load_performance": info.performance,
                        "artemis_version": info.artemis_version,
                        "rust_version": info.rust_version,
                        "build_date": info.build_date,
                        "interfaces": info.interfaces,
                }];
            }
            // If we are outputting to jsonl write to newline
            if output.format.to_lowercase() == "jsonl" {
                let mut line = serde_json::to_vec(&entry).unwrap_or_default();

                line.push(b'\n');
                if let Err(err) = writer.write_all(&line) {
                    error!("[forensics] Could not write all bytes to jsonl: {err:?}");
                }
            }
        }

        // If we are writing jsonl output. We are done now
        if output.format.to_lowercase() == "jsonl" {
            // Track output files
            output.output_files.push(output_file);
            return finish_writer(writer);
        }
    } else if data.is_object() && start_time != disable_meta {
        data["collection_metadata"] = json![{
                "endpoint_id": output.endpoint_id,
                "uuid": uuid,
                "id": output.collection_id,
                "artifact_name": artifact_name,
                "complete_time": complete,
                "start_time": unixepoch_to_iso(start_time as i64),
                "hostname": info.hostname,
                "os_version": info.os_version,
                "platform": info.platform,
                "kernel_version": info.kernel_version,
                "load_performance": info.performance,
                "artemis_version": info.artemis_version,
                "rust_version": info.rust_version,
                "build_date": info.build_date,
                "interfaces": info.interfaces,
        }];
    }
    // Write as normal json object
    let mut line = serde_json::to_vec(&data).unwrap_or_default();

    line.push(b'\n');
    if let Err(err) = writer.write_all(&line) {
        error!("[forensics] Could not write all bytes to json: {err:?}");
        return Err(LocalError::CreateFile);
    }

    // Track output files
    output.output_files.push(output_file);

    finish_writer(writer)
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
fn cell_to_string(data: &Value) -> String {
    match data {
        Value::Null => "".into(),
        Value::Bool(bool) => bool.to_string(),
        Value::Number(int) => int.to_string(),
        Value::String(str) => str.clone(),
        _ => serde_json::to_string(data).unwrap_or_default(),
    }
}

/// Complete the output process if compression with gzip
fn finish_writer<W: Write>(writer: LocalWrite<W>) -> Result<(), LocalError> {
    if let LocalWrite::Gzip(gz) = writer
        && let Err(err) = gz.finish()
    {
        error!("[forensics] Could not finish writing compressed bytes: {err:?}");
        return Err(LocalError::CreateFile);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::systeminfo::info::get_info,
        output::local::output::{cell_to_string, local_output},
        structs::toml::Output,
    };
    use serde_json::Value;

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
        local_output(
            &mut serde_json::to_value(test).unwrap(),
            &mut output,
            name,
            0,
            test,
        )
        .unwrap();
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
        local_output(
            &mut serde_json::to_value(test).unwrap(),
            &mut output,
            name,
            0,
            test,
        )
        .unwrap();
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
        let mut value = serde_json::to_value(&info).unwrap();
        local_output(&mut value, &mut output, "csv_info", 0, "test").unwrap();
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
