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
 * 3. Add support for precision formatters
 * 4. More tests
 * 4.5 Add caching!?
 * 5. Remove unwraps
 * 6. Return a struct instead of string? Struct has:
 *   - message
 *   - event id
 *   - provider
 *   - guid?
 */

/// Combine eventlog strings with the template strings
pub(crate) fn add_message_strings(
    log: &EventLogRecord,
    resources: &StringResource,
    /*Cache provider_name and event_id as key. Make another struct for very fast lookups? */
    cache: &mut HashMap<String, StringResource>,
    param_regex: &Regex,
) -> Option<String> {
    let event_id = get_event_id(&log.data)?;
    let version = get_version(&log.data)?;
    let provider_name = get_provider(&log.data)?;
    let guid = get_guid(&log.data)?.to_lowercase();

    let provider_opt = resources.providers.get(provider_name);
    let provider = match provider_opt {
        Some(result) => result,
        None => return merge_strings_no_manifest(&log.data),
    };

    let message_files: &[String] = provider.message_file.as_ref();
    let parameter_files: &[String] = provider.parameter_file.as_ref();

    let mut param_message_table = HashMap::new();
    // If we parameter files. Then we extract the message from it. There should only be one?
    for file in parameter_files {
        let template = resources.templates.get(file);
        if template.is_none() {
            continue;
        }

        param_message_table = template.unwrap().message_table.as_ref()?.clone();
    }

    // If we do not have a parameter message file. Fallback to the default parameter file(s), just incase
    if param_message_table.is_empty() {
        for (key, value) in &resources.templates {
            // Are there more? How unique are the IDs?
            // https://github.com/libyal/libevtx/blob/main/documentation/Windows%20XML%20Event%20Log%20(EVTX).asciidoc#parameter-expansion mentions two
            // But both have same ID: %%1833. %%1833 is used in 4624 logon events
            // MsObjs associated with security and kernel32 associated with system records?
            if (provider.registry_path.to_lowercase().contains("security")
                && !key.to_lowercase().contains("msobjs"))
                || (provider.registry_path.to_lowercase().contains("system")
                    && !key.to_lowercase().contains("kernel32"))
            {
                continue;
            }
            if value.message_table.is_none() {
                continue;
            }

            let table = value.message_table.as_ref()?;

            param_message_table.extend(table.clone());
        }
    }

    for message in message_files {
        let template = resources.templates.get(message);
        if template.is_none() {
            continue;
        }

        let message_table = template.unwrap().message_table.as_ref();
        let manifest = template.unwrap().wevt_template.as_ref();

        // We need both
        if message_table.is_none() || manifest.is_none() {
            continue;
        }

        let table = message_table.unwrap().get(&(event_id as u32));
        let manifest_template = manifest.unwrap().get(&guid);

        // We need both
        if table.is_none() || manifest_template.is_none() {
            continue;
        }

        let definition = manifest_template
            .unwrap()
            .definitions
            .get(&format!("{event_id}_{version}"));

        if definition.is_none() {
            continue;
        }

        // We have everything needed to make an attempt to merge strings!
        // But no guarantee of 100% perfect merge!
        return merge_strings(
            &log.data,
            table.unwrap(),
            definition.unwrap(),
            param_regex,
            &param_message_table,
        );
    }

    // If we failed to find anything. Try to make message anyway
    return merge_strings_no_manifest(&log.data);
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

/// Attempt to merge template eventlog strings with the eventlog data  
/// We *try* to follow the approach defined at [libyal docs](https://github.com/libyal/libevtx/blob/main/documentation/Windows%20XML%20Event%20Log%20(EVTX).asciidoc#parsing-event-data)
fn merge_strings(
    log: &Value,
    table: &MessageTable,
    manifest: &Definition,
    param_regex: &Regex,
    parameter_message: &HashMap<u32, MessageTable>,
) -> Option<String> {
    // Manifest data type should? map to EventData, UserData, etc
    let data = log.as_object()?["Event"].as_object()?[&manifest.template.as_ref()?.event_data_type]
        .as_object()?;

    let mut clean_message = clean_table(&table.message);

    for found in param_regex.find_iter(&clean_message.clone()) {
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
            if value.as_str().is_some_and(|s| s.starts_with("%%")) {
                let num_result = value.as_str()?.get(2..)?.parse();
                if num_result.is_err() {
                    warn!(
                        "[eventlogs] Could not get parameter message id for log message: {:?}",
                        num_result.unwrap_err()
                    );
                    continue;
                }

                let param_message_id: u32 = num_result.unwrap_or_default();
                if parameter_message.is_empty() {
                    warn!("[eventlogs] Got parameter message id {value:?} but no parameter message table");
                    continue;
                }

                let final_param = parameter_message.get(&param_message_id);
                let param_message_value = match final_param {
                    Some(message) => message,
                    None => {
                        warn!("[eventlogs] Could not find parameter message for {value:?} in message table");
                        continue;
                    }
                };

                clean_message = clean_message.replacen(param, &param_message_value.message, 1);
                continue;
            }
            clean_message = clean_message.replacen(
                param,
                &serde_json::from_value(value.clone()).unwrap_or(value.to_string()),
                1,
            );
        }
    }

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
        test_location.push("tests/test_data/windows/eventlogs/samples/complex_log.json");

        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let log: EventLogRecord = serde_json::from_slice(&data).unwrap();
        let resources = get_resources().unwrap();

        let mut cache = HashMap::new();
        let params = create_regex(r"%\d+").unwrap();
        let message = add_message_strings(&log, &resources, &mut cache, &params).unwrap();

        assert_eq!(
            message,
            "Credential Manager credentials were read.\n\nSubject:\n\tSecurity ID:\t\tS-1-5-21-549467458-3727351111-1684278619-1001\n\tAccount Name:\t\tbob\n\tAccount Domain:\t\tDESKTOP-9FSUKAJ\n\tLogon ID:\t\t0x3311b1\n\tRead Operation:\t\tEnumerate Credentials\r\n\n\nThis event occurs when a user performs a read operation on stored credentials in Credential Manager.\r\n"
        );
    }

    #[test]
    fn test_add_message_strings_log() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/eventlogs/samples/eventlog.json");

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
        test_location.push("tests/test_data/windows/eventlogs/samples/logon_log.json");

        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let log: EventLogRecord = serde_json::from_slice(&data).unwrap();
        let resources = get_resources().unwrap();

        let mut cache = HashMap::new();
        let params = create_regex(r"%\d+").unwrap();
        let message = add_message_strings(&log, &resources, &mut cache, &params).unwrap();

        assert!(message.contains("An account was successfully logged on"));
        assert!(!message.contains("%%"));

        // Depending on Windows version the eventlog message will be different sizes. Size below is for Windows 11 (4624 version 3)
        if message.len() == 2212 {
            assert_eq!(
                message,
                "An account was successfully logged on.\n\nSubject:\n\tSecurity ID:\t\tS-1-5-18\n\tAccount Name:\t\tDESKTOP-9FSUKAJ$\n\tAccount Domain:\t\tWORKGROUP\n\tLogon ID:\t\t0x3e7\n\nLogon Information:\n\tLogon Type:\t\t5\n\tRestricted Admin Mode:\t-\n\tRemote Credential Guard:\t-\n\tVirtual Account:\t\tNo\r\n\n\tElevated Token:\t\tYes\r\n\n\nImpersonation Level:\t\tImpersonation\r\n\n\nNew Logon:\n\tSecurity ID:\t\tS-1-5-18\n\tAccount Name:\t\tSYSTEM\n\tAccount Domain:\t\tNT AUTHORITY\n\tLogon ID:\t\t0x3e7\n\tLinked Logon ID:\t\t0x0\n\tNetwork Account Name:\t-\n\tNetwork Account Domain:\t-\n\tLogon GUID:\t\t00000000-0000-0000-0000-000000000000\n\nProcess Information:\n\tProcess ID:\t\t0x444\n\tProcess Name:\t\tC:\\Windows\\System32\\services.exe\n\nNetwork Information:\n\tWorkstation Name:\t-\n\tSource Network Address:\t-\n\tSource Port:\t\t-\n\nDetailed Authentication Information:\n\tLogon Process:\t\tAdvapi  \n\tAuthentication Package:\tNegotiate\n\tTransited Services:\t-\n\tPackage Name (NTLM only):\t-\n\tKey Length:\t\t0\n\nThis event is generated when a logon session is created. It is generated on the computer that was accessed.\n\nThe subject fields indicate the account on the local system which requested the logon. This is most commonly a service such as the Server service, or a local process such as Winlogon.exe or Services.exe.\n\nThe logon type field indicates the kind of logon that occurred. The most common types are 2 (interactive) and 3 (network).\n\nThe New Logon fields indicate the account for whom the new logon was created, i.e. the account that was logged on.\n\nThe network fields indicate where a remote logon request originated. Workstation name is not always available and may be left blank in some cases.\n\nThe impersonation level field indicates the extent to which a process in the logon session can impersonate.\n\nThe authentication information fields provide detailed information about this specific logon request.\n\t- Logon GUID is a unique identifier that can be used to correlate this event with a KDC event.\n\t- Transited services indicate which intermediate services have participated in this logon request.\n\t- Package name indicates which sub-protocol was used among the NTLM protocols.\n\t- Key length indicates the length of the generated session key. This will be 0 if no session key was requested.\r\n"
            );
        }
    }

    #[test]
    fn test_add_message_strings_parameters() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/eventlogs/samples/parameter_log.json");

        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let log: EventLogRecord = serde_json::from_slice(&data).unwrap();
        let resources = get_resources().unwrap();

        let mut cache = HashMap::new();
        let params = create_regex(r"%\d+").unwrap();
        let message = add_message_strings(&log, &resources, &mut cache, &params).unwrap();

        assert!(!message.contains("%%"));

        assert_eq!(
                message,
                "Boot Configuration Data loaded.\n\nSubject:\n\tSecurity ID:\t\tS-1-5-18\n\tAccount Name:\t\t-\n\tAccount Domain:\t\t-\n\tLogon ID:\t\t0x3e7\n\nGeneral Settings:\n\tLoad Options:\t\t-\n\tAdvanced Options:\t\tNo\r\n\n\tConfiguration Access Policy:\tDefault\r\n\n\tSystem Event Logging:\tNo\r\n\n\tKernel Debugging:\tNo\r\n\n\tVSM Launch Type:\tOff\r\n\n\nSignature Settings:\n\tTest Signing:\t\tNo\r\n\n\tFlight Signing:\t\tNo\r\n\n\tDisable Integrity Checks:\tNo\r\n\n\nHyperVisor Settings:\n\tHyperVisor Load Options:\t-\n\tHyperVisor Launch Type:\tOff\r\n\n\tHyperVisor Debugging:\tNo\r\n\r\n"
            );
    }
}
