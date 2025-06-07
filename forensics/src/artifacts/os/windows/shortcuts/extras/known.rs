use crate::utils::{
    nom_helper::{Endian, nom_unsigned_four_bytes},
    uuid::format_guid_le_bytes,
};
use nom::bytes::complete::{take, take_until};
use std::mem::size_of;

/// Determine if extra Known Folder data exists in `Shortcut` data
pub(crate) fn has_known(data: &[u8]) -> (bool, String) {
    let result = parse_known(data);
    match result {
        Ok((_, guid)) => (true, guid),
        Err(_err) => (false, String::new()),
    }
}

/// Parse `Shortcut` Known Folder info
fn parse_known(data: &[u8]) -> nom::IResult<&[u8], String> {
    let sig = [11, 0, 0, 160];
    let (_, sig_start) = take_until(sig.as_slice())(data)?;

    let adjust_start = 4;
    let (known_data, _) = take(sig_start.len() - adjust_start)(data)?;
    let (input, _size_data) = take(size_of::<u32>())(known_data)?;
    let (input, _sig_data) = take(size_of::<u32>())(input)?;

    let (input, guid_data) = take(size_of::<u128>())(input)?;

    let guid = format_guid_le_bytes(guid_data);
    // This is supposedly the offset to shellitems in the shellitems signature (0xa000000c)
    let (input, _offset) = nom_unsigned_four_bytes(input, Endian::Le)?;

    Ok((input, guid))
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::shortcuts::extras::known::{has_known, parse_known};

    #[test]
    fn test_has_known() {
        let test = [
            28, 0, 0, 0, 11, 0, 0, 160, 182, 99, 94, 144, 191, 193, 78, 73, 178, 156, 101, 183, 50,
            211, 210, 26, 177, 0, 0, 0, 96, 0, 0, 0, 3, 0, 0, 160, 88, 0, 0, 0, 0, 0, 0, 0, 108,
            104, 45, 105, 120, 110, 51, 110, 50, 109, 120, 53, 108, 50, 48, 0, 22, 86, 91, 84, 155,
            157, 104, 79, 143, 201, 152, 179, 145, 238, 238, 44, 80, 244, 189, 167, 133, 106, 219,
            17, 181, 174, 0, 20, 34, 14, 200, 5, 22, 86, 91, 84, 155, 157, 104, 79, 143, 201, 152,
            179, 145, 238, 238, 44, 80, 244, 189, 167, 133, 106, 219, 17, 181, 174, 0, 20, 34, 14,
            200, 5, 0, 0, 0, 0,
        ];

        let (has_known, known) = has_known(&test);
        assert!(has_known);
        assert_eq!(known, "905e63b6-c1bf-494e-b29c-65b732d3d21a");
    }

    #[test]
    fn test_parse_known() {
        let test = [
            28, 0, 0, 0, 11, 0, 0, 160, 182, 99, 94, 144, 191, 193, 78, 73, 178, 156, 101, 183, 50,
            211, 210, 26, 177, 0, 0, 0, 96, 0, 0, 0, 3, 0, 0, 160, 88, 0, 0, 0, 0, 0, 0, 0, 108,
            104, 45, 105, 120, 110, 51, 110, 50, 109, 120, 53, 108, 50, 48, 0, 22, 86, 91, 84, 155,
            157, 104, 79, 143, 201, 152, 179, 145, 238, 238, 44, 80, 244, 189, 167, 133, 106, 219,
            17, 181, 174, 0, 20, 34, 14, 200, 5, 22, 86, 91, 84, 155, 157, 104, 79, 143, 201, 152,
            179, 145, 238, 238, 44, 80, 244, 189, 167, 133, 106, 219, 17, 181, 174, 0, 20, 34, 14,
            200, 5, 0, 0, 0, 0,
        ];

        let (_, known) = parse_known(&test).unwrap();
        assert_eq!(known, "905e63b6-c1bf-494e-b29c-65b732d3d21a");
    }
}
