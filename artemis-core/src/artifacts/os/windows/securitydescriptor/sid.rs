use crate::utils::nom_helper::{nom_unsigned_four_bytes, nom_unsigned_one_byte, Endian};
use byteorder::{BigEndian, ReadBytesExt};
use nom::bytes::complete::take;

/**
 * Parse the data into a properly formatted SID
 */
pub(crate) fn grab_sid(data: &[u8]) -> nom::IResult<&[u8], String> {
    let (input, sid_revision) = nom_unsigned_one_byte(data, Endian::Le)?;
    let (input, subauthorities) = nom_unsigned_one_byte(input, Endian::Le)?;
    let authority_size: usize = 6;
    let (mut sid_data, mut authority) = take(authority_size)(input)?;

    let auth = authority.read_i48::<BigEndian>().unwrap_or(0);
    let mut sub_authority_count = 0;
    let mut windows_sid = format!("S-{sid_revision}-{auth}");
    while (sub_authority_count < subauthorities) && !sid_data.is_empty() {
        let (sub_data, subauth_sid) = nom_unsigned_four_bytes(sid_data, Endian::Le)?;
        windows_sid += &format!("-{subauth_sid}");
        sid_data = sub_data;
        sub_authority_count += 1;
    }

    Ok((input, windows_sid))
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::securitydescriptor::sid::grab_sid;

    #[test]
    fn test_grab_sid() {
        let test = [1, 1, 0, 0, 0, 0, 0, 5, 7, 0, 0, 0];
        let (_, results) = grab_sid(&test).unwrap();
        assert_eq!(results, "S-1-5-7");
    }
}
