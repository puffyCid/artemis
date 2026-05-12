use super::error::ArtemisError;
use super::nom_helper::nom_unsigned_two_bytes;
use crate::utils::nom_helper::Endian;
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, SecondsFormat, TimeZone, Utc};
use log::error;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Return time now in seconds or 0
pub(crate) fn time_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::new(0, 0))
        .as_secs()
}

/// Convert Windows filetime values to ISO8601 format with millisecond precision
pub(crate) fn filetime_to_iso(filetime: u64) -> String {
    let windows_milliseconds = 10000;
    let seconds_to_unix: i64 = 11644473600000;

    // We should not overflow because of the division.
    let timestamp = (filetime / windows_milliseconds) as i64 - seconds_to_unix;
    let iso_opt = DateTime::from_timestamp_millis(timestamp);
    match iso_opt {
        Some(result) => result.to_rfc3339_opts(SecondsFormat::Millis, true),
        None => String::from("1970-01-01T00:00:00.000Z"),
    }
}

/// Convert macOS Cocoa timestamp to ISO8601 format with millisecond precision
pub(crate) fn cocoatime_to_iso(timestamp: f64) -> String {
    let adjust_to_unix = 978307200.0;
    unixepoch_to_iso_float(timestamp + adjust_to_unix)
}

/// Convert OLE Automation time (sometimes also referred to as Variant time) to ISO8601 format with millisecond precision
pub(crate) fn ole_automationtime_to_iso(oletime: f64) -> String {
    // OLE automation time is just the number of days since Jan 1 1900 as float64
    let hours = 24.0;
    let mins = 60.0;
    let secs = 60.0;
    let adjust_epoch = 2208988800.0;

    // Jan 1 1900 is actually a value of two (2) days instead of one (1) due to some old Lotus bug
    // and Microsoft wanting to be compatible between Excel and Lotus notes
    let adjust_jan1 = 172800.0;

    let mut timestamp = oletime * hours * mins * secs;
    timestamp -= adjust_epoch;
    timestamp -= adjust_jan1;
    unixepoch_to_iso_float(timestamp)
}

