use super::{
    resources::{
        manifest::{defintion::Definition, maps::MapInfo},
        message::MessageTable,
    },
    strings::StringResource,
};
use common::windows::EventLogRecord;
use log::{error, warn};
use regex::Regex;
use serde_json::{Map, Value};
use std::collections::HashMap;

/**TODO
 * 4. More tests - insane_log.json next
 * 4.5 Add caching!?
 * 5. Remove unwraps
 * 6. Return a struct instead of string? Struct has:
 *   - message
 *   - event id
 *   - provider
 *   - guid?
 *   - include source evtx file
 *   - message_table entry? (like macos unified logs)
 *   - event source name
 *   - provider info?
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
    let qualifier_check = 16;
    // https://github.com/libyal/libevtx/blob/main/documentation/Windows%20XML%20Event%20Log%20(EVTX).asciidoc#message-string-identifier
    let real_event_id = (event_id.qualifier << qualifier_check) | event_id.id;

    let version = get_version(&log.data)?;
    let provider_name = get_provider(&log.data)?;
    let guid_opt = get_guid(&log.data);

    let guid = match guid_opt {
        Some(result) => result.to_lowercase().replace('{', "").replace('}', ""),
        None => String::new(),
    };

    let source = match get_event_source_name(&log.data) {
        Some(result) => result,
        None => "",
    };

    let provider_opt = resources.providers.get(&format!("{guid}"));
    let provider = match provider_opt {
        Some(result) => result,
        None => {
            // try provider name before we give up
            match resources.providers.get(&provider_name.to_lowercase()) {
                Some(result) => result,
                None => return merge_strings_no_manifest(&log.data),
            }
        }
    };

    // If we have ProcessingErrorData, there is nothing we can do to assemble the log message.
    // We just combine everything and return that
    if let Some(_) = log
        .data
        .as_object()?
        .get("Event")?
        .as_object()?
        .get("ProcessingErrorData")
    {
        return merge_strings_no_manifest(&log.data);
    }

    let message_files: &[String] = provider.message_file.as_ref();
    let parameter_files: &[String] = provider.parameter_file.as_ref();

    let mut param_message_table = HashMap::new();
    // If we have parameter files. Then we extract the message from it. There should only be one?
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

        let message_table = template?.message_table.as_ref();
        let manifest = template?.wevt_template.as_ref();

        // We need at least one
        if message_table.is_none() && manifest.is_none() {
            continue;
        }

        if guid.is_empty() {
            let table = match message_table?.get(&(real_event_id as u32)) {
                Some(result) => result,
                None => continue,
            };

            return merge_strings_message_table(
                &log.data,
                table,
                param_regex,
                &param_message_table,
            );
        }

        let manifest_op = manifest?.get(&guid);
        let manifist_template = match manifest_op {
            Some(result) => result,
            None => continue,
        };

        let event_definition = match manifist_template
            .definitions
            .get(&format!("{}_{version}", event_id.id))
        {
            Some(result) => result,
            None => continue,
        };

        let table_opt = message_table?.get(&event_definition.message_id);
        let table = match table_opt {
            Some(result) => result,
            None => continue,
        };

        // If we do not have any templates. Can just try messagetable only
        if event_definition.template.is_none() {
            return merge_strings_message_table(
                &log.data,
                table,
                param_regex,
                &param_message_table,
            );
        }

        // We have everything needed to make an attempt to merge strings!
        // But no guarantee of 100% perfect merge!
        return merge_strings(
            &log.data,
            table,
            message_table?,
            event_definition,
            &manifist_template.maps,
            param_regex,
            &param_message_table,
        );
    }

    // If we failed to find anything. Try to make message anyway
    merge_strings_no_manifest(&log.data)
}

#[derive(Debug)]
struct EventId {
    id: u64,
    qualifier: u64,
}

/// Get Event ID and Qualifier if available
fn get_event_id(data: &Value) -> Option<EventId> {
    let id = data
        .as_object()?
        .get("Event")?
        .as_object()?
        .get("System")?
        .as_object()?
        .get("EventID")?;

    let mut event_id = EventId {
        id: 0,
        qualifier: 0,
    };

    if id.is_u64() {
        event_id.id = id.as_u64()?;
        return Some(event_id);
    }

    for (attr_key, attr_value) in id.as_object()? {
        if attr_key == "#attributes" {
            for (key, value) in attr_value.as_object()? {
                if key == "Qualifiers" {
                    event_id.qualifier = value.as_u64()?;
                }
            }
        } else if attr_key == "#text" {
            event_id.id = attr_value.as_u64()?;
        }
    }

    Some(event_id)
}

/// Get log entry version
fn get_version(data: &Value) -> Option<u64> {
    let version = &data
        .as_object()?
        .get("Event")?
        .as_object()?
        .get("System")?
        .as_object()?
        .get("Version")?;
    if version.is_u64() {
        return version.as_u64();
    }

    panic!(
        "[eventlogs] Version number is not a number: {:?}",
        &data
            .as_object()?
            .get("Event")?
            .as_object()?
            .get("System")?
            .as_object()?
            .get("Version")?
    );
    None
}

/// Get log provider
fn get_provider(data: &Value) -> Option<&str> {
    let provider = &data
        .as_object()?
        .get("Event")?
        .as_object()?
        .get("System")?
        .as_object()?
        .get("Provider")?
        .as_object()?["#attributes"]
        .as_object()?["Name"];
    if provider.is_string() {
        return provider.as_str();
    }

    panic!(
        "[eventlogs] Provider is not a string: {:?}",
        &data
            .as_object()?
            .get("Event")?
            .as_object()?
            .get("System")?
            .as_object()?
            .get("Provider")?
            .as_object()?["#attributes"]
            .as_object()?["Name"]
    );
    None
}

/// Get GUID for log if available
fn get_guid(data: &Value) -> Option<&str> {
    let guid = data
        .as_object()?
        .get("Event")?
        .as_object()?
        .get("System")?
        .as_object()?
        .get("Provider")?
        .as_object()?
        .get("#attributes")?
        .as_object()?
        .get("Guid")?;

    if guid.is_string() {
        return guid.as_str();
    }

    panic!(
        "[eventlogs] Guid is not a string: {:?}",
        &data
            .as_object()?
            .get("Event")?
            .as_object()?
            .get("System")?
            .as_object()?
            .get("Provider")?
            .as_object()?["#attributes"]
            .as_object()?["Guid"]
    );
    None
}

fn get_event_source_name(data: &Value) -> Option<&str> {
    let name = data
        .as_object()?
        .get("Event")?
        .as_object()?
        .get("System")?
        .as_object()?
        .get("Provider")?
        .as_object()?
        .get("#attributes")?
        .as_object()?
        .get("EventSourceName")?;

    if name.is_string() {
        return name.as_str();
    }

    None
}

/// Attempt to merge template eventlog strings with the eventlog data  
/// We *try* to follow the approach defined at [libyal docs](https://github.com/libyal/libevtx/blob/main/documentation/Windows%20XML%20Event%20Log%20(EVTX).asciidoc#parsing-event-data)
fn merge_strings(
    log: &Value,
    table: &MessageTable,
    other_messages: &HashMap<u32, MessageTable>,
    manifest: &Definition,
    maps: &[MapInfo],
    param_regex: &Regex,
    parameter_message: &HashMap<u32, MessageTable>,
) -> Option<String> {
    let mut data = log.as_object()?.get("Event")?;
    let mut clean_message = clean_table(&table.message);
    println!("{clean_message:?}");

    let mut event_data = &Map::new();
    // Loop through keys until we get to our data
    while data.is_object() {
        for (key, value) in data.as_object()? {
            if key != &manifest.template.as_ref()?.event_data_type {
                data = value;
                continue;
            }

            if value.is_null() {
                return Some(clean_message);
            }

            event_data = value.as_object()?;
            data = &Value::Null;
            break;
        }
    }

    let element_list = &manifest.template.as_ref()?.elements;
    let mut element_names = Vec::new();
    for element in element_list {
        element_names.push(element.element_name.as_str());
    }

    for found in param_regex.find_iter(&clean_message.clone()) {
        let param = found.as_str();
        if !param.starts_with('%') {
            continue;
        }
        let num_result = param.get(1..)?.parse();
        if num_result.is_err() {
            error!(
                "[eventlogs] Could not get parameter for log message: {:?}",
                num_result.unwrap_err()
            );
            continue;
        }

        let param_num = num_result.unwrap_or(0);

        if param_num == 0 {
            warn!("[eventlogs] Got zero or lower as parameter value. This is wrong");
            continue;
        }

        // Parameter ID starts at 0
        let adjust_id = 1;
        let element_attributes = &element_list.get(param_num - adjust_id)?.attribute_list;

        // If we do not have an attribute list. Then we have to use the element_names
        // Seen for UserData entries: https://github.com/libyal/libevtx/blob/main/documentation/Windows%20XML%20Event%20Log%20(EVTX).asciidoc#event-data
        if element_attributes.is_empty() {
            let name = *element_names.get(param_num - adjust_id)?;
            let value = event_data.get(name)?;

            if value.is_number() {
                let mut new_value = Value::Null;
                // The EventLog data may not actually be a real number. Could be an enum that maps to the messagetable
                // Check out maps array/hashmap and messagetable
                for map in maps {
                    let message_id = match map.data.get(&(value.as_u64()? as u32)) {
                        Some(result) => result,
                        None => continue,
                    };

                    let string_data = other_messages.get(&(message_id.message_id as u32))?;
                    new_value = serde_json::to_value(string_data.message.strip_suffix("\r\n"))
                        .unwrap_or(Value::Null);

                    clean_message =
                        add_event_string(&new_value, clean_message, param, parameter_message)?;

                    break;
                }

                if !new_value.is_null() {
                    continue;
                }
            }
            clean_message = add_event_string(value, clean_message, param, parameter_message)?;
            continue;
        }

        for attribute in element_attributes {
            let value = event_data.get(&attribute.value)?;
            clean_message = add_event_string(value, clean_message, param, parameter_message)?;
        }
    }

    Some(clean_message)
}

/// Add the eventlog data to our message
fn add_event_string(
    value: &Value,
    mut message: String,
    param: &str,
    parameter_message: &HashMap<u32, MessageTable>,
) -> Option<String> {
    if value.as_str().is_some_and(|s| s.starts_with("%%")) {
        let num_result = value.as_str()?.get(2..)?.parse();
        if num_result.is_err() {
            warn!(
                "[eventlogs] Could not get parameter message id for log message: {:?}",
                num_result.unwrap_err()
            );
            return None;
        }

        let param_message_id: u32 = num_result.unwrap_or_default();
        if parameter_message.is_empty() {
            warn!("[eventlogs] Got parameter message id {value:?} but no parameter message table");
            return None;
        }

        let final_param = parameter_message.get(&param_message_id);
        let param_message_value = match final_param {
            Some(message) => message,
            None => {
                warn!(
                    "[eventlogs] Could not find parameter message for {value:?} in message table"
                );
                return None;
            }
        };

        message = message.replacen(param, &param_message_value.message, 1);
        return Some(message);
    }

    message = message.replacen(
        param,
        &serde_json::from_value(value.clone()).unwrap_or(value.to_string()),
        1,
    );

    Some(message)
}

/// Combine eventlog data if we have neither `MESSAGETABLE` or `WEVT_TEMPLATE`. Sometimes neither will exist
fn merge_strings_no_manifest(log: &Value) -> Option<String> {
    let data = log.as_object()?.get("Event")?.as_object()?;
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

/// Combine eventlog data if we only have `MESSAGETABLE`
fn merge_strings_message_table(
    log: &Value,
    table: &MessageTable,
    param_regex: &Regex,
    parameter_message: &HashMap<u32, MessageTable>,
) -> Option<String> {
    let mut clean_message = clean_table(&table.message);
    let data = log.as_object()?.get("Event")?.as_object()?;

    let mut values = Vec::new();
    let event_defaults = vec![
        "EventData",
        "UserData",
        "ProcessingErrorData",
        "BinaryEventData",
        "DebugData",
    ];
    for (key, value) in data {
        // Key should? be one of the following: EventData, UserData, DebugData, ProcessingErrorData, BinaryEventData
        if !key.ends_with("Data") {
            continue;
        }
        if value.is_null() && event_defaults.contains(&key.as_str()) {
            return Some(clean_message);
        }

        for (key_data, value_data) in value.as_object()? {
            if value_data.is_null() {
                continue;
            }

            for (text_key, text_value) in value_data.as_object()? {
                if !text_value.is_array() {
                    continue;
                }
                values = text_value.as_array()?.clone();
            }
        }
    }

    for found in param_regex.find_iter(&clean_message.clone()) {
        let param = found.as_str();

        if !param.starts_with('%') {
            continue;
        }
        let num_result = param.get(1..)?.parse();
        if num_result.is_err() {
            error!(
                "[eventlogs] Could not get parameter for log message: {:?}",
                num_result.unwrap_err()
            );
            continue;
        }

        let param_num = num_result.unwrap_or(0);

        if param_num == 0 {
            warn!("[eventlogs] Got zero or lower as parameter value. This is wrong");
            continue;
        }

        let adjust_id = 1;
        let value = values.get(param_num - adjust_id)?;
        clean_message = add_event_string(value, clean_message, param, parameter_message)?;
    }

    Some(clean_message)
}

/// Iterate through all eventlog data keys and get values
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

/// Windows uses % for formatting. Clean these up
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
    use super::{add_message_strings, build_string, get_event_id};
    use crate::{
        artifacts::os::windows::eventlogs::{
            combine::{clean_table, get_guid, get_provider, get_version},
            strings::get_resources,
        },
        filesystem::files::read_file,
        utils::regex_options::create_regex,
    };
    use common::windows::EventLogRecord;
    use serde_json::json;
    use std::{collections::HashMap, path::PathBuf};

    #[test]
    fn test_add_message_strings() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/eventlogs/samples/");
        let samples = [
            "complex_log.json",
            "eventlog.json",
            "logon_log.json",
            "parameter_log.json",
            "storage_log.json",
            "processingerror_log.json",
            "qualifiers_log.json",
            "userdata_log.json",
            "null_log.json",
            "qualifier_non_zero_log.json",
            "formater_log.json",
        ];

        let resources = get_resources().unwrap();
        let mut cache = HashMap::new();
        let params = create_regex(r"%\d+").unwrap();

        for sample in samples {
            test_location.push(sample);
            let data = read_file(test_location.to_str().unwrap()).unwrap();
            let log: EventLogRecord = serde_json::from_slice(&data).unwrap();

            test_location.pop();

            let message = add_message_strings(&log, &resources, &mut cache, &params).unwrap();

            assert!(!message.contains("%%"));

            match sample {
                "processingerror_log.json" => {
                    // Windows Event Viewer shows "PSMFlags for Desktop AppX process %1 with applicationID %2 is %3." But i think that is what a successful log entry is suppose to be
                    assert_eq!(message,"ErrorCode: 15005\nDataItemName: PsmFlags\nEventPayload: 4D006900630072006F0073006F00660074002E004D006900630072006F0073006F006600740045006400670065002E0053007400610062006C0065005F003100320037002E0030002E0032003600350031002E00370034005F006E00650075007400720061006C005F005F003800770065006B0079006200330064003800620062007700650000004D006900630072006F0073006F00660074002E004D006900630072006F0073006F006600740045006400670065002E0053007400610062006C0065005F003800770065006B007900620033006400380062006200770065002100410070007000000001010110\n");
                }
                "complex_log.json" => {
                    assert_eq!(
                        message,
                        "Credential Manager credentials were read.\n\nSubject:\n\tSecurity ID:\t\tS-1-5-21-549467458-3727351111-1684278619-1001\n\tAccount Name:\t\tbob\n\tAccount Domain:\t\tDESKTOP-9FSUKAJ\n\tLogon ID:\t\t0x3311b1\n\tRead Operation:\t\tEnumerate Credentials\r\n\n\nThis event occurs when a user performs a read operation on stored credentials in Credential Manager.\r\n"
                    );
                }
                "eventlog.json" => {
                    assert_eq!(
                        message,
                        "Setting MSA Client Id for Token requests: {f0c62012-2cef-4831-b1f7-930682874c86}\nError: -2147467259\nFunction: WinStoreAuth::AuthenticationInternal::SetMsaClientId\nSource: onecoreuap\\enduser\\winstore\\auth\\lib\\winstoreauth.cpp (265)\r\n"
                    );
                }
                "logon_log.json" => {
                    assert!(message.contains("An account was successfully logged on"));

                    // Depending on Windows version the eventlog message will be different sizes. Size below is for Windows 11 (4624 version 3)
                    if message.len() == 2212 {
                        assert_eq!(message,"An account was successfully logged on.\n\nSubject:\n\tSecurity ID:\t\tS-1-5-18\n\tAccount Name:\t\tDESKTOP-9FSUKAJ$\n\tAccount Domain:\t\tWORKGROUP\n\tLogon ID:\t\t0x3e7\n\nLogon Information:\n\tLogon Type:\t\t5\n\tRestricted Admin Mode:\t-\n\tRemote Credential Guard:\t-\n\tVirtual Account:\t\tNo\r\n\n\tElevated Token:\t\tYes\r\n\n\nImpersonation Level:\t\tImpersonation\r\n\n\nNew Logon:\n\tSecurity ID:\t\tS-1-5-18\n\tAccount Name:\t\tSYSTEM\n\tAccount Domain:\t\tNT AUTHORITY\n\tLogon ID:\t\t0x3e7\n\tLinked Logon ID:\t\t0x0\n\tNetwork Account Name:\t-\n\tNetwork Account Domain:\t-\n\tLogon GUID:\t\t00000000-0000-0000-0000-000000000000\n\nProcess Information:\n\tProcess ID:\t\t0x444\n\tProcess Name:\t\tC:\\Windows\\System32\\services.exe\n\nNetwork Information:\n\tWorkstation Name:\t-\n\tSource Network Address:\t-\n\tSource Port:\t\t-\n\nDetailed Authentication Information:\n\tLogon Process:\t\tAdvapi  \n\tAuthentication Package:\tNegotiate\n\tTransited Services:\t-\n\tPackage Name (NTLM only):\t-\n\tKey Length:\t\t0\n\nThis event is generated when a logon session is created. It is generated on the computer that was accessed.\n\nThe subject fields indicate the account on the local system which requested the logon. This is most commonly a service such as the Server service, or a local process such as Winlogon.exe or Services.exe.\n\nThe logon type field indicates the kind of logon that occurred. The most common types are 2 (interactive) and 3 (network).\n\nThe New Logon fields indicate the account for whom the new logon was created, i.e. the account that was logged on.\n\nThe network fields indicate where a remote logon request originated. Workstation name is not always available and may be left blank in some cases.\n\nThe impersonation level field indicates the extent to which a process in the logon session can impersonate.\n\nThe authentication information fields provide detailed information about this specific logon request.\n\t- Logon GUID is a unique identifier that can be used to correlate this event with a KDC event.\n\t- Transited services indicate which intermediate services have participated in this logon request.\n\t- Package name indicates which sub-protocol was used among the NTLM protocols.\n\t- Key length indicates the length of the generated session key. This will be 0 if no session key was requested.\r\n");
                    }
                }
                "parameter_log.json" => {
                    assert_eq!(
                            message,
                            "Boot Configuration Data loaded.\n\nSubject:\n\tSecurity ID:\t\tS-1-5-18\n\tAccount Name:\t\t-\n\tAccount Domain:\t\t-\n\tLogon ID:\t\t0x3e7\n\nGeneral Settings:\n\tLoad Options:\t\t-\n\tAdvanced Options:\t\tNo\r\n\n\tConfiguration Access Policy:\tDefault\r\n\n\tSystem Event Logging:\tNo\r\n\n\tKernel Debugging:\tNo\r\n\n\tVSM Launch Type:\tOff\r\n\n\nSignature Settings:\n\tTest Signing:\t\tNo\r\n\n\tFlight Signing:\t\tNo\r\n\n\tDisable Integrity Checks:\tNo\r\n\n\nHyperVisor Settings:\n\tHyperVisor Load Options:\t-\n\tHyperVisor Launch Type:\tOff\r\n\n\tHyperVisor Debugging:\tNo\r\n\r\n"
                    );
                }
                "storage_log.json" => {
                    assert_eq!(
                            message,
                            "Error summary for Storport Device (Port = 0, Path = 2, Target = 0, Lun = 0) whose Corresponding Class Disk Device Guid is 00000000-0000-0000-0000-000000000000:\r\n                    \nThere were 730 total errors seen and 0 timeouts.\r\n                    \nThe last error seen had opcode 0 and completed with SrbStatus 4 and ScsiStatus 2.\r\n                    \nThe sense code was (2,58,0).\r\n                    \nThe latency was 0 ms.\r\n"
                    );
                }
                "qualifiers_log.json" => {
                    assert_eq!(
                            message,
                            "Provider \"Registry\" is Started. \n\nDetails: \n\tProviderName=Registry\r\n\tNewProviderState=Started\r\n\r\n\tSequenceNumber=1\r\n\r\n\tHostName=Chocolatey_PSHost\r\n\tHostVersion=5.1.22621.1\r\n\tHostId=719491d7-e472-4d47-8057-9a2f29ae1c91\r\n\tHostApplication=C:\\ProgramData\\chocolatey\\choco.exe install 7zip\r\n\tEngineVersion=\r\n\tRunspaceId=\r\n\tPipelineId=\r\n\tCommandName=\r\n\tCommandType=\r\n\tScriptName=\r\n\tCommandPath=\r\n\tCommandLine=\r\n"
                    );
                }
                "userdata_log.json" => {
                    assert_eq!(
                        message,
                        "Package KB5044033 was successfully changed to the Installed state.\r\n"
                    );
                }
                "null_log.json" => {
                    assert_eq!(
                        message,
                        "Windows Management Instrumentation Service started sucessfully\r\n"
                    );
                }
                "qualifier_non_zero_log.json" => {
                    assert!(
                        message.starts_with("The Software Protection service has completed licensing status check.\nApplication Id=55c92734-d682-4d71-983e-d6ec3f16059f")
                    );
                    assert_eq!(message.len(), 6291);
                }
                "formater_log.json" => {
                    assert_eq!(
                        message,
                        "Windows Management Instrumentation Service started sucessfully\r\n"
                    );
                }
                _ => panic!("should not have an unknown sample?"),
            }
        }
    }

    #[test]
    fn test_clean_table() {
        let test = "%1 hello Rust!%n";
        assert_eq!(clean_table(test), "%1 hello Rust!\n");
    }

    #[test]
    fn test_build_string() {
        let test = "hello ";
        let data = json!({"test": "value"});
        let result = build_string(test, &data).unwrap();
        assert_eq!(result, "hello test: value\n");
    }

    #[test]
    fn test_get_event_id() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/eventlogs/samples/userdata_log.json");
        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let log: EventLogRecord = serde_json::from_slice(&data).unwrap();

        let result = get_event_id(&log.data).unwrap();
        assert_eq!(result.id, 2);
        assert_eq!(result.qualifier, 0);
    }

    #[test]
    fn test_get_version() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/eventlogs/samples/userdata_log.json");
        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let log: EventLogRecord = serde_json::from_slice(&data).unwrap();

        let result = get_version(&log.data).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_get_provider() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/eventlogs/samples/userdata_log.json");
        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let log: EventLogRecord = serde_json::from_slice(&data).unwrap();

        let result = get_provider(&log.data).unwrap();
        assert_eq!(result, "Microsoft-Windows-Servicing");
    }

    #[test]
    fn test_get_guid() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/eventlogs/samples/userdata_log.json");
        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let log: EventLogRecord = serde_json::from_slice(&data).unwrap();

        let result = get_guid(&log.data).unwrap();
        assert_eq!(result, "BD12F3B8-FC40-4A61-A307-B7A013A069C1");
    }
}
