use super::{
    error::TaskError,
    schemas::{actions::parse_actions, registration::parse_registration, triggers::parse_trigger},
};
use crate::utils::encoding::read_xml;
use crate::{
    artifacts::os::windows::tasks::schemas::{
        principals::parse_principals, settings::parse_settings,
    },
    utils::encoding::base64_encode_standard,
};
use common::windows::{Actions, TaskXml};
use log::error;
use quick_xml::{events::Event, Reader};

/// Parse Schedule Task XML files. Windows Vista and higher use XML for Tasks
pub(crate) fn parse_xml(path: &str) -> Result<TaskXml, TaskError> {
    // Read XML file at provided path. Tasks use UTF16 encoding
    let xml_result = read_xml(path);
    let xml_data = match xml_result {
        Ok(result) => result,
        Err(err) => {
            error!("[tasks] Could not read Task XML file at {path}: {err:?}");
            return Err(TaskError::ReadXml);
        }
    };
    process_xml(&xml_data, path)
}

/// Parse the different parts the XML schema format
fn process_xml(xml: &str, path: &str) -> Result<TaskXml, TaskError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut task_xml = TaskXml {
        registration_info: None,
        triggers: None,
        settings: None,
        data: None,
        principals: None,
        actions: Actions {
            exec: Vec::new(),
            com_handler: Vec::new(),
            send_email: Vec::new(),
            show_message: Vec::new(),
        },
        path: path.to_string(),
    };

    // Track Principals
    let mut principals = Vec::new();

    loop {
        match reader.read_event() {
            Err(err) => {
                error!("[tasks] Could not read xml data: {err:?}");
                break;
            }
            Ok(Event::Eof) => break,
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                b"RegistrationInfo" => {
                    let reg_info = parse_registration(&mut reader);
                    task_xml.registration_info = Some(reg_info);
                }
                b"Triggers" => {
                    let trig_info = parse_trigger(&mut reader);
                    task_xml.triggers = Some(trig_info);
                }
                b"Settings" => {
                    let set_info = parse_settings(&mut reader);
                    task_xml.settings = Some(set_info);
                }
                b"Principal" => {
                    let prin_info = parse_principals(&mut reader);
                    principals.push(prin_info);
                }
                b"Actions" => {
                    let action_info = parse_actions(&mut reader);
                    task_xml.actions = action_info;
                }
                b"Data" => {
                    task_xml.data = Some(base64_encode_standard(
                        reader.read_text(tag.name()).unwrap_or_default().as_bytes(),
                    ));
                }
                _ => continue,
            },
            _ => continue,
        }
    }
    task_xml.principals = Some(principals);

    Ok(task_xml)
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::tasks::xml::{parse_xml, process_xml},
        utils::encoding::read_xml,
    };
    use std::path::PathBuf;

    #[test]
    fn test_parse_xml() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/tasks/win10/VSIX Auto Update");

        let result = parse_xml(&test_location.display().to_string()).unwrap();

        assert_ne!(result.principals, None);
        assert_eq!(result.actions.exec.len(), 1);
        assert_eq!(result.path, test_location.display().to_string())
    }

    #[test]
    fn test_process_xml() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/tasks/win10/VSIX Auto Update");

        let xml = read_xml(&test_location.display().to_string()).unwrap();
        let result = process_xml(&xml, &test_location.display().to_string()).unwrap();

        assert_ne!(result.principals, None);
        assert_eq!(result.actions.exec.len(), 1);
        assert_eq!(result.path, test_location.display().to_string())
    }
}
