use chrono::{DateTime, SecondsFormat};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Convert `UnixEpoch` to ISO8601 format
pub(crate) fn unixepoch_to_iso(timestamp: i64) -> String {
    let iso_opt = DateTime::from_timestamp(timestamp, 0);
    match iso_opt {
        Some(result) => result.to_rfc3339_opts(SecondsFormat::Millis, true),
        None => String::from("1970-01-01T00:00:00.000Z"),
    }
}

/// Return time now in seconds or 0
pub(crate) fn time_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::new(0, 0))
        .as_secs()
}

#[cfg(test)]
mod tests {
    use crate::utils::time::{time_now, unixepoch_to_iso};

    #[test]
    fn test_unixepoch_to_iso() {
        assert_eq!(unixepoch_to_iso(1574819646), "2019-11-27T01:54:06.000Z")
    }

    #[test]
    fn test_time_now() {
        let seconds_now = time_now();
        assert!(seconds_now > 100)
    }
}
