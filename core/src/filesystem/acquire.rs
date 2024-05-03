use super::{error::FileSystemError, files::get_filename};
use crate::output::files::local::{AcquireActionLocal, AcquireFileApi};
use crate::output::files::remote::{AcquireActionRemote, AcquireFileApiRemote, RemoteType};
use crate::structs::toml::Output;
use crate::utils::uuid::generate_uuid;
use log::error;
use md5::{Digest, Md5};
use std::io::{copy, Read, Write};

/// Acquire a file using OS APIs
pub(crate) fn acquire_file(path: &str, output: Output) -> Result<(), FileSystemError> {
    let mut acquire = AcquireFileApi {
        path: path.to_string(),
        filename: get_filename(path),
        output,
        md5: String::new(),
    };
    // Read 64MB of data at a time
    let bytes_limit = 1024 * 1024 * 64;

    let reader_result = acquire.reader();
    let mut reader = match reader_result {
        Ok(result) => result,
        Err(_err) => {
            return Err(FileSystemError::ReadFile);
        }
    };

    let compressor_result = acquire.compressor();
    let mut compressor = match compressor_result {
        Ok(result) => result,
        Err(_err) => {
            return Err(FileSystemError::CompressFile);
        }
    };

    let mut buf = vec![0; bytes_limit];
    let mut md5 = Md5::new();

    loop {
        let bytes_read = reader.read(&mut buf);
        if bytes_read.is_err() {
            error!(
                "[artemis-core] Failed to read all bytes from file {path}: {:?}",
                bytes_read.unwrap_err()
            );
            return Err(FileSystemError::ReadFile);
        }

        let bytes = bytes_read.unwrap_or_default();

        if bytes == 0 {
            break;
        }

        if bytes < bytes_limit {
            buf = buf[0..bytes].to_vec();
        }
        let _ = copy(&mut buf.as_slice(), &mut md5);

        let bytes_written = compressor.write_all(&buf);
        if bytes_written.is_err() {
            error!(
                "[artemis-core] Failed to compress all bytes from file {path}: {:?}",
                bytes_written.unwrap_err()
            );
            return Err(FileSystemError::CompressFile);
        }
    }

    let compress_file = compressor.finish();
    if compress_file.is_err() {
        error!(
            "[artemis-core] Could not finish compression: {:?}",
            compress_file.unwrap_err()
        );
        return Err(FileSystemError::CompressedBytes);
    }
    let hash = md5.finalize();
    acquire.md5 = format!("{hash:x}");

    let status = acquire.finish();
    if status.is_err() {
        error!(
            "[artemis-core] Could not finish file acquisition: {:?}",
            status.unwrap_err()
        );
        return Err(FileSystemError::AcquireFile);
    }

    Ok(())
}

