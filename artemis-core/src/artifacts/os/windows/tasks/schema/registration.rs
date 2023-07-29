use log::{error, warn};
use quick_xml::{
    events::{BytesText, Event},
    Reader,
};

#[derive(Debug)]
pub(crate) struct RegistrationInfo {
    uri: Option<String>,
    sid: Option<String>,
    source: Option<String>,
    date: Option<String>,
    author: Option<String>,
    version: Option<String>,
    description: Option<String>,
    documentation: Option<String>,
}

enum RegType {
    Uri,
    Sid,
    Source,
    Date,
    Author,
    Version,
    Description,
    Documentation,
    Unknown,
}

/// Parse RegistrationInfo of Task
pub(crate) fn parse_registration(reader: &mut Reader<&[u8]>) -> RegistrationInfo {
    let mut info = RegistrationInfo {
        uri: None,
        sid: None,
        source: None,
        date: None,
        author: None,
        version: None,
        description: None,
        documentation: None,
    };

    let mut reg_type = RegType::Unknown;
    loop {
        match reader.read_event() {
            Err(err) => {
                error!("[tasks] Could not read RegistrationInfo xml data: {err:?}");
                break;
            }
            Ok(Event::Eof) => break,
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                b"URI" => reg_type = RegType::Uri,
                b"SecurityDescriptor" => reg_type = RegType::Sid,
                b"Source" => reg_type = RegType::Source,
                b"Date" => reg_type = RegType::Date,
                b"Author" => reg_type = RegType::Author,
                b"Version" => reg_type = RegType::Version,
                b"Description" => reg_type = RegType::Description,
                b"Documentation" => reg_type = RegType::Documentation,
                _ => break,
            },
            Ok(Event::Text(tag)) => process_registration(&mut info, &tag, &reg_type),
            Ok(Event::End(tag)) => match tag.name().as_ref() {
                b"RegistrationInfo" => break,
                _ => continue,
            },
            _ => (),
        }
    }

    info
}

/// Process each RegistrationType
fn process_registration(info: &mut RegistrationInfo, data: &BytesText<'_>, reg_type: &RegType) {
    match reg_type {
        RegType::Uri => info.uri = Some(data.unescape().unwrap_or_default().to_string()),
        RegType::Date => info.date = Some(data.unescape().unwrap_or_default().to_string()),
        RegType::Author => info.author = Some(data.unescape().unwrap_or_default().to_string()),
        RegType::Description => {
            info.description = Some(data.unescape().unwrap_or_default().to_string())
        }
        RegType::Sid => info.sid = Some(data.unescape().unwrap_or_default().to_string()),
        RegType::Source => info.source = Some(data.unescape().unwrap_or_default().to_string()),
        RegType::Version => info.version = Some(data.unescape().unwrap_or_default().to_string()),
        RegType::Documentation => {
            info.documentation = Some(data.unescape().unwrap_or_default().to_string())
        }
        RegType::Unknown => (),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_registration, process_registration, RegType, RegistrationInfo};
    use crate::artifacts::os::windows::tasks::task::TaskData;
    use quick_xml::{events::Event, Reader};
    use std::path::PathBuf;

    #[test]
    fn test_parse_registration() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/tasks/win10/VSIX Auto Update");

        let xml = TaskData::read_xml(&test_location.display().to_string()).unwrap();
        let mut reader = Reader::from_str(&xml);
        reader.trim_text(true);

        loop {
            match reader.read_event() {
                Err(err) => {
                    break;
                }
                Ok(Event::Eof) => break,
                Ok(Event::Start(tag)) => match tag.name().as_ref() {
                    b"RegistrationInfo" => {
                        let reg_info = parse_registration(&mut reader);
                        assert_eq!(
                            reg_info.uri,
                            Some(String::from("\\Microsoft\\VisualStudio\\VSIX Auto Update"))
                        );
                        assert_eq!(
                            reg_info.author,
                            Some(String::from("Microsoft VisualStudio"))
                        );
                    }
                    _ => (),
                },
                _ => (),
            }
        }
    }

    #[test]
    fn test_process_registration() {
        let xml = r#"
                <URI>Test</URI>
                <tag2>Test 2</tag2>
             "#;

        let mut reader = Reader::from_str(xml);
        reader.trim_text(true);

        let mut info = RegistrationInfo {
            uri: None,
            sid: None,
            source: None,
            date: None,
            author: None,
            version: None,
            description: None,
            documentation: None,
        };
        let mut reg_type = RegType::Unknown;

        loop {
            match reader.read_event() {
                Err(_err) => {
                    break;
                }
                Ok(Event::Eof) => break,
                Ok(Event::Start(tag)) => match tag.name().as_ref() {
                    b"URI" => reg_type = RegType::Uri,
                    b"SecurityDescriptor" => reg_type = RegType::Sid,
                    b"Source" => reg_type = RegType::Source,
                    b"Date" => reg_type = RegType::Date,
                    b"Author" => reg_type = RegType::Author,
                    b"Version" => reg_type = RegType::Version,
                    b"Description" => reg_type = RegType::Description,
                    b"Documentation" => reg_type = RegType::Documentation,
                    _ => break,
                },
                Ok(Event::Text(tag)) => process_registration(&mut info, &tag, &reg_type),
                Ok(Event::End(tag)) => match tag.name().as_ref() {
                    b"RegistrationInfo" => break,
                    _ => continue,
                },
                _ => (),
            }
        }
        assert_eq!(info.uri, Some(String::from("Test")));
    }
}
