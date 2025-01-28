use crate::utils::nom_helper::{nom_unsigned_eight_bytes, nom_unsigned_two_bytes, Endian};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct LoggedStream {
    manager_root_reference: u64,
    usn_index: u64,
    tx_id: u64,
    data_lsn: u64,
    metadata_lsn: u64,
    directory_lsn: u64,
    flags: u16,
}

impl LoggedStream {
    /// Extract transaction stream data
    pub(crate) fn parse_transactional_stream(data: &[u8]) -> nom::IResult<&[u8], LoggedStream> {
        let (input, manager_root_reference) = nom_unsigned_eight_bytes(data, Endian::Le)?;
        let (input, usn_index) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, tx_id) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, data_lsn) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, metadata_lsn) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, directory_lsn) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, flags) = nom_unsigned_two_bytes(input, Endian::Le)?;

        let stream = LoggedStream {
            manager_root_reference,
            usn_index,
            tx_id,
            data_lsn,
            metadata_lsn,
            directory_lsn,
            flags,
        };

        Ok((input, stream))
    }
}

#[cfg(test)]
mod tests {
    use super::LoggedStream;

    #[test]
    fn test_parse_transactional_stream() {
        let test = [
            0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 5, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 98, 135, 0, 27, 0, 0, 0, 2, 0, 0,
            0, 0, 0, 0, 0,
        ];

        let (_, stream) = LoggedStream::parse_transactional_stream(&test).unwrap();
        assert_eq!(stream.manager_root_reference, 1407374883553280);
        assert_eq!(stream.usn_index, 281496451547136);
        assert_eq!(stream.tx_id, 65536);
        assert_eq!(stream.data_lsn, 0);
        assert_eq!(stream.directory_lsn, 7061644215716937728);
    }
}
