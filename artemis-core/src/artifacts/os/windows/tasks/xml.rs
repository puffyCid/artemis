use super::schemas::{
    actions::Actions, principals::Principals, registration::RegistrationInfo, settings::Settings,
    triggers::Triggers,
};
use super::{
    error::TaskError,
    schemas::{actions::parse_actions, registration::parse_registration, triggers::parse_trigger},
};
use crate::{
    artifacts::os::windows::tasks::schemas::{
        principals::parse_principals, settings::parse_settings,
    },
    filesystem::files::read_file,
    utils::{
        encoding::base64_encode_standard,
        nom_helper::{nom_unsigned_two_bytes, Endian},
        strings::extract_utf16_string,
    },
};
use log::error;
use quick_xml::{events::Event, Reader};
use serde::Serialize;

/**
 * Structure of a XML format Schedule Task
 * Schema at: [Task XML](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-tsch/0d6383e4-de92-43e7-b0bb-a60cfa36379f)
 */
#[derive(Debug, Serialize)]
pub(crate) struct TaskXml {
    registration_info: Option<RegistrationInfo>,
    triggers: Option<Triggers>,
    settings: Option<Settings>,
    /**Arbitrary data, we base64 encode the data */
    data: Option<String>,
    principals: Option<Vec<Principals>>,
    actions: Actions,
    path: String,
}
impl TaskXml {
    /// Parse Schedule Task XML files. Windows Vista and higher use XML for Tasks
    pub(crate) fn parse_xml(path: &str) -> Result<TaskXml, TaskError> {
        let xml_data = TaskXml::read_xml(path)?;
        TaskXml::process_xml(&xml_data, path)
    }

    /// Read a XML file into a string and check for UTF16 Byte Order Mark (BOM)
    pub(crate) fn read_xml(path: &str) -> Result<String, TaskError> {
        let bytes_result = read_file(path);
        let bytes = match bytes_result {
            Ok(result) => result,
            Err(err) => {
                error!("[tasks] Could not read Task XML file at {path}: {err:?}");
                return Err(TaskError::ReadXml);
            }
        };

        let utf_check = nom_unsigned_two_bytes(&bytes, Endian::Be);
        let (data, utf_status) = match utf_check {
            Ok(result) => result,
            Err(_err) => {
                error!("[tasks] Could not read XML to determine UTF16 {path}");
                return Err(TaskError::ReadXml);
            }
        };

        let utf16_le = 0xfffe;
        let utf16_be = 0xfeff;

        let xml_string = if utf_status == utf16_be || utf_status == utf16_le {
            extract_utf16_string(data)
        } else {
            extract_utf16_string(&bytes)
        };

        Ok(xml_string)
    }

    /// Parse the different parts the XML schema format
    fn process_xml(xml: &str, path: &str) -> Result<TaskXml, TaskError> {
        let mut reader = Reader::from_str(xml);
        reader.trim_text(true);
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
                        task_xml.principals = Some(principals.clone());
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

        Ok(task_xml)
    }
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::tasks::xml::TaskXml;
    use std::path::PathBuf;

    #[test]
    fn test_parse_xml() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/tasks/win10/VSIX Auto Update");

        let result = TaskXml::parse_xml(&test_location.display().to_string()).unwrap();

        assert_ne!(result.principals, None);
        assert_eq!(result.actions.exec.len(), 1);
        assert_eq!(result.path, test_location.display().to_string())
    }

    #[test]
    fn test_read_xml() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/tasks/win10/VSIX Auto Update");

        let result = TaskXml::read_xml(&test_location.display().to_string()).unwrap();
        assert!(result.starts_with("<?xml version=\"1.0\" encoding=\"UTF-16\"?>"));
        assert!(result.contains("<URI>\\Microsoft\\VisualStudio\\VSIX Auto Update</URI>"));
        assert_eq!(result.len(), 1356);
    }

    #[test]
    fn test_process_xml() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/tasks/win10/VSIX Auto Update");

        let xml = TaskXml::read_xml(&test_location.display().to_string()).unwrap();
        let result = TaskXml::process_xml(&xml, &test_location.display().to_string()).unwrap();

        assert_ne!(result.principals, None);
        assert_eq!(result.actions.exec.len(), 1);
        assert_eq!(result.path, test_location.display().to_string())
    }
}
