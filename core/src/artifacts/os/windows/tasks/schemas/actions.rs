use common::windows::{Actions, ComHandlerType, ExecType, Message, SendEmail};
use log::error;
use quick_xml::{Reader, events::Event};
use std::collections::HashMap;

/// Parse all Task Actions
pub(crate) fn parse_actions(reader: &mut Reader<&[u8]>) -> Actions {
    let mut info = Actions {
        exec: Vec::new(),
        com_handler: Vec::new(),
        send_email: Vec::new(),
        show_message: Vec::new(),
    };

    loop {
        match reader.read_event() {
            Err(err) => {
                error!("[tasks] Could not read Settings xml data: {err:?}");
                break;
            }
            Ok(Event::Eof) => break,
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                b"Exec" => info.exec.push(process_exec(reader)),
                b"ComHandler" => info.com_handler.push(process_com(reader)),
                b"SendEmail" => info.send_email.push(process_email(reader)),
                b"ShowMessage" => info.show_message.push(process_message(reader)),
                _ => break,
            },
            Ok(Event::End(tag)) => {
                if tag.name().as_ref() == b"Actions" {
                    break;
                }
            }
            _ => (),
        }
    }

    info
}

/// Parse Execution Task Action
fn process_exec(reader: &mut Reader<&[u8]>) -> ExecType {
    let mut exec = ExecType {
        command: String::new(),
        arguments: None,
        working_directory: None,
    };

    loop {
        match reader.read_event() {
            Err(err) => {
                error!("[tasks] Could not read Exec xml data: {err:?}");
                break;
            }
            Ok(Event::Eof) => break,
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                b"Command" => {
                    exec.command = reader.read_text(tag.name()).unwrap_or_default().to_string();
                }
                b"Arguments" => {
                    exec.arguments =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string());
                }
                b"WorkingDirectory" => {
                    exec.working_directory =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string());
                }
                _ => break,
            },
            Ok(Event::End(tag)) => {
                if tag.name().as_ref() == b"Exec" {
                    break;
                }
            }
            _ => (),
        }
    }

    exec
}

/// Parse `COMHander` Task Action
fn process_com(reader: &mut Reader<&[u8]>) -> ComHandlerType {
    let mut com = ComHandlerType {
        class_id: String::new(),
        data: None,
    };

    loop {
        match reader.read_event() {
            Err(err) => {
                error!("[tasks] Could not read ComHandler xml data: {err:?}");
                break;
            }
            Ok(Event::Eof) => break,
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                b"ClassId" => {
                    com.class_id = reader.read_text(tag.name()).unwrap_or_default().to_string();
                }
                b"Data" => {
                    com.data = Some(reader.read_text(tag.name()).unwrap_or_default().to_string());
                }
                _ => break,
            },
            Ok(Event::End(tag)) => {
                if tag.name().as_ref() == b"ComHandler " {
                    break;
                }
            }
            _ => (),
        }
    }

    com
}

/// Parse Email Task Action
fn process_email(reader: &mut Reader<&[u8]>) -> SendEmail {
    let mut email = SendEmail {
        server: None,
        subject: None,
        to: None,
        cc: None,
        bcc: None,
        reply_to: None,
        from: String::new(),
        header_fields: None,
        body: None,
        attachment: None,
    };
    let mut header_key = String::new();
    let mut header_value = String::new();

    let mut headers = HashMap::new();
    let mut attachments = Vec::new();
    loop {
        match reader.read_event() {
            Err(err) => {
                error!("[tasks] Could not read SendEmail xml data: {err:?}");
                break;
            }
            Ok(Event::Eof) => break,
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                b"Server" => {
                    email.server =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string());
                }
                b"Subject" => {
                    email.subject =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string());
                }
                b"To" => {
                    email.to = Some(reader.read_text(tag.name()).unwrap_or_default().to_string());
                }
                b"Cc" => {
                    email.cc = Some(reader.read_text(tag.name()).unwrap_or_default().to_string());
                }
                b"Bcc" => {
                    email.bcc = Some(reader.read_text(tag.name()).unwrap_or_default().to_string());
                }
                b"ReplyTo" => {
                    email.reply_to =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string());
                }
                b"From" => {
                    email.from = reader.read_text(tag.name()).unwrap_or_default().to_string();
                }
                b"Name" => {
                    header_key = reader.read_text(tag.name()).unwrap_or_default().to_string();
                }
                b"Value" => {
                    header_value = reader.read_text(tag.name()).unwrap_or_default().to_string();
                }
                b"File" => {
                    attachments.push(reader.read_text(tag.name()).unwrap_or_default().to_string());
                }
                _ => (),
            },
            Ok(Event::End(tag)) => {
                if tag.name().as_ref() == b"SendEmail " {
                    break;
                }
            }
            _ => (),
        }

        // If we have both email header key and value. Add to hashmap tracker
        if !header_key.is_empty() && !header_value.is_empty() {
            headers.insert(header_key, header_value);

            header_key = String::new();
            header_value = String::new();
        }
    }

    email.header_fields = Some(headers);
    email.attachment = Some(attachments);

    email
}