/// Convert Windows FAT time (UTC) values to to ISO8601 format with millisecond precision
pub(crate) fn fattime_utc_to_iso(fattime: &[u8]) -> String {
    let minimum_length = 4;
    if fattime.len() < minimum_length {
        return String::from("1970-01-01T00:00:00.000Z");
    }
    let result = get_fat_bits(fattime);
    let (_, (date, time)) = match result {
        Ok(result) => result,
        Err(_err) => {
            error!("[time] Could not get FAT time");
            return String::from("1970-01-01T00:00:00.000Z");
        }
    };

    let day_sec_adjust = 0x1f;
    let month_adjust = 0x1e0;
    let month_min_shift = 5;
    let year_hour_adjust = 0xfe00;
    let year_shift = 9;
    let year_start = 1980;

    let year = ((date & year_hour_adjust) >> year_shift) + year_start;
    let month = (date & month_adjust) >> month_min_shift;
    let day = date & day_sec_adjust;

    if month == 0 || day == 0 {
        return String::from("1970-01-01T00:00:00.000Z");
    }

    let sec_multi = 2;
    let min_adjust = 0x7e0;
    let hour_shift = 11;

    let hour = (time & year_hour_adjust) >> hour_shift;
    let min = (time & min_adjust) >> month_min_shift;
    let second = (time & day_sec_adjust) * sec_multi;

    let year_res = year.try_into();
    let year = match year_res {
        Ok(result) => result,
        Err(_err) => {
            error!(
                "[time] Got an extremely large year for FAT time (max should be 2108). Got: {year}"
            );
            return String::from("1970-01-01T00:00:00.000Z");
        }
    };
    let ymd_opt = NaiveDate::from_ymd_opt(year, month, day);
    let ymd = if let Some(result) = ymd_opt {
        result
    } else {
        error!("[time] Could not get FAT time year month day: {year}-{month}-{day}");
        return String::from("1970-01-01T00:00:00.000Z");
    };

    let hms_opt = NaiveTime::from_hms_opt(hour, min, second);
    let hms = if let Some(result) = hms_opt {
        result
    } else {
        error!("[time] Could not get FAT time hour min sec: {hour}:{min}:{second}");
        return String::from("1970-01-01T00:00:00.000Z");
    };
    let utc = NaiveDateTime::new(ymd, hms);

    // The FAT time is already in UTC format
    let epoch: DateTime<Utc> = DateTime::from_naive_utc_and_offset(utc, Utc);
    epoch.to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// Convert `UnixEpoch` to ISO8601 format with millisecond precision
pub(crate) fn unixepoch_to_iso(timestamp: i64) -> String {
    let iso_opt = DateTime::from_timestamp(timestamp, 0);
    match iso_opt {
        Some(result) => result.to_rfc3339_opts(SecondsFormat::Millis, true),
        None => String::from("1970-01-01T00:00:00.000Z"),
    }
}

/// Convert `UnixEpoch` float value to ISO8601 format with millisecond precision
pub(crate) fn unixepoch_to_iso_float(timestamp: f64) -> String {
    // Calculations performed like:
    // 1. Round to smallest value. -1.5 rounds to -2.0. 1.5 rounds to 1
    // 2. Subtract rounded value from timestamp. -1.5 - -2.0 = .5. 1.5 - 1 = .5
    // 3. Convert timestamp
    let round_value = timestamp.floor();
    let fract_nano = (timestamp - round_value) * 1000000000.0;

    match DateTime::from_timestamp(round_value as i64, fract_nano as u32) {
        Some(result) => result.to_rfc3339_opts(SecondsFormat::Millis, true),
        None => String::from("1970-01-01T00:00:00.000Z"),
    }
}

/// Convert `UnixEpoch` to ISO8601 format with with nanoseconds
pub(crate) fn unixepoch_to_iso_with_nano(timestamp: i64, nanoseconds: i64) -> String {
    let mut nano_value = nanoseconds;
    let mut time_value = timestamp;

    // Handle negative time values
    // Similar to how we handle negative floats above
    if nano_value < 0 {
        time_value -= 1;
        nano_value += 1000000000;
    }

    let iso_opt = DateTime::from_timestamp(time_value, nano_value as u32);
    match iso_opt {
        Some(result) => result.to_rfc3339_opts(SecondsFormat::Millis, true),
        None => String::from("1970-01-01T00:00:00.000Z"),
    }
}

/// Convert `UnixEpoch` nanoseconds to ISO8601 format
pub(crate) fn unixepoch_nanoseconds_to_iso(timestamp: i64) -> String {
    Utc.timestamp_nanos(timestamp)
        .to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
}

/// Convert `UnixEpoch` to ISO8601 format
pub(crate) fn unixepoch_microseconds_to_iso(timestamp: i64) -> String {
    let iso_opt = DateTime::from_timestamp_micros(timestamp);
    match iso_opt {
        Some(result) => result.to_rfc3339_opts(SecondsFormat::Millis, true),
        None => String::from("1970-01-01T00:00:00.000Z"),
    }
}

/// Check if `time1` is greater than `time2`. Timestamps must be in RFC 3339 format. (YYYY-MM-DDTHH:mm:ss.000Z)
pub(crate) fn compare_timestamps(time1: &str, time2: &str) -> Result<bool, ArtemisError> {
    let time1_result = DateTime::parse_from_rfc3339(time1);
    let timestamp1 = match time1_result {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Failed to parse timestamp {time1}: {err:?}");
            return Err(ArtemisError::BadTime);
        }
    };

    let time2_result = DateTime::parse_from_rfc3339(time2);
    let timestamp2 = match time2_result {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Failed to parse timestamp {time2}: {err:?}");
            return Err(ArtemisError::BadTime);
        }
    };

    if timestamp1 > timestamp2 {
        return Ok(true);
    }

    Ok(false)
}

/// Parse the bits in FAT timestamp
fn get_fat_bits(fattime: &[u8]) -> nom::IResult<&[u8], (u32, u32)> {
    let (input, date) = nom_unsigned_two_bytes(fattime, Endian::Le)?;
    let (input, time) = nom_unsigned_two_bytes(input, Endian::Le)?;

    Ok((input, (date as u32, time as u32)))
}

