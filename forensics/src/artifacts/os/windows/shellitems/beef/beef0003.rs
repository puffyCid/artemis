use crate::utils::{
    nom_helper::{Endian, nom_unsigned_two_bytes},
    uuid::format_guid_le_bytes,
};
use nom::{Needed, bytes::complete::take};

/// Extract GUID from beef0003 structure
pub(crate) fn parse_beef(data: &[u8]) -> nom::IResult<&[u8], String> {
    let (input, sig_size) = nom_unsigned_two_bytes(data, Endian::Le)?;

    // Size includes size itself
    let adjust_size = 2;
    if sig_size < adjust_size {
        return Err(nom::Err::Incomplete(Needed::Unknown));
    }

    let (remaining_data, input) = take(sig_size - adjust_size)(input)?;
    let (input, _version) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, _signature) = take(size_of::<u32>())(input)?;

    let guid_size: u8 = 16;
    let (input, guid_bytes) = take(guid_size)(input)?;
    let guid = format_guid_le_bytes(guid_bytes);

    // Always 0x16?
    let (_, _version_offset) = nom_unsigned_two_bytes(input, Endian::Le)?;
    Ok((remaining_data, guid))
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::shellitems::beef::beef0003::parse_beef;

    #[test]
    fn test_parse_beef() {
        let test = [
            26, 0, 0, 0, 3, 0, 239, 190, 96, 53, 57, 255, 167, 194, 207, 17, 191, 244, 68, 69, 83,
            84, 0, 0, 22, 0,
        ];
        let (_, item) = parse_beef(&test).unwrap();
        assert_eq!(item, "ff393560-c2a7-11cf-bff4-444553540000");
    }
}