/// Parse Message popup Task Action
fn process_message(reader: &mut Reader<&[u8]>) -> Message {
    let mut message = Message {
        title: None,
        body: String::new(),
    };

    loop {
        match reader.read_event() {
            Err(err) => {
                error!("[tasks] Could not read SendMessage xml data: {err:?}");
                break;
            }
            Ok(Event::Eof) => break,
            Ok(Event::Start(tag)) => match tag.name().as_ref() {
                b"Body" => {
                    message.body = reader.read_text(tag.name()).unwrap_or_default().to_string();
                }
                b"Title" => {
                    message.title =
                        Some(reader.read_text(tag.name()).unwrap_or_default().to_string());
                }
                _ => break,
            },
            Ok(Event::End(tag)) => {
                if tag.name().as_ref() == b"ShowMessage  " {
                    break;
                }
            }
            _ => (),
        }
    }

    message
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::tasks::schemas::actions::{
        parse_actions, process_com, process_email, process_exec, process_message,
    };
    use quick_xml::Reader;

    #[test]
    fn test_parse_actions() {
        let xml = r#"
        <Exec>
        <Command>C:\Program Files (x86)\Microsoft Visual Studio\Installer\resources\app\ServiceHub\Services\Microsoft.VisualStudio.Setup.Service\VSIXAutoUpdate.exe</Command>
      </Exec>
             "#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        let result = parse_actions(&mut reader);
        assert_eq!(
            result.exec[0].command,
            "C:\\Program Files (x86)\\Microsoft Visual Studio\\Installer\\resources\\app\\ServiceHub\\Services\\Microsoft.VisualStudio.Setup.Service\\VSIXAutoUpdate.exe"
        );
    }

    #[test]
    fn test_process_exec() {
        let xml = r#"
        <Command>C:\Program Files (x86)\Microsoft Visual Studio\Installer\resources\app\ServiceHub\Services\Microsoft.VisualStudio.Setup.Service\VSIXAutoUpdate.exe</Command>
        <Arguments>-all</Arguments>
        <WorkingDirectory>here</WorkingDirectory>
        "#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        let result = process_exec(&mut reader);
        assert_eq!(
            result.command,
            "C:\\Program Files (x86)\\Microsoft Visual Studio\\Installer\\resources\\app\\ServiceHub\\Services\\Microsoft.VisualStudio.Setup.Service\\VSIXAutoUpdate.exe"
        );
        assert_eq!(result.arguments.unwrap(), "-all");
        assert_eq!(result.working_directory.unwrap(), "here");
    }

    #[test]
    fn test_process_com() {
        let xml = r#"
        <ClassId>111-222-33389091-12321-4252asdf</ClassId>
        <Data>whatever</Data>
        "#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        let result = process_com(&mut reader);
        assert_eq!(result.class_id, "111-222-33389091-12321-4252asdf");
        assert_eq!(result.data.unwrap(), "whatever");
    }

    #[test]
    fn test_process_email() {
        let xml = r#"
        <To>help@rust[.]com</To>
        <From>me</From>
        <Server>mozila</Server>
        <Cc>thunderbird</Cc>
        <Bcc>netscape</Bcc>
        <ReplyTo>mozilla</ReplyTo>
        <Name>test</Name>
        <Value>value</Value>
        <File>help.docx</File>
        <Subject>Help in rust!</Subject>
        "#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        let result = process_email(&mut reader);
        assert_eq!(result.to.unwrap(), "help@rust[.]com");
        assert_eq!(result.from, "me");
    }

    #[test]
    fn test_process_message() {
        let xml = r#"
        <Body>messsage here</Body>
        <Title>Fancy</Title>
        "#;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);
        let result = process_message(&mut reader);
        assert_eq!(result.body, "messsage here");
        assert_eq!(result.title.unwrap(), "Fancy");
    }
}
