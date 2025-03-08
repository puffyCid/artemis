use crate::filesystem::error::FileSystemError;
use log::{error, warn};
use ntfs::{NtfsError, NtfsFile, NtfsReadSeek};
use std::io::{BufReader, Error, ErrorKind, Read, Seek, SeekFrom};

/**
 * Read file bytes based on offset and size
 * `offset` - Offset to start reading
 * `bytes` - Number of bytes to read
 * `file_references` - NTFS file reference. Can get via `raw_reader`. If None (non-Windows platforms) it will use standard `BufReader`
 *
 * returns bytes read as Vec<u8>
 */
pub(crate) fn read_bytes<T: std::io::Read + std::io::Seek>(
    offset: &u64,
    bytes: u64,
    ntfs_file_opt: Option<&NtfsFile<'_>>,
    fs: &mut BufReader<T>,
) -> Result<Vec<u8>, NtfsError> {
    if ntfs_file_opt.is_none() {
        let bytes_results = read_bytes_api(offset, bytes, fs);
        let bytes_read = match bytes_results {
            Ok(result) => result,
            Err(err) => {
                error!("[artemis-core] Could not read bytes via API {err:?}");
                return Err(NtfsError::Io(Error::new(
                    ErrorKind::InvalidData,
                    "Could not seek to offset",
                )));
            }
        };

        return Ok(bytes_read);
    }

    let ntfs_file = ntfs_file_opt.unwrap();
    let data_name = "";
    let ntfs_data_option = ntfs_file.data(fs, data_name);
    let ntfs_data_result = match ntfs_data_option {
        Some(result) => result,
        None => return Err(NtfsError::Io(Error::new(ErrorKind::InvalidData, "No data"))),
    };

    let ntfs_data = ntfs_data_result?;
    let ntfs_attribute = ntfs_data.to_attribute()?;

    let mut data_reader = ntfs_attribute.value(fs)?;

    if data_reader.seek(fs, SeekFrom::Start(*offset)).is_err() {
        error!("[artemis-core] Could not seek to offset {offset}");
        return Err(NtfsError::Io(Error::new(
            ErrorKind::InvalidData,
            "Could not seek to offset",
        )));
    }

    let mut buff_size = vec![0u8; bytes as usize];
    let bytes_read = data_reader.read(fs, &mut buff_size)?;

    if bytes_read != buff_size.len() {
        warn!(
            "[artemis-core] Did not read expected number of bytes. Read {bytes_read} bytes. Wanted: {bytes}"
        );
    }

    Ok(buff_size)
}

/// Read bytes from provided `BufReader`
fn read_bytes_api<T: std::io::Read + std::io::Seek>(
    offset: &u64,
    bytes: u64,
    reader: &mut BufReader<T>,
) -> Result<Vec<u8>, FileSystemError> {
    if reader.seek(SeekFrom::Start(*offset)).is_err() {
        error!("[artemis-core] Could not seek to offset {offset} via API");
        return Err(FileSystemError::ReadFile);
    }

    let mut buff_size = vec![0u8; bytes as usize];
    let bytes_read = match reader.read(&mut buff_size) {
        Ok(result) => result,
        Err(err) => {
            error!("[artemis-core] Could not read bytes via API: {err:?}");
            return Err(FileSystemError::ReadFile);
        }
    };

    if bytes_read != buff_size.len() {
        warn!(
            "[artemis-core] Did not read expected number of bytes via API. Wanted {bytes} got {bytes_read}",
        );
    }

    Ok(buff_size)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::read_bytes;
    use crate::filesystem::{
        files::file_reader,
        ntfs::{raw_files::raw_reader, reader::read_bytes_api, setup::setup_ntfs_parser},
    };
    use std::{io::BufReader, path::PathBuf};

    #[test]
    fn test_read_bytes() {
        let mut ntfs_parser = setup_ntfs_parser(&'C').unwrap();
        let result = raw_reader(
            "C:\\Windows\\explorer.exe",
            &ntfs_parser.ntfs,
            &mut ntfs_parser.fs,
        )
        .unwrap();

        let bytes = read_bytes(&0, 50, Some(&result), &mut ntfs_parser.fs).unwrap();
        assert_eq!(bytes.len(), 50);
    }

    #[test]
    fn test_read_bytes_api() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\ese\\win10\\qmgr.db");

        let reader = file_reader(test_location.to_str().unwrap()).unwrap();
        let mut buf_reader = BufReader::new(reader);

        let bytes = read_bytes_api(&0, 50, &mut buf_reader).unwrap();
        assert_eq!(bytes.len(), 50);
    }
}
