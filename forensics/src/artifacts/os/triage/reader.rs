use crate::{
    artifacts::os::triage::error::TriageError, filesystem::ntfs::sector_reader::SectorReader,
};
use log::error;
use md5::{Digest, Md5};
use ntfs::{NtfsError, NtfsFile, NtfsReadSeek};
use std::io::{BufReader, Error, ErrorKind, Read, Write, copy};
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

pub(crate) struct TriageReader<T: std::io::Seek + std::io::Read, W: std::io::Seek + std::io::Write>
{
    pub(crate) fs: Option<BufReader<T>>,
    pub(crate) zip: ZipWriter<W>,
    pub(crate) path: String,
}

impl<T: std::io::Seek + std::io::Read, W: std::io::Seek + std::io::Write> TriageReader<T, W> {
    /// Acquire a file and add to triage zip collection
    pub(crate) fn acquire_file(&mut self) -> Result<String, TriageError> {
        if self.fs.is_none() {
            return Err(TriageError::NoReader);
        }
        // Read 64MB of data at a time
        let bytes_limit = 1024 * 1024 * 64;
        let mut buf = vec![0; bytes_limit];
        let mut md5 = Md5::new();
        let method = CompressionMethod::DEFLATE;
        let options = SimpleFileOptions::default().compression_method(method);
        if let Err(err) = self.zip.start_file_from_path(&self.path, options) {
            error!("[triage] Failed to start file read into zip: {err:?}");
            return Err(TriageError::StartZip);
        }

        loop {
            // Unwrap is safe since we check to make it is set above
            let bytes = match self.fs.as_mut().unwrap().read(&mut buf) {
                Ok(result) => result,
                Err(err) => {
                    error!("[triage] Failed to read all bytes from file: {err:?}");
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
            let _ = copy(&mut buf.as_slice(), &mut self.zip);
        }
        let hash = format!("{:x}", md5.finalize());
        Ok(hash)
    }

    /// Acquire a file by parsing the NTFS filesystem. Will bypass locked files
    pub(crate) fn acquire_file_ntfs(
        &mut self,
        ntfs: &NtfsFile<'_>,
        fs: &mut BufReader<SectorReader<std::fs::File>>,
    ) -> Result<String, NtfsError> {
        let data_name = "";
        let ntfs_data_option = ntfs.data(fs, data_name);
        let ntfs_data_result = match ntfs_data_option {
            Some(result) => result,
            None => return Err(NtfsError::Io(Error::new(ErrorKind::InvalidData, "No data"))),
        };

        let ntfs_data = ntfs_data_result?;
        let ntfs_attribute = ntfs_data.to_attribute()?;

        let mut data_reader = ntfs_attribute.value(fs)?;
        // Read 64MB of data at a time
        let bytes_limit = 1024 * 1024 * 64;
        let mut buf = vec![0; bytes_limit];
        let mut md5 = Md5::new();
        let method = CompressionMethod::DEFLATE;
        let options = SimpleFileOptions::default().compression_method(method);
        if let Err(err) = self.zip.start_file_from_path(&self.path, options) {
            error!("[triage] Failed to start file read into zip: {err:?}");
            return Err(NtfsError::Io(Error::new(
                ErrorKind::InvalidData,
                "Failed to start zip writer",
            )));
        }

        loop {
            // Unwrap is safe since we check to make it is set above
            let bytes = match data_reader.read(fs, &mut buf) {
                Ok(result) => result,
                Err(err) => {
                    error!("[triage] Failed to read all bytes from file: {err:?}");
                    return Err(NtfsError::Io(Error::new(
                        ErrorKind::InvalidData,
                        "Failed to read all data",
                    )));
                }
            };
            if bytes == 0 {
                break;
            }

            if bytes < bytes_limit {
                buf = buf[0..bytes].to_vec();
            }
            let _ = copy(&mut buf.as_slice(), &mut md5);
            let _ = copy(&mut buf.as_slice(), &mut self.zip);
        }
        let hash = format!("{:x}", md5.finalize());
        Ok(hash)
    }

    /// Write the triage JSON report to the triage zip file
    pub(crate) fn write_report(&mut self, report: &mut [u8]) -> Result<(), TriageError> {
        let method = CompressionMethod::Stored;
        let options = SimpleFileOptions::default().compression_method(method);
        let filename = "acquisition_report.json";
        if let Err(err) = self.zip.start_file_from_path(filename, options) {
            error!("[triage] Failed to start report into zip: {err:?}");
            return Err(TriageError::StartZip);
        }
        if let Err(err) = self.zip.write_all(report) {
            error!("[triage] Failed to write report into zip: {err:?}");
            return Err(TriageError::WriteReport);
        };

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::triage::reader::TriageReader, filesystem::metadata::glob_paths,
        structs::toml::Output,
    };
    use std::{
        fs::{File, create_dir_all},
        io::BufReader,
        path::PathBuf,
    };
    use zip::ZipWriter;

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
            compress,
            endpoint_id: String::from("abcd"),
            output: output.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_acquire_file_recreate_paths() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/quick.toml");

        let output = output_options("triage_test", "local", "./tmp", false);
        create_dir_all(&output.directory).unwrap();
        let file = File::create(format!("{}/{}.zip", output.directory, output.name)).unwrap();
        let zip = ZipWriter::new(file);
        let reader = File::open(test_location.to_str().unwrap()).unwrap();
        let buf = BufReader::new(reader);
        let mut acq = TriageReader {
            fs: Some(buf),
            zip,
            path: test_location.to_str().unwrap().to_string(),
        };
        let hash = acq.acquire_file().unwrap();
        assert_eq!(hash, "a6d4d85e832a17e230842de55e4f0ccc");
        acq.zip.finish().unwrap();
    }

    #[test]
    fn test_acquire_multiple_files_recreate_paths() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/*/*.toml");
        let output = output_options("triage_test_multiple_files", "local", "./tmp", false);
        create_dir_all(&output.directory).unwrap();
        let paths = glob_paths(test_location.to_str().unwrap()).unwrap();
        let file = File::create(format!("{}/{}.zip", output.directory, output.name)).unwrap();

        let zip = ZipWriter::new(file);
        let mut acq = TriageReader {
            fs: None,
            zip,
            path: String::new(),
        };
        for path in paths {
            if !path.is_file {
                continue;
            }
            let reader = File::open(&path.full_path).unwrap();
            let buf = BufReader::new(reader);
            acq.fs = Some(buf);
            acq.path = path.full_path;
            let hash = acq.acquire_file().unwrap();
            assert!(!hash.is_empty());
        }

        acq.zip.finish().unwrap();
    }

    #[test]
    fn test_acquire_file_filename_only() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/quick.toml");

        let output = output_options("triage_test_filename_only", "local", "./tmp", false);
        create_dir_all(&output.directory).unwrap();
        let file = File::create(format!("{}/{}.zip", output.directory, output.name)).unwrap();
        let zip = ZipWriter::new(file);
        let reader = File::open(test_location.to_str().unwrap()).unwrap();
        let buf = BufReader::new(reader);
        let mut acq = TriageReader {
            fs: Some(buf),
            zip,
            path: String::from("quick.toml"),
        };
        let hash = acq.acquire_file().unwrap();
        assert_eq!(hash, "a6d4d85e832a17e230842de55e4f0ccc");
        acq.zip.finish().unwrap();
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_acquire_file_ntfs() {
        use crate::filesystem::ntfs::{raw_files::raw_reader, setup::setup_ntfs_parser};

        let path = "C:\\Windows\\System32\\config\\SOFTWARE";
        let output = output_options("triage_ntfs_acquire_file", "local", "./tmp", false);
        create_dir_all(&output.directory).unwrap();
        let file = File::create(format!("{}/{}.zip", output.directory, output.name)).unwrap();

        let zip = ZipWriter::new(file);
        let mut acq: TriageReader<File, File> = TriageReader {
            fs: None,
            zip,
            path: path.to_string(),
        };
        let mut ntfs_parser = setup_ntfs_parser('C').unwrap();
        let ntfs_file = raw_reader(path, &ntfs_parser.ntfs, &mut ntfs_parser.fs).unwrap();
        let hash = acq
            .acquire_file_ntfs(&ntfs_file, &mut ntfs_parser.fs)
            .unwrap();
        assert!(!hash.is_empty());
    }
}