/// Acquire a file using OS APIs and upload to remote services
pub(crate) fn acquire_file_remote(path: &str, output: Output) -> Result<(), FileSystemError> {
    let mut acquire = AcquireFileApiRemote {
        path: path.to_string(),
        filename: format!("{}_{}", get_filename(path), generate_uuid()),
        output,
        md5: String::new(),
        remote: RemoteType::Gcp,
        session: String::new(),
        token: String::new(),
    };
    acquire.output.format = String::from("gz");

    // Read 64MB of data at a time
    let bytes_limit = 1024 * 1024 * 64;

    let reader_result = acquire.reader();
    let mut reader = match reader_result {
        Ok(result) => result,
        Err(_err) => {
            return Err(FileSystemError::ReadFile);
        }
    };

    let mut buf = vec![0; bytes_limit];
    let mut md5 = Md5::new();

    let mut bytes_offset = 0;

    let setup_result = acquire.upload_setup();
    if setup_result.is_err() {
        return Err(FileSystemError::UploadSetup);
    }

    let mut upload_bytes = Vec::new();
    loop {
        let bytes_read = reader.read(&mut buf);
        if bytes_read.is_err() {
            error!(
                "[artemis-core] Failed to read all bytes from file {path}: {:?}",
                bytes_read.unwrap_err()
            );
            return Err(FileSystemError::ReadFile);
        }

        let bytes = bytes_read.unwrap_or_default();
        let done = 0;
        if bytes == done {
            break;
        }

        if bytes < bytes_limit {
            buf = buf[0..bytes].to_vec();
        }
        let _ = copy(&mut buf.as_slice(), &mut md5);

        let mut compressor = acquire.compressor();

        let bytes_written = compressor.write_all(&buf);
        if bytes_written.is_err() {
            error!(
                "[artemis-core] Failed to compress all bytes from file {path}: {:?}",
                bytes_written.unwrap_err()
            );
            return Err(FileSystemError::CompressFile);
        }
        let compress_data_result = compressor.finish();
        let compress_data = match compress_data_result {
            Ok(result) => result,
            Err(err) => {
                error!("[artemis-core] Could not finish compression: {err:?}");
                return Err(FileSystemError::CompressedBytes);
            }
        };

        upload_bytes.append(&mut compress_data.clone());

        // Minimum size for resumable GCP uploads
        let min_size = 262144;
        let total_size = if upload_bytes.len() < min_size {
            let hash = md5.clone().finalize();
            acquire.md5 = format!("{hash:x}");
            format!("{}", upload_bytes.len())
        } else {
            // Size is unknown until we finish compressing last bytes of data
            String::from("*")
        };

        let upload_result = acquire.upload(&upload_bytes, &bytes_offset, &total_size);

        if upload_result.is_err() {
            return Err(FileSystemError::AcquireFile);
        }

        let remaining_bytes = upload_bytes.len() % min_size;

        if remaining_bytes != 0 {
            bytes_offset += upload_bytes.len() - remaining_bytes;
            upload_bytes = upload_bytes[upload_bytes.len() - remaining_bytes..].to_vec();
            continue;
        }

        bytes_offset += upload_bytes.len();
        upload_bytes = Vec::new();
    }

    if !upload_bytes.is_empty() {
        let hash = md5.finalize();
        acquire.md5 = format!("{hash:x}");

        // last upload
        let last_result = acquire.upload(
            &upload_bytes,
            &bytes_offset,
            &format!("{}", bytes_offset + upload_bytes.len()),
        );

        if last_result.is_err() {
            return Err(FileSystemError::FinalUpload);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::acquire_file_remote;
    use crate::filesystem::acquire::acquire_file;
    use crate::structs::toml::Output;
    use httpmock::MockServer;
    use serde_json::json;
    use std::path::PathBuf;

    fn output_options(
        name: &str,
        output: &str,
        directory: &str,
        compress: bool,
        port: u16,
    ) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
            compress,
            url: Some(format!("http://127.0.0.1:{port}")),
            api_key: Some(String::from("ewogICJ0eXBlIjogInNlcnZpY2VfYWNjb3VudCIsCiAgInByb2plY3RfaWQiOiAiZmFrZW1lIiwKICAicHJpdmF0ZV9rZXlfaWQiOiAiZmFrZW1lIiwKICAicHJpdmF0ZV9rZXkiOiAiLS0tLS1CRUdJTiBQUklWQVRFIEtFWS0tLS0tXG5NSUlFdndJQkFEQU5CZ2txaGtpRzl3MEJBUUVGQUFTQ0JLa3dnZ1NsQWdFQUFvSUJBUUM3VkpUVXQ5VXM4Y0tqTXpFZll5amlXQTRSNC9NMmJTMUdCNHQ3TlhwOThDM1NDNmRWTXZEdWljdEdldXJUOGpOYnZKWkh0Q1N1WUV2dU5Nb1NmbTc2b3FGdkFwOEd5MGl6NXN4alptU25YeUNkUEVvdkdoTGEwVnpNYVE4cytDTE95UzU2WXlDRkdlSlpxZ3R6SjZHUjNlcW9ZU1c5YjlVTXZrQnBaT0RTY3RXU05HajNQN2pSRkRPNVZvVHdDUUFXYkZuT2pEZkg1VWxncDJQS1NRblNKUDNBSkxRTkZOZTdicjFYYnJoVi8vZU8rdDUxbUlwR1NEQ1V2M0UwRERGY1dEVEg5Y1hEVFRsUlpWRWlSMkJ3cFpPT2tFL1owL0JWbmhaWUw3MW9aVjM0YktmV2pRSXQ2Vi9pc1NNYWhkc0FBU0FDcDRaVEd0d2lWdU5kOXR5YkFnTUJBQUVDZ2dFQkFLVG1qYVM2dGtLOEJsUFhDbFRRMnZwei9ONnV4RGVTMzVtWHBxYXNxc2tWbGFBaWRnZy9zV3FwalhEYlhyOTNvdElNTGxXc00rWDBDcU1EZ1NYS2VqTFMyang0R0RqSTFaVFhnKyswQU1KOHNKNzRwV3pWRE9mbUNFUS83d1hzMytjYm5YaEtyaU84WjAzNnE5MlFjMStOODdTSTM4bmtHYTBBQkg5Q044M0htUXF0NGZCN1VkSHp1SVJlL21lMlBHaElxNVpCemo2aDNCcG9QR3pFUCt4M2w5WW1LOHQvMWNOMHBxSStkUXdZZGdmR2phY2tMdS8ycUg4ME1DRjdJeVFhc2VaVU9KeUtyQ0x0U0QvSWl4di9oekRFVVBmT0NqRkRnVHB6ZjNjd3RhOCtvRTR3SENvMWlJMS80VGxQa3dtWHg0cVNYdG13NGFRUHo3SURRdkVDZ1lFQThLTlRoQ08yZ3NDMkk5UFFETS84Q3cwTzk4M1dDRFkrb2krN0pQaU5BSnd2NURZQnFFWkIxUVlkajA2WUQxNlhsQy9IQVpNc01rdTFuYTJUTjBkcml3ZW5RUVd6b2V2M2cyUzdnUkRvUy9GQ0pTSTNqSitramd0YUE3UW16bGdrMVR4T0ROK0cxSDkxSFc3dDBsN1ZuTDI3SVd5WW8ycVJSSzNqenhxVWlQVUNnWUVBeDBvUXMycmVCUUdNVlpuQXBEMWplcTduNE12TkxjUHZ0OGIvZVU5aVV2Nlk0TWowU3VvL0FVOGxZWlhtOHViYnFBbHd6MlZTVnVuRDJ0T3BsSHlNVXJ0Q3RPYkFmVkRVQWhDbmRLYUE5Z0FwZ2ZiM3h3MUlLYnVRMXU0SUYxRkpsM1Z0dW1mUW4vL0xpSDFCM3JYaGNkeW8zL3ZJdHRFazQ4UmFrVUtDbFU4Q2dZRUF6VjdXM0NPT2xERGNRZDkzNURkdEtCRlJBUFJQQWxzcFFVbnpNaTVlU0hNRC9JU0xEWTVJaVFIYklIODNENGJ2WHEwWDdxUW9TQlNOUDdEdnYzSFl1cU1oZjBEYWVncmxCdUpsbEZWVnE5cVBWUm5LeHQxSWwySGd4T0J2YmhPVCs5aW4xQnpBK1lKOTlVekM4NU8wUXowNkErQ210SEV5NGFaMmtqNWhIakVDZ1lFQW1OUzQrQThGa3NzOEpzMVJpZUsyTG5pQnhNZ21ZbWwzcGZWTEtHbnptbmc3SDIrY3dQTGhQSXpJdXd5dFh5d2gyYnpic1lFZll4M0VvRVZnTUVwUGhvYXJRbllQdWtySk80Z3dFMm81VGU2VDVtSlNaR2xRSlFqOXE0WkIyRGZ6ZXQ2SU5zSzBvRzhYVkdYU3BRdlFoM1JVWWVrQ1pRa0JCRmNwcVdwYklFc0NnWUFuTTNEUWYzRkpvU25YYU1oclZCSW92aWM1bDB4RmtFSHNrQWpGVGV2Tzg2RnN6MUMyYVNlUktTcUdGb09RMHRtSnpCRXMxUjZLcW5ISW5pY0RUUXJLaEFyZ0xYWDR2M0NkZGpmVFJKa0ZXRGJFL0NrdktaTk9yY2YxbmhhR0NQc3BSSmoyS1VrajFGaGw5Q25jZG4vUnNZRU9OYndRU2pJZk1Qa3Z4Ris4SFE9PVxuLS0tLS1FTkQgUFJJVkFURSBLRVktLS0tLVxuIiwKICAiY2xpZW50X2VtYWlsIjogImZha2VAZ3NlcnZpY2VhY2NvdW50LmNvbSIsCiAgImNsaWVudF9pZCI6ICJmYWtlbWUiLAogICJhdXRoX3VyaSI6ICJodHRwczovL2FjY291bnRzLmdvb2dsZS5jb20vby9vYXV0aDIvYXV0aCIsCiAgInRva2VuX3VyaSI6ICJodHRwczovL29hdXRoMi5nb29nbGVhcGlzLmNvbS90b2tlbiIsCiAgImF1dGhfcHJvdmlkZXJfeDUwOV9jZXJ0X3VybCI6ICJodHRwczovL3d3dy5nb29nbGVhcGlzLmNvbS9vYXV0aDIvdjEvY2VydHMiLAogICJjbGllbnRfeDUwOV9jZXJ0X3VybCI6ICJodHRwczovL3d3dy5nb29nbGVhcGlzLmNvbS9yb2JvdC92MS9tZXRhZGF0YS94NTA5L2Zha2VtZSIsCiAgInVuaXZlcnNlX2RvbWFpbiI6ICJnb29nbGVhcGlzLmNvbSIKfQo=")),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new())
        }
    }

    #[test]
    fn test_acquire_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/unix/bash/bash_history");

        let out = output_options("acquire_file", "local", "./tmp", false, 0);

        acquire_file(&test_location.display().to_string(), out).unwrap();
    }

    #[test]
    #[should_panic(expected = "ReadFile")]
    fn test_acquire_bad_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/unix/bash/bash_historyadsfasdfsdf");

        let out = output_options("acquire_file", "local", "./tmp", false, 0);

        acquire_file(&test_location.display().to_string(), out).unwrap();
    }

    #[test]
    fn test_acquire_file_gcp() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/unix/bash/bash_history");

        let server = MockServer::start();
        let port = server.port();

        let mock_me = server.mock(|when, then| {
            when.any_request();
            then.status(200)
                .header("content-type", "application/json")
                .header("Location", format!("http://127.0.0.1:{port}"))
                .json_body(json!({ "timeCreated": "whatever", "name":"mockme" }));
        });

        let out = output_options("acquire_file", "gcp", "./tmp", false, port);

        acquire_file_remote(test_location.to_str().unwrap(), out).unwrap();

        mock_me.assert_hits(5);
    }
}
