use crate::{
    artifacts::os::macos::spotlight::store::property::parse_variable_size,
    utils::time::cocoatime_to_unixepoch,
};
use nom::{bytes::complete::take, number::complete::le_f64};
use serde_json::{json, Value};
use std::mem::size_of;

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
            let unix_epoch = cocoatime_to_unixepoch(&mac_date);
            input = remaining;
            count += 1;

            dates.push(unix_epoch);
        }
        return Ok((input, json!(dates)));
    }
    let (input, date_data) = take(size_of::<f64>())(data)?;
    println!("{date_data:?}");
    let (_, mac_date) = le_f64(date_data)?;
    println!("{mac_date}");
    let unix_epoch = cocoatime_to_unixepoch(&mac_date);

    dates.push(unix_epoch);

    Ok((input, json!(dates)))
}
