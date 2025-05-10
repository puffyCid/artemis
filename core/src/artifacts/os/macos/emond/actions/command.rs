use crate::artifacts::os::macos::plist::property_list::get_string;
use common::macos::Command;
use log::warn;
use plist::Dictionary;

// Parse the Run Command Action Emond Rule
pub(crate) fn parse_action_run_command(action_dictionary: &Dictionary) -> Command {
    let mut command_data = Command {
        command: String::new(),
        user: String::new(),
        group: String::new(),
        arguments: Vec::new(),
    };
    for (key, action_value) in action_dictionary {
        if key == "command" {
            command_data.command = get_string(action_value).unwrap_or_default();
        } else if key == "user" {
            command_data.user = get_string(action_value).unwrap_or_default();
        } else if key == "group" {
            command_data.group = get_string(action_value).unwrap_or_default();
        } else if key == "arguments" {
            let arg_array = if let Some(results) = action_value.as_array() {
                results
            } else {
                warn!("[emond] Failed to parse Run Command Action array: {action_value:?}",);
                continue;
            };

            for args in arg_array {
                command_data
                    .arguments
                    .push(get_string(args).unwrap_or_default());
            }
        }
    }
    command_data
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::macos::emond::actions::command::parse_action_run_command;
    use plist::{Dictionary, Value};

    #[test]
    fn test_parse_action_run_command() {
        let mut test_dictionary = Dictionary::new();
        test_dictionary.insert(String::from("message"), Value::String(String::from("test")));
        test_dictionary.insert(
            String::from("command"),
            Value::String(String::from("nc -l")),
        );
        test_dictionary.insert(String::from("user"), Value::String(String::from("root")));
        test_dictionary.insert(String::from("arguments"), Value::Array(Vec::new()));
        test_dictionary.insert(String::from("group"), Value::String(String::from("wheel")));

        let results = parse_action_run_command(&test_dictionary);
        assert_eq!(results.user, "root");
        assert_eq!(results.group, "wheel");
        assert_eq!(results.command, "nc -l");
        assert_eq!(results.arguments.len(), 0);
    }
}
