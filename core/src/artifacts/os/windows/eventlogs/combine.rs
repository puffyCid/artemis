use super::{
    resources::{manifest::defintion::Definition, message::MessageTable},
    strings::StringResource,
};
use common::windows::EventLogRecord;
use log::{error, warn};
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;

/**TODO
 * 1. Reformat the StringResource struct. Need to make sure ParameterFile is a separate structure
 * 2. Add parameter evaulation support (%%1800)
 * 3. Add support for precision formatters
 * 4. More tests
 */

/// Combine eventlog strings with the template strings
pub(crate) fn add_message_strings(
    log: &EventLogRecord,
    resources: &HashMap<String, StringResource>,
    cache: &mut HashMap<String, StringResource>,
    parameters: &Regex,
) -> Option<String> {
    let event_id = get_event_id(&log.data)?;
    let version = get_version(&log.data)?;
    let provider = get_provider(&log.data)?;
    let guid = get_guid(&log.data)?;

    let manifest = get_manifest(resources, provider);

    // There may not be a manifest associated with this eventlog record. We will try to make a message anyway
    // This is best effort
    if manifest.is_none() {
        return merge_strings_no_manifest(&log.data);
    }

    let table = manifest
        .unwrap()
        .message_table
        .as_ref()?
        .get(&(event_id as u32))?;

    // Get the template based on the Provider GUID. Then get the template defitnition based on EventID and Version number
    // Ex: Event ID 4624 version 3
    let template = manifest
        .unwrap()
        .wevt_template
        .as_ref()?
        .get(&guid.to_lowercase())?
        .definitions
        .get(&format!("{}_{}", event_id, version))?;

    // We have everthing needed to make an attempt to merge strings!
    // But no guarantee of 100% perfect merge!

    merge_strings(&log.data, table, template, parameters)
}

fn get_event_id(data: &Value) -> Option<u64> {
    let id = &data.as_object()?["Event"].as_object()?["System"].as_object()?["EventID"];
    if id.is_u64() {
        return id.as_u64();
    }

    panic!("handle qualifiers?");

    None
}

fn get_version(data: &Value) -> Option<u64> {
    let version = &data.as_object()?["Event"].as_object()?["System"].as_object()?["Version"];
    if version.is_u64() {
        return version.as_u64();
    }

    panic!(
        "[eventlogs] Version number is not a number: {:?}",
        &data.as_object()?["Event"].as_object()?["System"].as_object()?["Version"]
    );
    None
}

fn get_provider(data: &Value) -> Option<&str> {
    let provider = &data.as_object()?["Event"].as_object()?["System"].as_object()?["Provider"]
        .as_object()?["#attributes"]
        .as_object()?["Name"];
    if provider.is_string() {
        return provider.as_str();
    }

    panic!(
        "[eventlogs] Provider is not a string: {:?}",
        &data.as_object()?["Event"].as_object()?["System"].as_object()?["Provider"].as_object()?
            ["#attributes"]
            .as_object()?["Name"]
    );
    None
}

fn get_guid(data: &Value) -> Option<&str> {
    let guid = &data.as_object()?["Event"].as_object()?["System"].as_object()?["Provider"]
        .as_object()?["#attributes"]
        .as_object()?["Guid"];
    if guid.is_string() {
        return guid.as_str();
    }

    panic!(
        "[eventlogs] Guid is not a string: {:?}",
        &data.as_object()?["Event"].as_object()?["System"].as_object()?["Provider"].as_object()?
            ["#attributes"]
            .as_object()?["Guid"]
    );
    None
}

fn get_manifest<'a>(
    resources: &'a HashMap<String, StringResource>,
    provider: &str,
) -> Option<&'a StringResource> {
    for value in resources.values() {
        for reg in &value.registry_info {
            if !reg.registry_key.ends_with(provider) {
                continue;
            }

            return Some(value);
        }
    }

    None
}

