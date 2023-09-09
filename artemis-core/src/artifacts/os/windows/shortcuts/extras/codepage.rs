use crate::utils::nom_helper::{nom_unsigned_four_bytes, Endian};
use nom::bytes::complete::{take, take_until};
use std::mem::size_of;

/// Determine if extra Codepage data exists in `Shortcut` data
pub(crate) fn has_codepage(data: &[u8]) -> (bool, u32) {
    let result = parse_codepage(data);
    match result {
        Ok((_, codepage)) => (true, codepage),
        Err(_err) => (false, 0),
    }
}

/// Parse `Shortcut` Codepage info
fn parse_codepage(data: &[u8]) -> nom::IResult<&[u8], u32> {
    let sig = [4, 0, 0, 160];
    let (_, sig_start) = take_until(sig.as_slice())(data)?;

    let adjust_start = 4;
    let (code_data, _) = take(sig_start.len() - adjust_start)(data)?;
    let (input, _size_data) = take(size_of::<u32>())(code_data)?;
    let (input, _sig_data) = take(size_of::<u32>())(input)?;

    let (input, codepage) = nom_unsigned_four_bytes(input, Endian::Le)?;
    Ok((input, codepage))
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::shortcuts::extras::codepage::{
        has_codepage, parse_codepage,
    };

    #[test]
    fn test_has_console() {
        // Cant find real lnk file with Codepage data. Fake data below
        let test = [14, 0, 0, 0, 4, 0, 0, 160, 32, 0, 0, 0, 0, 0, 0];

        let (has_codepage, codepage) = has_codepage(&test);
        assert!(has_codepage);

        assert_eq!(codepage, 32);
    }

    #[test]
    fn test_parse_codepage() {
        // Cant find real lnk file with Codepage data. Fake data below
        let test = [14, 0, 0, 0, 4, 0, 0, 160, 32, 0, 0, 0, 0, 0, 0];

        let (_, codepage) = parse_codepage(&test).unwrap();
        assert_eq!(codepage, 32);
    }
}
