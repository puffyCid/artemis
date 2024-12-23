use self::metadata::get_timestamps;

use super::{
    actions::{
        command::parse_action_run_command, log::parse_action_log,
        send_email::parse_action_send_email, send_notification::parse_action_send_notification,
    },
    util::get_dictionary_values,
};
use crate::{
    artifacts::os::macos::plist::{
        error::PlistError,
        property_list::{get_boolean, get_string},
    },
    filesystem::{files::list_files, metadata},
};
use common::macos::{Actions, EmondData};
use log::{error, warn};
use plist::Value;

/// Parse all Emond rules files at provided path
pub(crate) fn parse_emond_rules(path: &str) -> Result<Vec<EmondData>, PlistError> {
    let rules_result = list_files(path);
    let rules = match rules_result {
        Ok(result) => result,
        Err(err) => {
            error!("[emond] Failed to read Emond rules directory {path}: {err:?}");
            return Err(PlistError::File);
        }
    };
    let mut emond_results: Vec<EmondData> = Vec::new();
    for rule in rules {
        let emond_data_results = parse_emond_data(&rule);
        let mut emond_data = match emond_data_results {
            Ok(results) => results,
            Err(err) => {
                error!("[emond] Failed to parse Emond file: {rule}. Error: {err:?}");
                continue;
            }
        };
        emond_results.append(&mut emond_data);
    }
    Ok(emond_results)
}

/// Parse a single Emond rule file
pub(crate) fn parse_emond_data(path: &str) -> Result<Vec<EmondData>, PlistError> {
    let mut emond_data_vec: Vec<EmondData> = Vec::new();
    let emond_plist_result = plist::from_file(path);
    let emond_plist = match emond_plist_result {
        Ok(result) => result,
        Err(err) => {
            error!("[emond] Failed to parse Emond PLIST Rule: {err:?}");
            return Err(PlistError::File);
        }
    };

    let meta = get_timestamps(path);

    if let Value::Array(plist_array) = emond_plist {
        let mut emond_data = EmondData {
            name: String::new(),
            enabled: false,
            event_types: Vec::new(),
            command_actions: Vec::new(),
            log_actions: Vec::new(),
            send_email_actions: Vec::new(),
            send_sms_actions: Vec::new(),
            send_notification_actions: Vec::new(),
            criterion: Vec::new(),
            variables: Vec::new(),
            allow_partial_criterion_match: false,
            start_time: String::from("1970-01-01T00:00:00.000Z"),
            emond_clients_enabled: false,
            source_file: path.to_string(),
            plist_created: String::from("1970-01-01T00:00:00.000Z"),
            plist_accessed: String::from("1970-01-01T00:00:00.000Z"),
            plist_changed: String::from("1970-01-01T00:00:00.000Z"),
            plist_modified: String::from("1970-01-01T00:00:00.000Z"),
        };

        if let Ok(timestamps) = meta {
            emond_data.plist_created = timestamps.created;
            emond_data.plist_accessed = timestamps.accessed;
            emond_data.plist_changed = timestamps.changed;
            emond_data.plist_modified = timestamps.modified;
        }

        for plist_values in plist_array {
            match plist_values {
                Value::Dictionary(plist_dictionary) => {
                    // Get the data in the Rule
                    for (key, value) in plist_dictionary {
                        if key == "eventTypes" {
                            emond_data.event_types = parse_event_types(&value)?;
                        } else if key == "enabled" {
                            emond_data.enabled = get_boolean(&value)?;
                        } else if key == "allowPartialCriterionMatch" {
                            emond_data.allow_partial_criterion_match = get_boolean(&value)?;
                        } else if key == "criterion" {
                            emond_data.criterion = get_dictionary_values(value);
                        } else if key == "startTime" {
                            emond_data.start_time = get_string(&value)?;
                        } else if key == "variables" {
                            emond_data.variables = get_dictionary_values(value);
                        } else if key == "name" {
                            emond_data.name = get_string(&value)?;
                        } else if key == "actions" {
                            let actions_results = parse_actions(&value);
                            let actions = match actions_results {
                                Ok(results) => results,
                                Err(err) => {
                                    warn!("[emond] Failed to parse Emond Action data: {err:?}");
                                    continue;
                                }
                            };

                            emond_data.log_actions = actions.log_actions;
                            emond_data.command_actions = actions.command_actions;
                            emond_data.send_email_actions = actions.send_email_actions;
                            emond_data.send_notification_actions = actions.send_notification;
                            emond_data.send_sms_actions = actions.send_sms_action;
                        } else {
                            warn!("[emond] Unknown key ({key}) in Emond Rule. Value: {value:?}");
                        }
                    }
                }
                _ => continue,
            }
        }

        emond_data.emond_clients_enabled = check_clients();
        emond_data_vec.push(emond_data);
    } else {
        warn!("[emond] Failed to get Emond Rule Array value");
        return Err(PlistError::Array);
    }

    Ok(emond_data_vec)
}

