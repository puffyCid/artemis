use crate::utils::nom_helper::{Endian, nom_unsigned_two_bytes};
use crate::utils::time::{filetime_to_unixepoch, unixepoch_to_iso};
use nom::bytes::complete::take;
use nom::number::complete::le_u64;
use std::mem::size_of;

/// Parse a 0xbeef0026 block. Contains a FILETIME timestamps. Returns: created, accessed, modified UNIXEPOCH
pub(crate) fn parse_beef(data: &[u8]) -> nom::IResult<&[u8], (String, String, String)> {
    let (input, sig_size) = nom_unsigned_two_bytes(data, Endian::Le)?;
    let (remaining_data, input) = take(sig_size)(input)?;

    let (input, _sig_version) = take(size_of::<u16>())(input)?;
    let (input, _signature) = take(size_of::<u32>())(input)?;
    let (input, _unknown) = take(size_of::<u32>())(input)?;

    let (input, created_data) = take(size_of::<u64>())(input)?;
    let (input, modified_data) = take(size_of::<u64>())(input)?;
    let (input, accessed_data) = take(size_of::<u64>())(input)?;
    let (_input, _offset_start) = take(size_of::<u32>())(input)?;

    let (_, create_filetime) = le_u64(created_data)?;
    let (_, mod_filetime) = le_u64(modified_data)?;
    let (_, access_filetime) = le_u64(accessed_data)?;

    let created = unixepoch_to_iso(&filetime_to_unixepoch(&create_filetime));
    let modified = unixepoch_to_iso(&filetime_to_unixepoch(&mod_filetime));
    let accessed = unixepoch_to_iso(&filetime_to_unixepoch(&access_filetime));
    Ok((remaining_data, (created, accessed, modified)))
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::shellitems::beef::beef0026::parse_beef;

    #[test]
    fn test_parse_beef() {
        let test_data = [
            38, 0, 1, 0, 38, 0, 239, 190, 16, 0, 0, 0, 178, 163, 12, 39, 105, 130, 214, 1, 247, 34,
            66, 226, 189, 132, 214, 1, 198, 63, 64, 72, 190, 132, 214, 1, 20, 0, 0, 0,
        ];
        let (_, (created, accessed, modified)) = parse_beef(&test_data).unwrap();
        assert_eq!(created, "2020-09-04T03:11:59.000Z");
        assert_eq!(accessed, "2020-09-07T02:26:24.000Z");
        assert_eq!(modified, "2020-09-07T02:23:33.000Z");
    }
}
