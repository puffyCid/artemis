use super::strings::StringResource;
use common::windows::EventLogRecord;
use std::collections::HashMap;

/// Combine eventlog strings with the template strings
pub(crate) fn add_message_strings(
    log: &EventLogRecord,
    resources: &HashMap<String, StringResource>,
    cache: &mut HashMap<String, StringResource>,
) -> String {
    String::new()
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::eventlogs::strings::get_resources, filesystem::files::read_file,
    };
    use common::windows::EventLogRecord;
    use std::path::PathBuf;

    #[test]
    fn test_add_message_strings_complex() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/eventlogs/complex_log.json");

        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let log: EventLogRecord = serde_json::from_slice(&data).unwrap();
        let resources = get_resources().unwrap();
        for value in resources.values() {
            for reg in &value.registry_info {
                if !reg
                    .registry_key
                    .ends_with("Microsoft-Windows-Security-Auditing")
                {
                    continue;
                }

                println!("{value:?}");
            }
        }
    }
}
