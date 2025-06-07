use crate::{
    artifacts::os::macos::spotlight::store::property::parse_variable_size,
    utils::time::{cocoatime_to_unixepoch, unixepoch_to_iso},
};
use nom::{bytes::complete::take, number::complete::le_f64};
use serde_json::{Value, json};
use std::mem::size_of;

/// Extract dates associated with Spotlight property
pub(crate) fn extract_dates<'a>(data: &'a [u8], prop_type: &u8) -> nom::IResult<&'a [u8], Value> {
    let mut dates = Vec::new();
    let multiple_dates = 2;

    if (prop_type & multiple_dates) == multiple_dates {
        let (mut input, multi_dates) = parse_variable_size(data)?;
        let num_values = multi_dates / 8;
        let mut count = 0;
        while count < num_values {
            let (remaining, date_data) = take(size_of::<f64>())(input)?;
            let (_, mac_date) = le_f64(date_data)?;
            let unix_epoch = unixepoch_to_iso(&cocoatime_to_unixepoch(&mac_date));
            input = remaining;
            count += 1;

            dates.push(unix_epoch);
        }
        return Ok((input, json!(dates)));
    }

    let (input, date_data) = take(size_of::<f64>())(data)?;
    let (_, mac_date) = le_f64(date_data)?;
    let unix_epoch = unixepoch_to_iso(&cocoatime_to_unixepoch(&mac_date));

    dates.push(unix_epoch);

    Ok((input, json!(dates)))
}

#[cfg(test)]
mod tests {
    use super::extract_dates;

    #[test]
    fn test_extract_dates() {
        let prop_type = 64;
        let data = [
            0, 0, 0, 192, 203, 201, 195, 65, 2, 20, 1, 0, 1, 79, 3, 0, 0, 0, 192, 203, 201, 195,
            65, 1, 122, 182, 108, 247, 224, 201, 195, 65, 1, 168, 158, 69, 247, 224, 201, 195, 65,
            1, 0, 1, 0, 3, 0, 1, 0, 1, 122, 182, 108, 247, 224, 201, 195, 65, 2, 0, 1, 0, 0, 0,
            192, 203, 201, 195, 65, 2, 169, 158, 69, 247, 224, 201, 195, 65, 2, 169, 158, 69, 247,
            224, 201, 195, 65, 1, 122, 182, 108, 247, 224, 201, 195, 65, 1, 0, 0, 0, 192, 203, 201,
            195, 65, 4, 7, 98, 111, 111, 116, 22, 2, 0, 1, 7, 98, 111, 111, 116, 22, 2, 0,
        ];

        let (_, result) = extract_dates(&data, &prop_type).unwrap();
        assert_eq!(
            result.as_array().unwrap()[0].as_str().unwrap(),
            "2022-01-16T00:00:00.000Z"
        );
    }
}
