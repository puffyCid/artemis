use chrono::DateTime;

/**
 * Determine if the data should be filtered and removed. Returns true if the data should be removed
 */
pub(crate) fn filter_data(datetime: &str, start: &Option<String>, end: &Option<String>) -> bool {
    // If no filtering is being done. We keep our data and do not remove it
    if start.is_none() && end.is_none() {
        return false;
    }

    // Check if the data timestamp falls between our range. If it does we keep it
    if start.as_ref().is_some_and(|start_filter| {
        let data_timestamp = match DateTime::parse_from_rfc3339(datetime) {
            Ok(result) => result,
            Err(_err) => return false,
        };
        let start_timestamp = match DateTime::parse_from_rfc3339(start_filter) {
            Ok(result) => result,
            Err(_err) => return false,
        };

        // Keep the data
        if data_timestamp > start_timestamp {
            return true;
        }

        // Filter the data
        false
    }) && end.as_ref().is_some_and(|end_filter| {
        let data_timestamp = match DateTime::parse_from_rfc3339(datetime) {
            Ok(result) => result,
            Err(_err) => return false,
        };
        let end_timestamp = match DateTime::parse_from_rfc3339(end_filter) {
            Ok(result) => result,
            Err(_err) => return false,
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
    if start.as_ref().is_some_and(|start_filter| {
        let data_timestamp = match DateTime::parse_from_rfc3339(datetime) {
            Ok(result) => result,
            Err(_err) => return false,
        };
        let start_timestamp = match DateTime::parse_from_rfc3339(start_filter) {
            Ok(result) => result,
            Err(_err) => return false,
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
    if end.as_ref().is_some_and(|end_filter| {
        let data_timestamp = match DateTime::parse_from_rfc3339(datetime) {
            Ok(result) => result,
            Err(_err) => return false,
        };
        let end_timestamp = match DateTime::parse_from_rfc3339(end_filter) {
            Ok(result) => result,
            Err(_err) => return false,
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

    // The data falls within our timestamp filters. We should keep it
    false
}

#[cfg(test)]
mod tests {
    use crate::artifacts::filter::filter_data;

    #[test]
    fn test_filter_data() {
        let start = String::from("1970-01-01T00:00:00.000Z");
        let end = String::from("3000-01-01T00:00:00.000Z");
        let now = "2026-03-01T00:00:00.000Z";
        assert!(!filter_data(now, &Some(start), &Some(end)))
    }

    #[test]
    fn test_filter_data_start() {
        let start = String::from("5970-01-01T00:00:00.000Z");
        let end = String::from("7000-01-01T00:00:00.000Z");
        let now = "2026-03-01T00:00:00.000Z";
        assert!(filter_data(now, &Some(start), &Some(end)))
    }

    #[test]
    fn test_filter_data_end() {
        let start = String::from("1970-01-01T00:00:00.000Z");
        let end = String::from("2000-01-01T00:00:00.000Z");
        let now = "2026-03-01T00:00:00.000Z";
        assert!(filter_data(now, &Some(start), &Some(end)))
    }

    #[test]
    fn test_filter_data_keep() {
        let start = String::from("2026-03-01T00:00:00.000Z");
        let end = String::from("2026-04-01T00:00:00.000Z");
        let now = "2026-03-14T00:00:00.000Z";
        assert!(!filter_data(now, &Some(start), &Some(end)))
    }

    #[test]
    fn test_filter_data_bad_end() {
        let start = String::from("2026-03-01T00:00:00.000Z");
        let end = String::from("1970-04-01T00:00:00.000Z");
        let now = "2026-03-14T00:00:00.000Z";
        assert!(filter_data(now, &Some(start), &Some(end)))
    }

    #[test]
    fn test_filter_data_bad_start() {
        let start = String::from("9000-03-01T00:00:00.000Z");
        let end = String::from("2026-04-01T00:00:00.000Z");
        let now = "2026-01-14T00:00:00.000Z";
        assert!(filter_data(now, &Some(start), &Some(end)))
    }

    #[test]
    fn test_filter_data_no_end() {
        let start = String::from("2025-05-13T00:00:00.000Z");
        let now = "2026-05-09T02:17:16.283Z";
        assert!(!filter_data(now, &Some(start), &None))
    }

    #[test]
    fn test_filter_data_no_end_no_start() {
        let start = String::from("2025-05-13T00:00:00.000Z");
        let now = "2023-05-09T02:17:16.283Z";
        assert!(filter_data(now, &Some(start), &None))
    }

    #[test]
    fn test_filter_data_no_start() {
        let end = String::from("2026-04-01T00:00:00.000Z");
        let now = "2026-07-14T00:00:00.000Z";
        assert!(filter_data(now, &None, &Some(end)))
    }

    #[test]
    fn test_filter_data_bad_time() {
        let end = String::from("dsfasdfasdf00Z");
        let now = "2026-01-14T00:00:00.000Z";
        assert!(!filter_data(now, &None, &Some(end)))
    }
}
