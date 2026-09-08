use crate::{accessor::io::reader::AccessorReader, artifacts::os::triage::error::TriageError};
use base16ct::lower::encode_str;
use digest_io::IoWrapper;
use md5::{Digest, Md5};
use std::{
    fs::File,
    io::{Read, Write, copy},
};
use tracing::error;
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

/// Acquire a file and add to triage zip collection
pub(crate) fn grab_file(
    reader: &mut AccessorReader,
    zip: &mut ZipWriter<File>,
) -> Result<String, TriageError> {
    // Read 64MB of data at a time
    let bytes_limit = 1024 * 1024 * 64;
    let mut buf = vec![0; bytes_limit];
    let mut md5 = IoWrapper(Md5::new());
    let method = CompressionMethod::DEFLATE;
    let options = SimpleFileOptions::default().compression_method(method);

    if let Err(err) =
        zip.start_file_from_path(reader.location.full_path().replace(":", ""), options)
    {
        error!("Failed to start file read into zip: {err:?}");
        return Err(TriageError::ReadFile);
    }

    loop {
        let bytes = match reader.read(&mut buf) {
            Ok(result) => result,
            Err(err) => {
                error!("Failed to read all bytes from file: {err:?}");
                return Err(TriageError::ReadFile);
            }
        };
        if bytes == 0 {
            break;
        }

        if bytes < bytes_limit {
            buf = buf[0..bytes].to_vec();
        }
        let _ = copy(&mut buf.as_slice(), &mut md5);
        let _ = copy(&mut buf.as_slice(), zip);
        if bytes < bytes_limit {
            break;
        }
    }
    let hash = md5.0.finalize();
    let mut buf = [0u8; 32];
    let md5_string = encode_str(&hash, &mut buf).unwrap_or_default().to_string();

    Ok(md5_string)
}

/// Write the triage JSON report to the triage zip file
pub(crate) fn write_report(
    zip: &mut ZipWriter<File>,
    report: &mut [u8],
) -> Result<(), TriageError> {
    let method = CompressionMethod::Stored;
    let options = SimpleFileOptions::default().compression_method(method);
    let filename = "acquisition_report.json";

    if let Err(err) = zip.start_file_from_path(filename, options) {
        error!("Failed to start report into zip: {err:?}");
        return Err(TriageError::StartZip);
    }

    if let Err(err) = zip.write_all(report) {
        error!("Failed to write report into zip: {err:?}");
        return Err(TriageError::WriteReport);
    };

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        accessor::{access::Accessor, entry::handle::EntryKind},
        artifacts::os::triage::reader::grab_file,
        structs::toml::{OutputConfig, OutputDestination, OutputFormat},
    };
    use std::{
        fs::{File, create_dir_all},
        path::PathBuf,
    };
    use zip::ZipWriter;

    fn output_options(name: &str, directory: &str, compress: bool) -> OutputConfig {
        OutputConfig {
            name: name.to_string(),
            directory: directory.to_string().into(),
            format: OutputFormat::Jsonl,
            compress,
            endpoint_id: String::from("abcd"),
            destination: OutputDestination::Local,
            ..Default::default()
        }
    }

    #[test]
    fn test_grab_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/quick.toml");

        let output = output_options("triage_test", "./tmp", false);
        create_dir_all(&output.directory).unwrap();
        let file = File::create(format!(
            "{}/{}.zip",
            output.directory.to_str().unwrap(),
            output.name
        ))
        .unwrap();

        let mut zip = ZipWriter::new(file);

        let mut reader = Accessor::with_defaults()
            .open_reader(test_location.to_str().unwrap())
            .unwrap();

        let hash = grab_file(&mut reader, &mut zip).unwrap();
        assert_eq!(hash, "bee488add81fef5a1d751cabc0d707a2");
        zip.finish().unwrap();
    }

    #[test]
    fn test_grab_file_multiple_files_recreate_paths() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/*/*.toml");
        let output = output_options("triage_test_multiple_files", "./tmp", false);
        create_dir_all(&output.directory).unwrap();

        let paths = Accessor::with_defaults()
            .globfs(test_location.to_str().unwrap())
            .unwrap();
        let file = File::create(format!(
            "{}/{}.zip",
            output.directory.to_str().unwrap(),
            output.name
        ))
        .unwrap();

        let mut zip = ZipWriter::new(file);

        for path in paths {
            if path.meta.kind != EntryKind::File {
                continue;
            }

            let mut reader = Accessor::with_defaults()
                .open_reader_handle(path.handle.as_file().unwrap())
                .unwrap();
            let hash = grab_file(&mut reader, &mut zip).unwrap();
            assert!(!hash.is_empty());
        }

        zip.finish().unwrap();
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_grab_file_ntfs() {
        let path = "ntfs:C:\\Windows\\System32\\config\\SOFTWARE";
        let output = output_options("triage_ntfs_acquire_file", "./tmp", false);
        create_dir_all(&output.directory).unwrap();

        let file = File::create(format!(
            "{}/{}.zip",
            output.directory.to_str().unwrap(),
            output.name
        ))
        .unwrap();

        let mut zip = ZipWriter::new(file);
        let mut reader = Accessor::with_defaults().open_reader(path).unwrap();

        let hash = grab_file(&mut reader, &mut zip).unwrap();
        assert!(!hash.is_empty());
    }
}
