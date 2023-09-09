use crate::utils::strings::extract_utf16_string;
use nom::bytes::complete::{take, take_until};
use std::mem::size_of;

/// Determine if extra Shim data exists in `Shortcut` data
pub(crate) fn has_shim(data: &[u8]) -> (bool, String) {
    let result = parse_shim(data);
    match result {
        Ok((_, shim)) => (true, shim),
        Err(_err) => (false, String::new()),
    }
}

/// Parse `Shortcut` Shim info
fn parse_shim(data: &[u8]) -> nom::IResult<&[u8], String> {
    let sig = [8, 0, 0, 160];
    let (_, sig_start) = take_until(sig.as_slice())(data)?;

    let adjust_start = 4;
    let (shim_data, _) = take(sig_start.len() - adjust_start)(data)?;
    let (input, _size_data) = take(size_of::<u32>())(shim_data)?;
    let (input, _sig_data) = take(size_of::<u32>())(input)?;

    let (input, string_data) = take_until([0, 0].as_slice())(input)?;
    let shim_string = extract_utf16_string(string_data);

    Ok((input, shim_string))
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::shortcuts::extras::shim::{has_shim, parse_shim};

    #[test]
    fn test_has_shim() {
        // Cant find real lnk file with Shim data. Fake data below
        let test = [
            62, 0, 0, 0, 8, 0, 0, 160, 103, 0, 105, 0, 109, 0, 109, 0, 101, 0, 32, 0, 109, 0, 111,
            0, 114, 0, 101, 0, 32, 0, 108, 0, 110, 0, 107, 0, 115, 0, 33, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let (has_shim, shim) = has_shim(&test);
        assert!(has_shim);
        assert_eq!(shim, "gimme more lnks!");
    }

    #[test]
    fn test_parse_shim() {
        // Cant find real lnk file with Shim data. Fake data below
        let test = [
            62, 0, 0, 0, 8, 0, 0, 160, 103, 0, 105, 0, 109, 0, 109, 0, 101, 0, 32, 0, 109, 0, 111,
            0, 114, 0, 101, 0, 32, 0, 108, 0, 110, 0, 107, 0, 115, 0, 33, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let (_, shim) = parse_shim(&test).unwrap();
        assert_eq!(shim, "gimme more lnks!");
    }
}