#[cfg(test)]
mod tests {
    use crate::utils::time::{
        cocoatime_to_iso, compare_timestamps, fattime_utc_to_iso, filetime_to_iso, get_fat_bits,
        ole_automationtime_to_iso, time_now, unixepoch_microseconds_to_iso, unixepoch_to_iso,
        unixepoch_to_iso_float, unixepoch_to_iso_with_nano,
    };

    #[test]
    fn test_time_now() {
        let seconds_now = time_now();
        assert!(seconds_now > 100)
    }

    #[test]
    fn test_filetime_to_iso() {
        let test_data = 132244766418940254;
        assert_eq!(filetime_to_iso(test_data), "2020-01-26T01:44:01.894Z")
    }

    #[test]
    fn test_fattime_utc_to_iso() {
        let test_data = [123, 79, 195, 14];
        assert_eq!(fattime_utc_to_iso(&test_data), "2019-11-27T01:54:06.000Z")
    }

    #[test]
    fn test_unixepoch_to_iso() {
        assert_eq!(unixepoch_to_iso(1574819646), "2019-11-27T01:54:06.000Z")
    }

    #[test]
    fn test_unixepoch_microseconds_to_iso() {
        assert_eq!(
            unixepoch_microseconds_to_iso(2500000000000000),
            "2049-03-22T04:26:40.000Z"
        )
    }

    #[test]
    fn test_get_fat_bits() {
        let test_data = [123, 79, 195, 14];
        let (_, (date, time)) = get_fat_bits(&test_data).unwrap();
        assert_eq!(date, 20347);
        assert_eq!(time, 3779);
    }

    #[test]
    fn test_fattime_utc_to_iso_bad() {
        assert_eq!(fattime_utc_to_iso(&[]), "1970-01-01T00:00:00.000Z");
    }

    #[test]
    fn test_ole_automationtime_to_iso() {
        let test = 43794.01875;
        let result = ole_automationtime_to_iso(test);
        assert_eq!(result, "2019-11-25T00:27:00.000Z");
    }

    #[test]
    fn test_ole_auomationtime_to_iso_milli() {
        assert_eq!(
            ole_automationtime_to_iso(45224.75001157),
            "2023-10-25T18:00:00.999Z"
        );
    }

    #[test]
    fn test_cocoatime_to_iso() {
        let test = 10.01875;
        let result = cocoatime_to_iso(test);
        assert_eq!(result, "2001-01-01T00:00:10.018Z");
    }

    #[test]
    fn test_unixepoch_to_iso_float() {
        let test = 1595003382.687535;
        let result = unixepoch_to_iso_float(test);
        assert_eq!(result, "2020-07-17T16:29:42.687Z");
    }

    #[test]
    fn test_unixepoch_to_iso_float_negative() {
        let test = -1.5;
        let result = unixepoch_to_iso_float(test);
        assert_eq!(result, "1969-12-31T23:59:58.500Z");
    }

    #[test]
    fn test_compare_timestamps() {
        let timestamp = unixepoch_to_iso(1574819646);
        let timestamp2 = unixepoch_to_iso(1474819646);

        assert!(compare_timestamps(&timestamp, &timestamp2).unwrap());
    }

    #[test]
    fn test_unixepoch_to_iso_with_nano() {
        let timestamp = unixepoch_to_iso_with_nano(1574819646, 33077000);

        assert_eq!(timestamp, "2019-11-27T01:54:06.033Z");
    }

    #[test]
    fn test_unixepoch_to_iso_with_nano_negative() {
        let timestamp = unixepoch_to_iso_with_nano(-1, -500000000);

        assert_eq!(timestamp, "1969-12-31T23:59:58.500Z");
    }

    #[test]
    fn test_unixepoch_to_iso_with_nano_negative_postive() {
        let timestamp = unixepoch_to_iso_with_nano(1, -500000000);

        assert_eq!(timestamp, "1970-01-01T00:00:00.500Z");
    }
}
