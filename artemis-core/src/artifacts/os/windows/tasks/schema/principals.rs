use log::error;
use quick_xml::{events::Event, Reader};

#[derive(Debug)]
pub(crate) struct Principals {
    user_id: Option<String>,
    logon_type: Option<String>,
    group_id: Option<String>,
    display_nme: Option<String>,
    run_level: Option<String>,
    process_token_sid_type: Option<String>,
    required_privileges: Option<Vec<String>>,
    id_attribute: Option<String>,
}

/// Parse Principal information for Task
pub(crate) fn parse_principals(reader: &mut Reader<&[u8]>) -> Principals {
    let mut info = Principals {
        user_id: None,
        logon_type: None,
        group_id: None,
        display_nme: None,
        run_level: None,
        process_token_sid_type: None,
        required_privileges: None,
        id_attribute: None,
    };

    let mut privs = Vec::new();
    loop {
        match reader.read_event() {
            Err(err) => {
                error!("[tasks] Could not read Settings xml data: {err:?}");
                break;
            }
            Ok(Event::Eof) => break,
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                b"UserId" => {
                    info.user_id =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                b"LogonType" => {
                    info.logon_type =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                b"GroupId" => {
                    info.group_id =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                b"DisplayName" => {
                    info.display_nme =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                b"RunLevel" => {
                    info.run_level =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                b"ProcessTokenSidType" => {
                    info.process_token_sid_type =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                b"Privilege" => {
                    privs.push(reader.read_text(tag.name()).unwrap_or_default().to_string());
                }
                b"id" => {
                    info.id_attribute =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string())
                }
                _ => continue,
            },
            Ok(Event::End(tag)) => match tag.name().as_ref() {
                b"Principals" => break,
                _ => continue,
            },
            _ => (),
        }
    }

    if !privs.is_empty() {
        info.required_privileges = Some(privs);
    }

    info
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::tasks::schema::principals::parse_principals;
    use quick_xml::Reader;

    #[test]
    fn test_parse_principals() {
        let xml = r#"
        <UserId>S-1-5-18</UserId>
        <ProcessTokenSidType>S-1-5-122228</ProcessTokenSidType>
        <Privilege>SuperAdmin</Privilege>
             "#;

        let mut reader = Reader::from_str(xml);
        reader.trim_text(true);
        let result = parse_principals(&mut reader);
        assert_eq!(result.user_id.unwrap(), "S-1-5-18");
        assert_eq!(result.required_privileges.unwrap()[0], "SuperAdmin");
    }
}
