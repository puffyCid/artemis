use chrono::{SecondsFormat, TimeZone, Utc};
use leptos::logging::error;

/// Convert UnixEpoch timstamps to RFC3339 format (YYYY-MM-DDTHH:mm:ss)
pub(crate) fn unixepoch_to_rfc(seconds: i64) -> String {
    let utc_result = Utc.timestamp_opt(seconds, 0);
    let naive = match utc_result {
        chrono::LocalResult::Single(result) => result,
        _ => {
            error!("Could not convert {seconds} to timestamp");
            return format!("{seconds}");
        }
    };
    naive.to_rfc3339_opts(SecondsFormat::Secs, true)
}