/// Get the `Emond` type
fn parse_event_types(value: &Value) -> Result<Vec<String>, PlistError> {
    let mut event_types_vec: Vec<String> = Vec::new();
    if let Some(events) = value.as_array() {
        for event in events {
            let event_type_string = get_string(event)?;
            event_types_vec.push(event_type_string);
        }
        Ok(event_types_vec)
    } else {
        error!("[emond] Failed to parse Emond Event Types");
        Err(PlistError::String)
    }
}

/// Parse all `Emond` Actions
fn parse_actions(value: &Value) -> Result<Actions, PlistError> {
    let mut emond_actions = Actions {
        command_actions: Vec::new(),
        log_actions: Vec::new(),
        send_email_actions: Vec::new(),
        send_sms_action: Vec::new(),
        send_notification: Vec::new(),
    };

    let value_array = if let Some(results) = value.as_array() {
        results
    } else {
        error!("[emond] Failed to parse Action array");
        return Err(PlistError::Array);
    };

    for value_data in value_array {
        let action_dictionary = if let Some(results) = value_data.as_dictionary() {
            results
        } else {
            error!("[emond] Failed to parse Action Dictionary");
            return Err(PlistError::Dictionary);
        };

        for (key, action_value) in action_dictionary {
            if key != "type" {
                continue;
            }
            let action_type = get_string(action_value)?;

            match action_type.as_str() {
                "Log" => {
                    let log_data = parse_action_log(action_dictionary);
                    emond_actions.log_actions.push(log_data);
                }
                "RunCommand" => {
                    let command_data = parse_action_run_command(action_dictionary);
                    emond_actions.command_actions.push(command_data);
                }
                "SendEmail" | "SendSMS" => {
                    let email_data = parse_action_send_email(action_dictionary);
                    emond_actions.send_sms_action.push(email_data);
                }
                "SendNotification" => {
                    let notification_data = parse_action_send_notification(action_dictionary);
                    emond_actions.send_notification.push(notification_data);
                }
                _ => warn!("[emond] Unknown Action Type: {action_type}"),
            }
        }
    }
    Ok(emond_actions)
}

/// Check for any files in `EmondClients` directory.
/// If any file exists in `emondClients` then the the emond daemon is started
fn check_clients() -> bool {
    let client_path = "/private/var/db/emondClients";
    let clients_result = list_files(client_path);
    let clients = match clients_result {
        Ok(result) => result,
        Err(err) => {
            error!("[emond] Failed to read Emond clients directory: {:?}", err);
            return false;
        }
    };

    if clients.is_empty() {
        return false;
    }
    true
}

