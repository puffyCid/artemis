use log::error;
use quick_xml::{events::Event, Reader};

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

    loop {
        match reader.read_event() {
            Err(err) => {
                error!("[tasks] Could not read RegistrationInfo xml data: {err:?}");
                break;
            }
            Ok(Event::Eof) => break,
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                b"URI" => {
                    info.uri = Some(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                b"SecurityDescriptor" => {
                    info.sid = Some(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                b"Source" => {
                    info.source = Some(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                b"Date" => {
                    info.date = Some(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                b"Author" => {
                    info.author = Some(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                b"Version" => {
                    info.version =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                b"Description" => {
                    info.description =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                b"Documentation" => {
                    info.documentation =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                _ => break,
            },
            Ok(Event::End(tag)) => match tag.name().as_ref() {
                b"RegistrationInfo" => break,
                _ => continue,
            },
            _ => (),
        }
    }

    info
}

#[cfg(test)]
mod tests {
    use super::parse_registration;
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
}
