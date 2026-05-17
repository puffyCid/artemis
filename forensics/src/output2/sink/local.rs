use std::{
    fs::{File, create_dir_all},
    io::{BufWriter, Write},
    path::PathBuf,
};

use flate2::{Compression, write::GzEncoder};

use crate::{
    output2::{
        config::OutputConfig,
        error::{OutputError, OutputResult},
        sink::{
            output_handle::{OutputHandle, OutputLocation, OutputType},
            output_sink::{LogOutput, OutputSink},
        },
    },
    utils::uuid::generate_uuid,
};

pub(crate) struct LocalSink {
    output_directory: PathBuf,
    collection_id: u64,
    compress: bool,
}

impl LocalSink {
    pub(crate) fn new(config: &OutputConfig) -> OutputResult<Self> {
        let output_dir = config.directory.join(&config.name);
        create_dir_all(&output_dir).map_err(|err| OutputError::io_path(&output_dir, err))?;

        Ok(Self {
            output_directory: output_dir,
            collection_id: config.collection_id,
            compress: config.compress,
        })
    }

    fn output_path(&self, artifact_name: &str, extension: &str) -> PathBuf {
        let uuid = generate_uuid();
        let filename = if self.compress {
            format!("{artifact_name}_{uuid}.{extension}.gz")
        } else {
            format!("{artifact_name}_{uuid}.{extension}")
        };

        self.output_directory.join(filename)
    }

    fn log_path(&self) -> PathBuf {
        let log = format!("artemis_{}_{}.log", self.collection_id, generate_uuid());
        self.output_directory.join(log)
    }
}

impl OutputSink for LocalSink {
    fn write_artifact(
        &mut self,
        artifact_name: &str,
        extension: &str,
        _mime_type: &str,
        encode: &mut dyn FnMut(&mut dyn std::io::Write) -> OutputResult<usize>,
    ) -> OutputResult<OutputHandle> {
        let output_path = self.output_path(artifact_name, extension);
        let file =
            File::create(&output_path).map_err(|err| OutputError::io_path(&output_path, err))?;
        let writer = BufWriter::new(file);

        let record_count = if self.compress {
            let mut gzip = GzEncoder::new(writer, Compression::default());
            let count = encode(&mut gzip)?;
            gzip.finish()?;
            count
        } else {
            let mut writer = writer;
            let count = encode(&mut writer)?;
            writer.flush()?;
            count
        };

        Ok(OutputHandle {
            artifact_name: artifact_name.to_string(),
            location: OutputLocation::Local(output_path),
            record_count,
            extension: extension.to_string(),
            compressed: self.compress,
            output_type: OutputType::Artifact,
        })
    }

    fn write_report(
        &mut self,
        report: &crate::output2::report::CollectionReport,
    ) -> OutputResult<OutputHandle> {
        let uuid = generate_uuid();
        let output_path = self.output_directory.join(format!("report_{uuid}.json"));

        let file =
            File::create(&output_path).map_err(|err| OutputError::io_path(&output_path, err))?;
        serde_json::to_writer(file, report)?;

        Ok(OutputHandle {
            artifact_name: String::from("report"),
            location: OutputLocation::Local(output_path),
            record_count: 1,
            extension: String::from("json"),
            compressed: false,
            output_type: OutputType::Report,
        })
    }

    fn create_log_file(&mut self) -> OutputResult<LogOutput> {
        let path = self.log_path();
        let file = File::create(&path).map_err(|err| OutputError::io_path(&path, err))?;

        Ok(LogOutput { path, file })
    }
}
