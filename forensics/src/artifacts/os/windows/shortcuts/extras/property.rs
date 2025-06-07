use crate::artifacts::os::windows::propertystore::parser::get_property_guid;
use log::error;
use nom::bytes::complete::{take, take_until};
use serde_json::Value;
use std::{collections::HashMap, mem::size_of};

/// Determine if extra Property Store data exists in `Shortcut` data
pub(crate) fn has_property(data: &[u8]) -> (bool, Vec<HashMap<String, Value>>) {
    let result = parse_property(data);
    match result {
        Ok((_, guid)) => (true, guid),
        Err(_err) => (false, Vec::new()),
    }
}

/// Scan for Property Store data and parse if exists
fn parse_property(data: &[u8]) -> nom::IResult<&[u8], Vec<HashMap<String, Value>>> {
    let sig = [9, 0, 0, 160];
    let (_, sig_start) = take_until(sig.as_slice())(data)?;

    let adjust_start = 4;
    let (property_tracker, _) = take(sig_start.len() - adjust_start)(data)?;
    let (input, _size_data) = take(size_of::<u32>())(property_tracker)?;
    let (input, _sig_data) = take(size_of::<u32>())(input)?;

    let prop_result = get_property_guid(input);
    match prop_result {
        Ok(stores) => Ok((input, stores)),
        Err(err) => {
            error!("[shortcut] Failed to parse extra property data: {:?}", err);
            Ok((input, Vec::new()))
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
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_parse_property() {
        let test = [
            69, 0, 0, 0, 9, 0, 0, 160, 57, 0, 0, 0, 49, 83, 80, 83, 177, 22, 109, 68, 173, 141,
            112, 72, 167, 72, 64, 46, 164, 61, 120, 140, 29, 0, 0, 0, 104, 0, 0, 0, 0, 72, 0, 0, 0,
            144, 47, 84, 8, 0, 0, 0, 0, 0, 0, 80, 31, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let (_, result) = parse_property(&test).unwrap();
        assert_eq!(
            result[0].get("value0").unwrap(),
            "08542f90-0000-0000-0000-501f00000000"
        );
    }
}
