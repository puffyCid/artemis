use crate::utils::nom_helper::{
    Endian, nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_two_bytes,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct NonResident {
    pub(crate) first_virtual_cluster: u64,
    pub(crate) last_virtual_cluster: u64,
    pub(crate) data_runs_offset: u16,
    pub(crate) compression_size: u16,
    pub(crate) allocated_size: u64,
    /**Not valid if first VCN is non-zero */
    pub(crate) size: u64,
    /**Not valid if first VCN is non-zero */
    pub(crate) valid_size: u64,
    /**If compression size greater than zero */
    pub(crate) total_allocated_size: u64,
}

impl NonResident {
    /// Parse non-Resident MFT metadata
    pub(crate) fn parse_nonresident(data: &[u8]) -> nom::IResult<&[u8], NonResident> {
        let (input, first_virtual_cluster) = nom_unsigned_eight_bytes(data, Endian::Le)?;
        let (input, last_virtual_cluster) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, data_runs_offset) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, compression_size) = nom_unsigned_two_bytes(input, Endian::Le)?;

        let (input, _padding) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let (input, allocated_size) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, size) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, valid_size) = nom_unsigned_eight_bytes(input, Endian::Le)?;

        let mut nonresident = NonResident {
            first_virtual_cluster,
            last_virtual_cluster,
            data_runs_offset,
            compression_size,
            allocated_size,
            size,
            valid_size,
            total_allocated_size: 0,
        };
        if compression_size == 0 {
            return Ok((input, nonresident));
        }
        let (input, total_allocated_size) = nom_unsigned_eight_bytes(input, Endian::Le)?;

        nonresident.total_allocated_size = total_allocated_size;
        Ok((input, nonresident))
    }
}

#[cfg(test)]
mod tests {
    use super::NonResident;

    #[test]
    fn test_parse_nonresident() {
        let test = [
            0, 0, 0, 0, 0, 0, 0, 0, 41, 1, 0, 0, 0, 0, 0, 0, 64, 0, 0, 0, 0, 0, 0, 0, 0, 208, 9, 0,
            0, 0, 0, 0, 0, 208, 9, 0, 0, 0, 0, 0, 0, 208, 9, 0, 0, 0, 0, 0, 50, 8, 1, 141, 172, 0,
            33, 50, 115, 211, 0, 225, 116, 34, 178, 238,
        ];

        let (_, result) = NonResident::parse_nonresident(&test).unwrap();
        assert_eq!(result.allocated_size, 643072);
        assert_eq!(result.compression_size, 0);
        assert_eq!(result.data_runs_offset, 64);
        assert_eq!(result.last_virtual_cluster, 297);
    }
}
