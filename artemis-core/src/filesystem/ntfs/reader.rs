use super::sector_reader::SectorReader;
use log::{error, warn};
use ntfs::{NtfsError, NtfsFile, NtfsReadSeek};
use std::{
    fs::File,
    io::{BufReader, Error, ErrorKind, SeekFrom},
};

/**
 * Read file bytes based on offset and size
 * `offset` - Offset to start reading
 * `bytes` - Number of bytes to read
 * `file_references` - NTFS filereference. Can get via `raw_reader`
 *
 * returns bytes read as Vec<u8>
 */
pub(crate) fn read_bytes(
    offset: &u64,
    bytes: u64,
    ntfs_file: &NtfsFile<'_>,
    fs: &mut BufReader<SectorReader<File>>,
) -> Result<Vec<u8>, NtfsError> {
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
        warn!("[artemis-core] Did not read expected number of bytes. Read {bytes_read} bytes. Wanted: {bytes}");
    }

    Ok(buff_size)
}

#[cfg(test)]
mod tests {
    use super::read_bytes;
    use crate::filesystem::ntfs::{raw_files::raw_reader, setup::setup_ntfs_parser};

    #[test]
    fn test_read_bytes() {
        let mut ntfs_parser = setup_ntfs_parser(&'C').unwrap();
        let result = raw_reader(
            "C:\\Windows\\explorer.exe",
            &ntfs_parser.ntfs,
            &mut ntfs_parser.fs,
        )
        .unwrap();

        let bytes = read_bytes(&0, 50, &result, &mut ntfs_parser.fs).unwrap();
        assert_eq!(bytes.len(), 50);
    }
}
