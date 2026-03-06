use crate::artifacts::os::windows::tasks::text::read_text_unescaped;
use common::windows::Principals;
use log::error;
use quick_xml::{Reader, events::Event};

/// Parse Principal information for Task
pub(crate) fn parse_principals(reader: &mut Reader<&[u8]>) -> Principals {
    let mut info = Principals {
        user_id: None,
        logon_type: None,
        group_id: None,
        display_name: None,
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
                    info.user_id = Some(read_text_unescaped(reader, tag.name()));
                }
                b"LogonType" => {
                    info.logon_type = Some(read_text_unescaped(reader, tag.name()));
                }
                b"GroupId" => {
                    info.group_id = Some(read_text_unescaped(reader, tag.name()));
                }
                b"DisplayName" => {
                    info.display_name = Some(read_text_unescaped(reader, tag.name()));
                }
                b"RunLevel" => {
                    info.run_level = Some(read_text_unescaped(reader, tag.name()));
                }
                b"ProcessTokenSidType" => {
                    info.process_token_sid_type = Some(read_text_unescaped(reader, tag.name()));
                }
                b"Privilege" => {
                    privs.push(read_text_unescaped(reader, tag.name()));
                }
                b"id" => {
                    info.id_attribute = Some(read_text_unescaped(reader, tag.name()));
                }
                _ => (),
            },
            Ok(Event::End(tag)) => {
                if tag.name().as_ref() == b"Principals" {
                    break;
                }
            }
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
    use crate::artifacts::os::windows::tasks::schemas::principals::parse_principals;
    use quick_xml::Reader;

    #[test]
    fn test_parse_principals() {
        let xml = r#"
        <UserId>S-1-5-18</UserId>
        <ProcessTokenSidType>S-1-5-122228</ProcessTokenSidType>
        <Privilege>SuperAdmin</Privilege>
             "#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        let result = parse_principals(&mut reader);
        assert_eq!(result.user_id.unwrap(), "S-1-5-18");
        assert_eq!(result.required_privileges.unwrap()[0], "SuperAdmin");
    }
}
