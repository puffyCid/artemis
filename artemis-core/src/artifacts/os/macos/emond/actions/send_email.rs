use crate::artifacts::os::macos::plist::property_list::get_string;
use common::macos::SendEmail;
use log::warn;
use plist::Dictionary;

/// Parse Send mail `Emond` action
pub(crate) fn parse_action_send_email(action_dictionary: &Dictionary) -> SendEmail {
    let mut email_data = SendEmail {
        message: String::new(),
        subject: String::new(),
        localization_bundle_path: String::new(),
        relay_host: String::new(),
        admin_email: String::new(),
        recipient_addresses: Vec::new(),
    };

    for (key, action_value) in action_dictionary {
        if key == "message" {
            email_data.message = get_string(action_value).unwrap_or_default();
        } else if key == "subject" {
            email_data.subject = get_string(action_value).unwrap_or_default();
        } else if key == "localization_bundle_path" {
            email_data.localization_bundle_path = get_string(action_value).unwrap_or_default();
        } else if key == "relay_host" {
            email_data.relay_host = get_string(action_value).unwrap_or_default();
        } else if key == "admin_email" {
            email_data.admin_email = get_string(action_value).unwrap_or_default();
        } else if key == "recipient_addresses" {
            let arg_array = if let Some(results) = action_value.as_array() {
                results
            } else {
                warn!("[emond] Failed to parse Send Email Action array: {action_value:?}",);
                continue;
            };

            for args in arg_array {
                email_data
                    .recipient_addresses
                    .push(get_string(args).unwrap_or_default());
            }
        } else if key == "type" {
            // Skip type values. We already know the action type
            continue;
        } else {
            warn!("[emond] Unknown Log Action key: {key}. Value: {action_value:?}");
        }
    }
    email_data
}