/// Attempt to merge template eventlog strings with the eventlog data  
/// We *try* to follow the approach defined at [libyal docs](https://github.com/libyal/libevtx/blob/main/documentation/Windows%20XML%20Event%20Log%20(EVTX).asciidoc#parsing-event-data)
fn merge_strings(
    log: &Value,
    table: &MessageTable,
    manifest: &Definition,
    parameters: &Regex,
) -> Option<String> {
    // Manifest data type should? map to EventData, UserData, etc
    let data = log.as_object()?["Event"].as_object()?[&manifest.template.as_ref()?.event_data_type]
        .as_object()?;

    let mut clean_message = clean_table(&table.message);

    for found in parameters.find_iter(&clean_message.clone()) {
        let param = found.as_str();
        let mut param_num = 0;
        // Should always be true
        if param.starts_with('%') {
            let num_result = param.get(1..)?.parse();
            if num_result.is_err() {
                error!(
                    "[eventlogs] Could not get parameter for log message: {:?}",
                    num_result.unwrap_err()
                );
                continue;
            }

            param_num = num_result.unwrap_or_default();
        }
        if param_num <= 0 {
            warn!("[eventlogs] Got zero or lower as parameter value. This is wrong");
            continue;
        }
        // Parameter ID starts at 0
        let adjust_id = 1;
        let element_attributes = &manifest
            .template
            .as_ref()?
            .elements
            .get(param_num - adjust_id)?
            .attribute_list;

        for attribute in element_attributes {
            let value = data.get(&attribute.value)?;
            if value.to_string().contains("%%") {
                println!("Add parameter lookup support!");
                continue;
            }
            clean_message = clean_message.replacen(
                param,
                &serde_json::from_value(value.clone()).unwrap_or(value.to_string()),
                1,
            );
        }
    }

    println!("{clean_message}");

    Some(clean_message)
}

fn merge_strings_no_manifest(log: &Value) -> Option<String> {
    let data = log.as_object()?["Event"].as_object()?;
    let mut clean_string = String::new();
    for (key, value) in data {
        // Key should? be one of the following: EventData, UserData, DebugData, ProcessingErrorData, BinaryEventData
        if !key.ends_with("Data") {
            continue;
        }

        clean_string = build_string(&clean_string, value)?;
    }
    Some(clean_string)
}

fn build_string(message: &str, data: &Value) -> Option<String> {
    let mut clean_message = message.to_string();
    for (key, value) in data.as_object()? {
        if key == "xmlns" {
            continue;
        }
        if !value.is_object() {
            clean_message = format!(
                "{clean_message}{key}: {}\n",
                serde_json::from_value(value.clone()).unwrap_or(value.to_string())
            );
            continue;
        }

        clean_message = format!("{clean_message}{key}:\n ");
        clean_message = build_string(&clean_message, value)?;
    }

    Some(clean_message)
}

// Windows uses % for formatting. Clean these up
fn clean_table(message: &str) -> String {
    let mut clean = message.replace("%t", "\t");
    clean = clean.replace("%r%n", "\n");
    clean = clean.replace("%n", "\n");
    clean = clean.replace("%r", "\n");
    clean = clean.replace("%_", " ");
    clean = clean.replace("%%", "%");
    clean = clean.replace("%!", "!");
    clean = clean.replace("%.", ".");
    clean = clean.replace("%b", " ");

    clean
}

#[cfg(test)]
mod tests {
    use super::add_message_strings;
    use crate::{
        artifacts::os::windows::eventlogs::strings::get_resources, filesystem::files::read_file,
        utils::regex_options::create_regex,
    };
    use common::windows::EventLogRecord;
    use std::{collections::HashMap, path::PathBuf};

    #[test]
    fn test_add_message_strings_complex() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/eventlogs/complex_log.json");

        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let log: EventLogRecord = serde_json::from_slice(&data).unwrap();
        let resources = get_resources().unwrap();

        let mut cache = HashMap::new();
        let params = create_regex(r"%\d+").unwrap();
        let message = add_message_strings(&log, &resources, &mut cache, &params).unwrap();

