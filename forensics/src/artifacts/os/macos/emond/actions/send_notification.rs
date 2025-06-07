use crate::artifacts::os::macos::plist::property_list::{get_dictionary, get_string};
use common::macos::SendNotification;
use log::warn;
use plist::Dictionary;

/// Parse send notification `Emond` action
pub(crate) fn parse_action_send_notification(action_dictionary: &Dictionary) -> SendNotification {
    let mut notification = SendNotification {
        message: String::new(),
        name: String::new(),
        details: Dictionary::new(),
    };

    for (key, action_value) in action_dictionary {
        if key == "message" {
            notification.message = get_string(action_value).unwrap_or_default();
        } else if key == "name" {
            notification.name = get_string(action_value).unwrap_or_default();
        } else if key == "details" {
            notification.details = get_dictionary(action_value).unwrap_or_default();
        } else if key != "type" {
            warn!("[emond] Unknown Log Action key: {key}. Value: {action_value:?}");
        }
    }
    notification
}
