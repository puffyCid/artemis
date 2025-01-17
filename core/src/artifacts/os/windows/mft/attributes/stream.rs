use crate::utils::nom_helper::{nom_unsigned_eight_bytes, nom_unsigned_two_bytes, Endian};

#[derive(Debug)]
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