#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {
    use super::parse_emond_rules;
    use crate::{
        artifacts::os::macos::emond::eventmonitor::{
            check_clients, parse_actions, parse_emond_data, parse_event_types,
        },
        filesystem::directory::is_directory,
    };
    use plist::{Dictionary, Value};
    use std::path::PathBuf;

    #[test]
    fn test_system_parse_emond_rules() {
        let default_path = "/etc/emond.d/rules";
        if !is_directory(default_path) {
            return;
        }
        let _ = parse_emond_rules(default_path).unwrap();
    }

    #[test]
    fn test_parse_emond_rules() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/emond");

        let results = parse_emond_rules(&test_location.display().to_string()).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].enabled, true);
        assert_eq!(results[0].name, "poisonapple rule");
        assert_eq!(results[0].event_types, ["startup"]);
        assert_eq!(results[0].allow_partial_criterion_match, false);
        assert_eq!(results[0].criterion.is_empty(), true);
        assert_eq!(results[0].log_actions.is_empty(), true);
        assert_eq!(results[0].send_notification_actions.is_empty(), true);
        assert_eq!(results[0].send_email_actions.is_empty(), true);
        assert_eq!(results[0].variables.is_empty(), true);

        assert_eq!(results[0].command_actions.len(), 1);
        assert_eq!(results[0].command_actions[0].command, "/Users/sur/Library/Python/3.8/lib/python/site-packages/poisonapple/auxiliary/poisonapple.sh");
        assert_eq!(results[0].command_actions[0].group, String::new());
        assert_eq!(results[0].command_actions[0].user, "root");
        assert_eq!(results[0].command_actions[0].arguments, ["Emond"]);

        assert_eq!(results[1].enabled, false);
        assert_eq!(results[1].name, "sample rule");
        assert_eq!(results[1].event_types, ["startup"]);
        assert_eq!(results[1].allow_partial_criterion_match, false);
        assert_eq!(results[1].criterion.len(), 1);

        let mut test_dictionary = Dictionary::new();
        test_dictionary.insert(
            String::from("operator"),
            Value::String(String::from("True")),
        );

        assert_eq!(results[1].criterion[0], test_dictionary);

        assert_eq!(results[1].send_notification_actions.is_empty(), true);
        assert_eq!(results[1].send_email_actions.is_empty(), true);
        assert_eq!(results[1].variables.is_empty(), true);
        assert_eq!(results[1].command_actions.is_empty(), true);

        assert_eq!(results[1].log_actions.len(), 1);

        assert_eq!(
            results[1].log_actions[0].message,
            "Event Monitor started at ${builtin:now}"
        );
        assert_eq!(results[1].log_actions[0].facility, String::new());
        assert_eq!(results[1].log_actions[0].log_level, "Notice");
        assert_eq!(results[1].log_actions[0].log_type, "syslog");
        assert_eq!(results[1].log_actions[0].parameters, Dictionary::new());
    }

    #[test]
    fn test_parse_emond_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/emond/test123.plist");

        let results = parse_emond_data(&test_location.display().to_string()).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].enabled, true);
        assert_eq!(results[0].name, "poisonapple rule");
        assert_eq!(results[0].event_types, ["startup"]);
        assert_eq!(results[0].allow_partial_criterion_match, false);
        assert_eq!(results[0].criterion.is_empty(), true);
        assert_eq!(results[0].log_actions.is_empty(), true);
        assert_eq!(results[0].send_notification_actions.is_empty(), true);
        assert_eq!(results[0].send_email_actions.is_empty(), true);
        assert_eq!(results[0].variables.is_empty(), true);

        assert_eq!(results[0].command_actions.len(), 1);
        assert_eq!(results[0].command_actions[0].command, "/Users/sur/Library/Python/3.8/lib/python/site-packages/poisonapple/auxiliary/poisonapple.sh");
        assert_eq!(results[0].command_actions[0].group, String::new());
        assert_eq!(results[0].command_actions[0].user, "root");
        assert_eq!(results[0].command_actions[0].arguments, ["Emond"]);
    }

    #[test]
    fn test_parse_event_types() {
        let test: Value = Value::Array(vec![
            Value::String(String::from("startup")),
            Value::String(String::from("auth:login")),
        ]);

        let results = parse_event_types(&test).unwrap();
        assert_eq!(results[0], "startup");
        assert_eq!(results[1], "auth:login");
    }

    #[test]
    fn test_check_clients() {
        let results = check_clients();
        assert_eq!(results, false);
    }

    #[test]
    fn test_parse_actions() {
        let mut test_dictionary = Dictionary::new();
        test_dictionary.insert(String::from("message"), Value::String(String::from("test")));
        test_dictionary.insert(
            String::from("command"),
            Value::String(String::from("nc -l")),
        );
        test_dictionary.insert(String::from("user"), Value::String(String::from("root")));
        test_dictionary.insert(String::from("arguments"), Value::Array(Vec::new()));
        test_dictionary.insert(String::from("group"), Value::String(String::from("wheel")));
        test_dictionary.insert(
            String::from("type"),
            Value::String(String::from("RunCommand")),
        );

        let test_value: Value = Value::Array(vec![Value::Dictionary(test_dictionary)]);

        let results = parse_actions(&test_value).unwrap();
        assert_eq!(results.command_actions[0].user, "root");
        assert_eq!(results.command_actions[0].group, "wheel");
        assert_eq!(results.command_actions[0].command, "nc -l");
        assert_eq!(results.command_actions[0].arguments.len(), 0);
    }
}
