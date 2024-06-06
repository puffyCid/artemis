use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, SecondsFormat, Utc};
use log::error;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Return time now in seconds or 0
pub(crate) fn time_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::new(0, 0))
        .as_secs()
}

/// Convert Windows filetime values to `UnixEpoch`
pub(crate) fn filetime_to_unixepoch(filetime: &u64) -> i64 {
    let windows_nano = 10000000;
    let seconds_to_unix: i64 = 11644473600;

    // We should not overflow because of the division.
    (filetime / windows_nano) as i64 - seconds_to_unix
}

/// Convert macOS Cocoa timestamp to `UnixEpoch` (also called mac time, mach absolute time)
pub(crate) fn cocoatime_to_unixepoch(cocoatime: &f64) -> i64 {
    let adjust_to_unix = 978307200.0;
    (cocoatime + adjust_to_unix) as i64
}

/// Convert macOS HFS+ timestamp to `UnixEpoch`
pub(crate) fn hfs_to_unixepoch(hfstime: &i64) -> i64 {
    let adjust_to_unix = 2082844800;
    hfstime - adjust_to_unix
}

/// Convert OLE Automation time (sometimes also referred to as Variant time) to `UnixEpoch`
pub(crate) fn ole_automationtime_to_unixepoch(oletime: &f64) -> i64 {
    // OLE automation time is just the number of days since Jan 1 1900 as float64
    let hours = 24.0;
    let mins = 60.0;
    let secs = 60.0;
    let adjust_epoch = 2208988800.0;

    // Jan 1 1900 is actually a value of two (2) days instead of one (1) due to some old Lotus bug
    // and Microsoft wanting to be compatible between Excel and Lotus notes
    let adjust_jan1 = 172800.0;

    let mut seconds = oletime * hours * mins * secs;
    seconds -= adjust_epoch;
    seconds -= adjust_jan1;
    seconds as i64
}

/// Convert Webkit time to `UnixEpoch`
pub(crate) fn webkit_time_to_unixepoch(webkittime: &i64) -> i64 {
    let adjust_epoch = 11644473600;
    webkittime - adjust_epoch
}

/// Convert Windows FAT time (UTC) values to `UnixEpoch`
pub(crate) fn fattime_utc_to_unixepoch(fattime: &[u8]) -> i64 {
    let result = get_fat_bits(fattime);
    let (_, (date, time)) = match result {
        Ok(result) => result,
        Err(_err) => {
            error!("[time] Could not get FAT time");
            return 0;
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
            return 0;
        }
    };
    let ymd_opt = NaiveDate::from_ymd_opt(year, month, day);
    let ymd = if let Some(result) = ymd_opt {
        result
    } else {
        error!("[time] Could not get FAT time year month day");
        return 0;
    };

    let hms_opt = NaiveTime::from_hms_opt(hour, min, second);
    let hms = if let Some(result) = hms_opt {
        result
    } else {
        error!("[time] Could not get FAT time hour min sec");
        return 0;
    };
    let utc = NaiveDateTime::new(ymd, hms);

    // The FAT time is already in UTC format
    let epoch: DateTime<Utc> = DateTime::from_naive_utc_and_offset(utc, Utc);
    epoch.timestamp()
}

/// Convert `UnixEpoch` to ISO8601 format
pub(crate) fn unixepoch_to_iso(timestamp: &i64) -> String {
    let iso_opt = DateTime::from_timestamp(*timestamp, 0);
    match iso_opt {
        Some(result) => result.to_rfc3339_opts(SecondsFormat::Millis, true),
        None => String::from("1970-01-01T:00:00:00.000Z"),
    }
}

/// Parse the bits in FAT timestamp
fn get_fat_bits(fattime: &[u8]) -> nom::IResult<&[u8], (u32, u32)> {
    use super::nom_helper::nom_unsigned_two_bytes;
    use crate::utils::nom_helper::Endian;

    let (input, date) = nom_unsigned_two_bytes(fattime, Endian::Le)?;
    let (input, time) = nom_unsigned_two_bytes(input, Endian::Le)?;

    Ok((input, (date as u32, time as u32)))
}

#[cfg(test)]
mod tests {
    use super::{hfs_to_unixepoch, time_now, webkit_time_to_unixepoch};
    use crate::utils::time::{
        cocoatime_to_unixepoch, fattime_utc_to_unixepoch, filetime_to_unixepoch, get_fat_bits,
        ole_automationtime_to_unixepoch, unixepoch_to_iso,
    };

    #[test]
    fn test_time_now() {
        let seconds_now = time_now();
        assert!(seconds_now > 100)
    }

    #[test]
    fn test_filetime_to_unixepoch() {
        let test_data = 132244766418940254;
        assert_eq!(filetime_to_unixepoch(&test_data), 1580003041)
    }

    #[test]
    fn test_fattime_utc_to_unixepoch() {
        let test_data = [123, 79, 195, 14];
        assert_eq!(fattime_utc_to_unixepoch(&test_data), 1574819646)
    }

    #[test]
    fn test_unixepoch_to_iso() {
        assert_eq!(unixepoch_to_iso(&1574819646), "2019-11-27T01:54:06.000Z")
    }

    #[test]
    fn test_get_fat_bits() {
        let test_data = [123, 79, 195, 14];
        let (_, (date, time)) = get_fat_bits(&test_data).unwrap();
        assert_eq!(date, 20347);
        assert_eq!(time, 3779);
    }

    #[test]
    fn test_ole_automationtime_to_unixepoch() {
        let test = 43794.01875;
        let result = ole_automationtime_to_unixepoch(&test);
        assert_eq!(result, 1574641620);
    }

    #[test]
    fn test_cocoatime_to_unixepoch() {
        let test = 10.01875;
        let result = cocoatime_to_unixepoch(&test);
        assert_eq!(result, 978307210);
    }

    #[test]
    fn test_webkit_to_unixepoch() {
        let test = 13289983960;
        let result = webkit_time_to_unixepoch(&test);
        assert_eq!(result, 1645510360);
    }

    #[test]
    fn test_hfs_to_unixepoch() {
        let test = 3453120824;
        let result = hfs_to_unixepoch(&test);
        assert_eq!(result, 1370276024);
    }
}
