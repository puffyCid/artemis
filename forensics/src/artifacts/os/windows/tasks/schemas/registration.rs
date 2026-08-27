use crate::artifacts::os::windows::tasks::text::read_text_unescaped;
use common::windows::RegistrationInfo;
use quick_xml::{Reader, events::Event};
use tracing::error;

/// Parse `RegistrationInfo` of Task
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

    loop {
        match reader.read_event() {
            Err(err) => {
                error!("Could not read RegistrationInfo xml data: {err:?}");
                break;
            }
            Ok(Event::Eof) => break,
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                "URI" => {
                    info.uri = Some(read_text_unescaped(reader, tag.name()));
                }
                "SecurityDescriptor" => {
                    info.sid = Some(read_text_unescaped(reader, tag.name()));
                }
                "Source" => {
                    info.source = Some(read_text_unescaped(reader, tag.name()));
                }
                "Date" => {
                    info.date = Some(read_text_unescaped(reader, tag.name()));
                }
                "Author" => {
                    info.author = Some(read_text_unescaped(reader, tag.name()));
                }
                "Version" => {
                    info.version = Some(read_text_unescaped(reader, tag.name()));
                }
                "Description" => {
                    info.description = Some(read_text_unescaped(reader, tag.name()));
                }
                "Documentation" => {
                    info.documentation = Some(read_text_unescaped(reader, tag.name()));
                }
                _ => break,
            },
            Ok(Event::End(tag)) if tag.name().as_ref() == "RegistrationInfo" => {
                break;
            }
            _ => (),
        }
    }

    info
}

#[cfg(test)]
mod tests {
    use super::parse_registration;
    use crate::utils::encoding::read_xml;
    use quick_xml::{Reader, events::Event};
    use std::path::PathBuf;

    #[test]
    fn test_parse_registration() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/tasks/win10/VSIX Auto Update");

        let xml = read_xml(&test_location.display().to_string()).unwrap();
        let mut reader = Reader::from_str(&xml);
        reader.config_mut().trim_text(true);

        loop {
            match reader.read_event() {
                Err(_) => {
                    break;
                }
                Ok(Event::Eof) => break,
                Ok(Event::Start(tag)) => match tag.name().as_ref() {
                    "RegistrationInfo" => {
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
    fn test_parse_registration_win11() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/tasks/win11/SoftLandingCreativeManagementTask");

        let xml = read_xml(&test_location.display().to_string()).unwrap();
        let mut reader = Reader::from_str(&xml);
        reader.config_mut().trim_text(true);

        loop {
            match reader.read_event() {
                Err(_) => {
                    break;
                }
                Ok(Event::Eof) => break,
                Ok(Event::Start(tag)) => match tag.name().as_ref() {
                    "RegistrationInfo" => {
                        let reg_info = parse_registration(&mut reader);
                        assert_eq!(
                            reg_info.uri,
                            Some(String::from(
                                "\\SoftLanding\\S-1-5-21-476446702-302789185-3387769606-1001\\SoftLandingCreativeManagementTask"
                            ))
                        );
                        assert_eq!(
                            reg_info.sid,
                            Some(String::from(
                                "D:P(A;;FA;;;SY)(A;CI;0x80010000;;;WD)(A;;FA;;;S-1-5-21-476446702-302789185-3387769606-1001)"
                            ))
                        );
                    }
                    _ => (),
                },
                _ => (),
            }
        }
    }
}
