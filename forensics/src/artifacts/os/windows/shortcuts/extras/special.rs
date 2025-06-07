use crate::utils::nom_helper::{Endian, nom_unsigned_four_bytes};
use nom::bytes::complete::{take, take_until};
use std::mem::size_of;

/// Determine if extra Special folder data exists in `Shortcut` data
pub(crate) fn has_special(data: &[u8]) -> (bool, u32) {
    let result = parse_special(data);
    match result {
        Ok((_, special)) => (true, special),
        Err(_err) => (false, 0),
    }
}

/// Parse `Shortcut` Special info
fn parse_special(data: &[u8]) -> nom::IResult<&[u8], u32> {
    let sig = [5, 0, 0, 160];
    let (_, sig_start) = take_until(sig.as_slice())(data)?;

    let adjust_start = 4;
    let (special_data, _) = take(sig_start.len() - adjust_start)(data)?;
    let (input, _size_data) = take(size_of::<u32>())(special_data)?;
    let (input, _sig_data) = take(size_of::<u32>())(input)?;

    let (input, special) = nom_unsigned_four_bytes(input, Endian::Le)?;
    // This is supposedly the offset to shellitems in the shellitems signature (0xa000000c)
    let (input, _offset) = nom_unsigned_four_bytes(input, Endian::Le)?;

    Ok((input, special))
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::shortcuts::extras::special::{has_special, parse_special};

    #[test]
    fn test_has_special() {
        let test = [
            22, 0, 0, 0, 5, 0, 0, 160, 38, 0, 0, 0, 177, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let (has_special, special) = has_special(&test);
        assert!(has_special);
        assert_eq!(special, 38);
    }

    #[test]
    fn test_parse_special() {
        let test = [
            22, 0, 0, 0, 5, 0, 0, 160, 38, 0, 0, 0, 177, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let (_, special) = parse_special(&test).unwrap();
        assert_eq!(special, 38);
    }
}
