use chrono::DateTime;
use log::warn;

/**
 * Determine if the data should be filtered and removed. Returns true if the data should be removed
 */
pub(crate) fn filter_data(datetime: &str, start: Option<&str>, end: Option<&str>) -> bool {
    // If no filtering is being done. We keep our data and do not remove it
    if start.is_none() && end.is_none() {
        return false;
    }

    // Check if the data timestamp falls between our range. If it does we keep it
    if start.is_some_and(|start_filter| {
        let data_timestamp = match DateTime::parse_from_rfc3339(datetime) {
            Ok(result) => result,
            Err(err) => {
                warn!("[timeline] Failed to parse timestamp '{datetime}' for start: {err:?}. Retaining data");
                return true;
            }
        };
        let start_timestamp = match DateTime::parse_from_rfc3339(start_filter) {
            Ok(result) => result,
            Err(err) => {
                warn!("[timeline] Failed to parse start time '{datetime}': {err:?}. Retaining data");
                return true;
            }
        };

        // Keep the data
        if data_timestamp > start_timestamp {
            return true;
        }

        // Filter the data
        false
    }) && end.is_some_and(|end_filter| {
        let data_timestamp = match DateTime::parse_from_rfc3339(datetime) {
            Ok(result) => result,
            Err(err) => {
                warn!("[timeline] Failed to parse timestamp '{datetime}' for end: {err:?}. Retaining data");
                return true;
            }
        };
        let end_timestamp = match DateTime::parse_from_rfc3339(end_filter) {
            Ok(result) => result,
            Err(err) => {
                warn!("[timeline] Failed to parse end time '{datetime}': {err:?}. Retaining data");
                return true;
            }
        };

        // Keep the data
        if data_timestamp < end_timestamp {
            return true;
        }

        // Filter the data
        false
    }) {
        // Keep the data
        return false;
    }

    // Checks if the data timestamp is greater than our start time. If it does we keep it
    if start.is_some_and(|start_filter| {
        let data_timestamp = match DateTime::parse_from_rfc3339(datetime) {
            Ok(result) => result,
            Err(err) => {
                warn!("[timeline] Failed to parse timestamp '{datetime}' for start: {err:?}. Retaining data");
                return true;
            }
        };
        let start_timestamp = match DateTime::parse_from_rfc3339(start_filter) {
            Ok(result) => result,
            Err(err) => {
                warn!("[timeline] Failed to parse start time '{datetime}': {err:?}. Retaining data");
                return true;
            }
        };

        // Keep the data
        if data_timestamp > start_timestamp {
            return false;
        }

        // Filter the data
        true
    }) {
        // Remove the data
        return true;
    }

    // Checks if the data timestamp is less than our end time. If it does we keep it
    if end.is_some_and(|end_filter| {
        let data_timestamp = match DateTime::parse_from_rfc3339(datetime) {
            Ok(result) => result,
            Err(err) => {
                warn!("[timeline] Failed to parse timestamp '{datetime}' for end: {err:?}. Retaining data");
                return true;
            }
        };
        let end_timestamp = match DateTime::parse_from_rfc3339(end_filter) {
            Ok(result) => result,
            Err(err) => {
                warn!("[timeline] Failed to parse end time '{datetime}': {err:?}. Retaining data");
                return true;
            }
        };

        // Keep the data
        if data_timestamp < end_timestamp {
            return false;
        }

       // Remove the data
        true
    }) {
        // Remove the data
        return true;
    }

    // The data falls outside our timestamp filters. We should remove it
    true
}

#[cfg(test)]
mod tests {
    use crate::artifacts::filter::filter_data;

    #[test]
    fn test_filter_data() {
        let start = "1970-01-01T00:00:00.000Z";
        let end = "3000-01-01T00:00:00.000Z";
        let now = "2026-03-01T00:00:00.000Z";
        assert!(!filter_data(now, Some(start), Some(end)))
    }

    #[test]
    fn test_filter_data_start() {
        let start = "5970-01-01T00:00:00.000Z";
        let end = "7000-01-01T00:00:00.000Z";
        let now = "2026-03-01T00:00:00.000Z";
        assert!(filter_data(now, Some(start), Some(end)))
    }

    #[test]
    fn test_filter_data_end() {
        let start = "1970-01-01T00:00:00.000Z";
        let end = "2000-01-01T00:00:00.000Z";
        let now = "2026-03-01T00:00:00.000Z";
        assert!(filter_data(now, Some(start), Some(end)))
    }

    #[test]
    fn test_filter_data_keep() {
        let start = "2026-03-01T00:00:00.000Z";
        let end = "2026-04-01T00:00:00.000Z";
        let now = "2026-03-14T00:00:00.000Z";
        assert!(!filter_data(now, Some(start), Some(end)))
    }

    #[test]
    fn test_filter_data_bad_end() {
        let start = "2026-03-01T00:00:00.000Z";
        let end = "1970-04-01T00:00:00.000Z";
        let now = "2026-03-14T00:00:00.000Z";
        assert!(filter_data(now, Some(start), Some(end)))
    }

    #[test]
    fn test_filter_data_bad_start() {
        let start = "9000-03-01T00:00:00.000Z";
        let end = "2026-04-01T00:00:00.000Z";
        let now = "2026-01-14T00:00:00.000Z";
        assert!(filter_data(now, Some(start), Some(end)))
    }
}
