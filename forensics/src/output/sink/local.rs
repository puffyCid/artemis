use crate::{
    filesystem::files::list_files,
    output::{
        encoder::artifact_encoder::StreamTarget,
        error::{OutputError, OutputResult},
        report::CollectionReport,
        sink::{
            output_handle::{OutputHandle, OutputLocation},
            output_sink::{LogOutput, OutputSink},
        },
    },
    structs::toml::OutputConfig,
    utils::{compression::compress::compress_output_zip, uuid::generate_uuid},
};
use flate2::{Compression, write::GzEncoder};
use std::{
    fs::{File, create_dir_all, remove_dir, remove_file},
    io::{BufWriter, Write},
    path::PathBuf,
};

/// A data Sink representing the local system pipeline flow
pub(crate) struct LocalSink {
    /// Target output directory
    output_directory: PathBuf,
    /// Collection ID for the Artemis execution
    collection_id: u64,
    /// Whether to compress the results with gzip. Then all files are compressed with zip
    compress: bool,
}

impl LocalSink {
    /// Creates a local sink and ensures the output directory exists
    pub(crate) fn new(config: &OutputConfig) -> OutputResult<Self> {
        let output_dir = config.directory.join(&config.name);
        create_dir_all(&output_dir).map_err(|err| OutputError::io_path(&output_dir, err))?;

        Ok(Self {
            output_directory: output_dir,
            collection_id: config.collection_id,
            compress: config.compress,
        })
    }

    pub(crate) fn stream_artifact(&self, artifact_name: &str, extension: &str) -> StreamTarget {
        let uuid = generate_uuid();
        let filename = format!("{artifact_name}_{uuid}.{extension}");
        StreamTarget::new(self.output_directory.join(filename))
    }

    /// Builds a unique output path for an artifact file
    fn output_path(&self, artifact_name: &str, extension: &str) -> PathBuf {
        let uuid = generate_uuid();
        let filename = if self.compress {
            format!("{artifact_name}_{uuid}.{extension}.gz")
        } else {
            format!("{artifact_name}_{uuid}.{extension}")
        };

        self.output_directory.join(filename)
    }

    /// Builds a unique log file path for this Artemis run
    fn log_path(&self) -> PathBuf {
        let log = format!("artemis_{}_{}.log", self.collection_id, generate_uuid());
        self.output_directory.join(log)
    }

    /// Zips the completed local output directory and removes loose output files
    fn compress_final_output(&self) -> OutputResult<()> {
        let output_dir = self.output_directory.display().to_string();
        let zip_name = self.output_directory.display().to_string();
        compress_output_zip(&output_dir, &zip_name).map_err(|err| {
            OutputError::Finalize(format!(
                "failed to zip output directory {}: {err:?}",
                self.output_directory.display()
            ))
        })?;
        let entries = list_files(&output_dir).map_err(|err| {
            OutputError::Finalize(format!(
                "failed to list output directory {}: {err:?}",
                self.output_directory.display()
            ))
        })?;
        // Only delete files associated with Artemis output
        for entry in entries {
            if !entry.ends_with(".json")
                && !entry.ends_with(".log")
                && !entry.ends_with(".gz")
                && !entry.ends_with(".csv")
                && !entry.ends_with(".jsonl")
                && !entry.ends_with(".zip")
                && !entry.ends_with(".xml")
            {
                continue;
            }
            remove_file(&entry).map_err(|err| OutputError::io_path(&entry, err))?;
        }
        // We only remove empty directories
        remove_dir(&self.output_directory)
            .map_err(|err| OutputError::io_path(&self.output_directory, err))?;
        Ok(())
    }
}

impl OutputSink for LocalSink {
    fn write_artifact(
        &mut self,
        artifact_name: &str,
        extension: &str,
        _mime_type: &str,
        encode: &mut dyn FnMut(&mut dyn Write) -> OutputResult<usize>,
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

        Ok(OutputHandle::artifact(
            artifact_name,
            OutputLocation::Local(output_path),
            record_count,
            extension,
            self.compress,
        ))
    }

    fn write_report(&mut self, report: &CollectionReport) -> OutputResult<OutputHandle> {
        let uuid = generate_uuid();
        let output_path = self.output_directory.join(format!("report_{uuid}.json"));

        let file =
            File::create(&output_path).map_err(|err| OutputError::io_path(&output_path, err))?;
        serde_json::to_writer(file, report)?;

        Ok(OutputHandle::report(OutputLocation::Local(output_path)))
    }

    fn create_log_file(&mut self) -> OutputResult<LogOutput> {
        let path = self.log_path();
        let file = File::create(&path).map_err(|err| OutputError::io_path(&path, err))?;

        Ok(LogOutput { path, file })
    }

    fn finalize(&mut self) -> OutputResult<()> {
        if !self.compress {
            return Ok(());
        }

        self.compress_final_output()
    }
}

#[cfg(test)]
mod tests {
    use crate::output::sink::{
        local::LocalSink, output_handle::OutputType, output_sink::OutputSink,
    };
    use crate::structs::toml::OutputConfig;
    use std::{io::Write, path::PathBuf};

    #[test]
    fn test_local_sink() {
        let mut config = OutputConfig::default();
        config.directory = PathBuf::from("./tmp");
        config.name = String::from("local_sink");
        config.compress = true;

        let mut encode = |writer: &mut dyn Write| {
            writer.write_all(br#"{"pid":1}"#)?;
            writer.write_all(b"\n")?;
            Ok(1)
        };
        let mut sink = LocalSink::new(&config).unwrap();
        let handle = sink
            .write_artifact("test", "jsonl", "application/jsonl", &mut encode)
            .unwrap();

        assert_eq!(handle.compressed, true);
        assert_eq!(handle.output_type, OutputType::Artifact);
        assert_eq!(handle.extension, "jsonl");

        sink.finalize().unwrap();
    }
}
