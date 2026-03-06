use crate::artifacts::os::windows::tasks::text::read_text_unescaped;
use common::windows::RegistrationInfo;
use log::error;
use quick_xml::{Reader, events::Event};

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
                error!("[tasks] Could not read RegistrationInfo xml data: {err:?}");
                break;
            }
            Ok(Event::Eof) => break,
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                b"URI" => {
                    info.uri = Some(read_text_unescaped(reader, tag.name()));
                }
                b"SecurityDescriptor" => {
                    info.sid = Some(read_text_unescaped(reader, tag.name()));
                }
                b"Source" => {
                    info.source = Some(read_text_unescaped(reader, tag.name()));
                }
                b"Date" => {
                    info.date = Some(read_text_unescaped(reader, tag.name()));
                }
                b"Author" => {
                    info.author = Some(read_text_unescaped(reader, tag.name()));
                }
                b"Version" => {
                    info.version = Some(read_text_unescaped(reader, tag.name()));
                }
                b"Description" => {
                    info.description = Some(read_text_unescaped(reader, tag.name()));
                }
                b"Documentation" => {
                    info.documentation = Some(read_text_unescaped(reader, tag.name()));
                }
                _ => break,
            },
            Ok(Event::End(tag)) => {
                if tag.name().as_ref() == b"RegistrationInfo" {
                    break;
                }
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
}
