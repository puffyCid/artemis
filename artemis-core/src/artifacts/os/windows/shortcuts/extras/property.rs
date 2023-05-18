use crate::artifacts::os::windows::propertystore::parser::get_property_guid;
use log::error;
use nom::bytes::complete::{take, take_until};
use std::mem::size_of;

/// Determine if extra Property Store data exists in `Shortcut` data
pub(crate) fn has_property(data: &[u8]) -> (bool, String) {
    let result = parse_property(data);
    match result {
        Ok((_, guid)) => (true, guid),
        Err(_err) => (false, String::new()),
    }
}

/// Scan for Property Store data and parse if exists
fn parse_property(data: &[u8]) -> nom::IResult<&[u8], String> {
    let tracker_sig = [9, 0, 0, 160];
    let (_, sig_start) = take_until(tracker_sig.as_slice())(data)?;

    let adjust_start = 4;
    let (property_tracker, _) = take(sig_start.len() - adjust_start)(data)?;
    let (input, _size_data) = take(size_of::<u32>())(property_tracker)?;
    let (input, _sig_data) = take(size_of::<u32>())(input)?;

    let prop_result = get_property_guid(input);
    match prop_result {
        Ok(guid) => Ok((input, guid)),
        Err(err) => {
            error!("[shortcut] Failed to parse extra property data: {:?}", err);
            Ok((input, String::from("Failed to get extra property data")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_property;
    use crate::artifacts::os::windows::shortcuts::extras::property::has_property;

    #[test]
    fn test_has_property() {
        let test = [
            69, 0, 0, 0, 9, 0, 0, 160, 57, 0, 0, 0, 49, 83, 80, 83, 177, 22, 109, 68, 173, 141,
            112, 72, 167, 72, 64, 46, 164, 61, 120, 140, 29, 0, 0, 0, 104, 0, 0, 0, 0, 72, 0, 0, 0,
            144, 47, 84, 8, 0, 0, 0, 0, 0, 0, 80, 31, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let (has_prop, result) = has_property(&test);
        assert_eq!(has_prop, true);
        assert_eq!(result, "446d16b1-8dad-4870-a748-402ea43d788c");
    }

    #[test]
    fn test_parse_property() {
        let test = [
            69, 0, 0, 0, 9, 0, 0, 160, 57, 0, 0, 0, 49, 83, 80, 83, 177, 22, 109, 68, 173, 141,
            112, 72, 167, 72, 64, 46, 164, 61, 120, 140, 29, 0, 0, 0, 104, 0, 0, 0, 0, 72, 0, 0, 0,
            144, 47, 84, 8, 0, 0, 0, 0, 0, 0, 80, 31, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let (_, result) = parse_property(&test).unwrap();
        assert_eq!(result, "446d16b1-8dad-4870-a748-402ea43d788c");
    }
}
