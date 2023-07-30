use super::{
    error::TaskError,
    schema::{registration::parse_registration, triggers::parse_trigger},
    task::TaskData,
};
use crate::{
    artifacts::os::windows::tasks::schema::{
        principals::parse_principals, settings::parse_settings,
    },
    filesystem::files::read_file,
    utils::{
        nom_helper::{nom_unsigned_two_bytes, Endian},
        strings::extract_utf16_string,
    },
};
use log::error;
use quick_xml::{events::Event, Reader};

impl TaskData {
    pub(crate) fn parse_xml(path: &str) {}

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

    fn process_xml(xml: &str) -> Result<(), TaskError> {
        let mut reader = Reader::from_str(xml);
        reader.trim_text(true);

        loop {
            match reader.read_event() {
                Err(err) => {
                    panic!("[tasks] Could not read xml data: {err:?}");
                    error!("[tasks] Could not read xml data: {err:?}");
                    break;
                }
                Ok(Event::Eof) => break,
                Ok(Event::Start(tag)) => match tag.name().as_ref() {
                    b"RegistrationInfo" => {
                        let reg_info = parse_registration(&mut reader);
                        println!("{reg_info:?}");
                    }
                    b"Triggers" => {
                        let trig_info = parse_trigger(&mut reader);
                        println!("{trig_info:?}");
                    }
                    b"Settings" => {
                        let set_info = parse_settings(&mut reader);
                        println!("{set_info:?}");
                    }
                    b"Principals" => {
                        let prin_info = parse_principals(&mut reader);
                        println!("{prin_info:?}");
                    }
                    _ => (),
                },
                _ => (),
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::tasks::task::TaskData;
    use std::path::PathBuf;

    #[test]
    fn test_read_xml() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/tasks/win10/VSIX Auto Update");

        let result = TaskData::read_xml(&test_location.display().to_string()).unwrap();
        assert!(result.starts_with("<?xml version=\"1.0\" encoding=\"UTF-16\"?>"));
        assert!(result.contains("<URI>\\Microsoft\\VisualStudio\\VSIX Auto Update</URI>"));
        assert_eq!(result.len(), 1356);
    }

    #[test]
    fn test_process_xml() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/tasks/win10/VSIX Auto Update");

        let xml = TaskData::read_xml(&test_location.display().to_string()).unwrap();
        let result = TaskData::process_xml(&xml).unwrap();
    }
}