        assert_eq!(
            message,
            "Credential Manager credentials were read.\n\nSubject:\n\tSecurity ID:\t\tS-1-5-21-549467458-3727351111-1684278619-1001\n\tAccount Name:\t\tbob\n\tAccount Domain:\t\tDESKTOP-9FSUKAJ\n\tLogon ID:\t\t0x3311b1\n\tRead Operation:\t\t%8\n\nThis event occurs when a user performs a read operation on stored credentials in Credential Manager.\r\n"
        );
    }

    #[test]
    fn test_add_message_strings_log() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/eventlogs/eventlog.json");

        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let log: EventLogRecord = serde_json::from_slice(&data).unwrap();
        let resources = get_resources().unwrap();

        let mut cache = HashMap::new();
        let params = create_regex(r"%\d+").unwrap();
        let message = add_message_strings(&log, &resources, &mut cache, &params).unwrap();

        assert_eq!(
            message,
            "Message: Setting MSA Client Id for Token requests: {f0c62012-2cef-4831-b1f7-930682874c86}\nFunction: WinStoreAuth::AuthenticationInternal::SetMsaClientId\nError Code: -2147467259\nSource: onecoreuap\\enduser\\winstore\\auth\\lib\\winstoreauth.cpp\nLine Number: 265\nCorrelationVector: NULL\nProductId: NULL\n"
        );
    }

    #[test]
    fn test_add_message_strings_logon() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/eventlogs/logon_log.json");

        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let log: EventLogRecord = serde_json::from_slice(&data).unwrap();
        let resources = get_resources().unwrap();

        let mut cache = HashMap::new();
        let params = create_regex(r"%\d+").unwrap();
        let message = add_message_strings(&log, &resources, &mut cache, &params).unwrap();

        assert_eq!(
            message,
            "An account was successfully logged on.\n\nSubject:\n\tSecurity ID:\t\tS-1-5-18\n\tAccount Name:\t\tDESKTOP-9FSUKAJ$\n\tAccount Domain:\t\tWORKGROUP\n\tLogon ID:\t\t0x3e7\n\nLogon Information:\n\tLogon Type:\t\t5\n\tRestricted Admin Mode:\t-\n\tRemote Credential Guard:\t-\n\tVirtual Account:\t\t%26\n\tElevated Token:\t\t%28\n\nImpersonation Level:\t\t%21\n\nNew Logon:\n\tSecurity ID:\t\tS-1-5-18\n\tAccount Name:\t\tSYSTEM\n\tAccount Domain:\t\tNT AUTHORITY\n\tLogon ID:\t\t0x3e7\n\tLinked Logon ID:\t\t0x0\n\tNetwork Account Name:\t-\n\tNetwork Account Domain:\t-\n\tLogon GUID:\t\t00000000-0000-0000-0000-000000000000\n\nProcess Information:\n\tProcess ID:\t\t0x444\n\tProcess Name:\t\tC:\\Windows\\System32\\services.exe\n\nNetwork Information:\n\tWorkstation Name:\t-\n\tSource Network Address:\t-\n\tSource Port:\t\t-\n\nDetailed Authentication Information:\n\tLogon Process:\t\tAdvapi  \n\tAuthentication Package:\tNegotiate\n\tTransited Services:\t-\n\tPackage Name (NTLM only):\t-\n\tKey Length:\t\t0\n\nThis event is generated when a logon session is created. It is generated on the computer that was accessed.\n\nThe subject fields indicate the account on the local system which requested the logon. This is most commonly a service such as the Server service, or a local process such as Winlogon.exe or Services.exe.\n\nThe logon type field indicates the kind of logon that occurred. The most common types are 2 (interactive) and 3 (network).\n\nThe New Logon fields indicate the account for whom the new logon was created, i.e. the account that was logged on.\n\nThe network fields indicate where a remote logon request originated. Workstation name is not always available and may be left blank in some cases.\n\nThe impersonation level field indicates the extent to which a process in the logon session can impersonate.\n\nThe authentication information fields provide detailed information about this specific logon request.\n\t- Logon GUID is a unique identifier that can be used to correlate this event with a KDC event.\n\t- Transited services indicate which intermediate services have participated in this logon request.\n\t- Package name indicates which sub-protocol was used among the NTLM protocols.\n\t- Key length indicates the length of the generated session key. This will be 0 if no session key was requested.\r\n"
        );
    }
}
