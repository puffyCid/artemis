/**
* Windows `Outlook` is a popular emali client. Outlook on Windows stores messages in OST or PST files
* PST was used by older Outlook versions (prior to Outlook 2013)
* OST is used by Outlook 2013+*
*
* *Outlook was re-written in 2022 (New Outlook for Windows). Which is a online only web app. This parser does not support that version
*
* References:
* `https://www.forensafe.com/blogs/outlook.html`
* `https://github.com/libyal/libpff/blob/main/documentation/Personal%20Folder%20File%20(PFF)%20format.asciidoc`
*
* Other parsers:
* `https://github.com/libyal/libpff`
*
*/
use super::{
    error::OutlookError,
    header::FormatType,
    helper::{OutlookReader, OutlookReaderAction},
    items::message::MessageDetails,
    reader::{setup_outlook_reader, setup_outlook_reader_windows},
};
use crate::{
    artifacts::os::{systeminfo::info::get_platform, windows::artifacts::output_data},
    filesystem::{metadata::glob_paths, ntfs::setup::setup_ntfs_parser},
    structs::{artifacts::os::windows::OutlookOptions, toml::Output},
    utils::{
        environment::get_systemdrive,
        time::{compare_timestamps, time_now},
        yara::{scan_base64_bytes, scan_bytes},
    },
};
use common::windows::{OutlookAttachment, OutlookMessage};
use log::error;
use ntfs::NtfsFile;
use std::io::BufReader;

/// Parse and grab Outlook messages based on options provided
pub(crate) async fn grab_outlook(
    options: &OutlookOptions,
    output: &mut Output,
    filter: bool,
) -> Result<(), OutlookError> {
    if let Some(file) = &options.alt_file {
        return grab_outlook_file(file, options, filter, output).await;
    }
    let systemdrive_result = get_systemdrive();
    let drive = match systemdrive_result {
        Ok(result) => result,
        Err(err) => {
            error!("[outlook] Could not get systemdrive: {err:?}");
            return Err(OutlookError::Systemdrive);
        }
    };

    // Only OST files supported right now. Outlook 2013+
    let glob_path = format!("{drive}:\\Users\\*\\AppData\\Local\\Microsoft\\Outlook\\*.ost");
    let paths_result = glob_paths(&glob_path);
    let paths = match paths_result {
        Ok(result) => result,
        Err(err) => {
            error!("[outlook] Failed to glob: {glob_path}: {err:?}");
            return Err(OutlookError::GlobPath);
        }
    };

    for path in paths {
        let status = grab_outlook_file(&path.full_path, options, filter, output).await;
        if status.is_err() {
            error!(
                "[outlook] Could not extract messages from {}: {:?}",
                path.full_path,
                status.unwrap_err()
            );
        }
    }

    Ok(())
}

/// Parse the provided OST file and grab messages
async fn grab_outlook_file(
    path: &str,
    options: &OutlookOptions,
    filter: bool,
    output: &mut Output,
) -> Result<(), OutlookError> {
    let start_time = time_now();

    let runner = OutlookRunner {
        start_date: options.start_date.clone(),
        end_date: options.end_date.clone(),
        include_attachments: options.include_attachments,
        yara_rule_attachment: options.yara_rule_attachment.clone(),
        yara_rule_message: options.yara_rule_message.clone(),
        start_time,
        filter,
        source: path.to_string(),
    };

    let plat = get_platform();
    if plat != "Windows" {
        let reader = setup_outlook_reader(path)?;
        let buf_reader = BufReader::new(reader);

        let mut outlook_reader = OutlookReader {
            fs: buf_reader,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unknown,
            // This will get updated when parsing starts
            size: 4096,
        };
        return read_outlook(&mut outlook_reader, None, &runner, output).await;
    }

    // Windows we default to parsing the NTFS in order to bypass locked OST
    let ntfs_parser_result = setup_ntfs_parser(path.chars().next().unwrap_or('C'));
    let mut ntfs_parser = match ntfs_parser_result {
        Ok(result) => result,
        Err(err) => {
            error!("[outlook] Could not setup NTFS parser: {err:?}");
            return Err(OutlookError::Systemdrive);
        }
    };
    let ntfs_file = setup_outlook_reader_windows(&ntfs_parser.ntfs, &mut ntfs_parser.fs, path)?;

    let mut outlook_reader = OutlookReader {
        fs: ntfs_parser.fs,
        block_btree: Vec::new(),
        node_btree: Vec::new(),
        format: FormatType::Unknown,
        // This will get updated when parsing starts
        size: 4096,
    };

    read_outlook(&mut outlook_reader, Some(&ntfs_file), &runner, output).await
}

struct OutlookRunner {
    start_date: Option<String>,
    end_date: Option<String>,
    include_attachments: bool,
    yara_rule_attachment: Option<String>,
    yara_rule_message: Option<String>,
    start_time: u64,
    filter: bool,
    source: String,
}

/// Start reading the OST file
async fn read_outlook<T: std::io::Seek + std::io::Read>(
    reader: &mut OutlookReader<T>,
    use_ntfs: Option<&NtfsFile<'_>>,
    options: &OutlookRunner,
    output: &mut Output,
) -> Result<(), OutlookError> {
    // Parse the Outlook header and extract the initial BTrees, format type, and page size
    reader.setup(use_ntfs)?;

    // Get the root folder
    let root = reader.root_folder(use_ntfs)?;

    for folders in root.subfolders {
        stream_outlook(reader, use_ntfs, options, output, folders.node, &root.name).await?;
    }

    Ok(())
}

/// Loop and stream all folders and messages in OST
async fn stream_outlook<T: std::io::Seek + std::io::Read>(
    reader: &mut OutlookReader<T>,
    use_ntfs: Option<&NtfsFile<'_>>,
    options: &OutlookRunner,
    output: &mut Output,
    folder: u64,
    folder_path: &str,
) -> Result<(), OutlookError> {
    // Read the provided folder
    let mut results = reader.read_folder(use_ntfs, folder)?;

    // If no messages or no subfolders, we are done
    if results.message_count == 0 && results.subfolder_count == 0 {
        return Ok(());
    }

    let message_limit = 200;
    // Right now we only extract email messages
    if results.message_count != 0 && results.messages_table.has_branch.is_none() {
        // Easy parsing
        let mut chunks = Vec::new();
        for message in 0..results.message_count {
            chunks.push(message);
            if chunks.len() != message_limit {
                continue;
            }
            // If we are at the limit get messages
            results.messages_table.rows = chunks.clone();

            // Get our messages
            let messages = reader.read_message(use_ntfs, &results.messages_table, None)?;
            let mut entries = Vec::new();

            // Now process messages
            for message in messages {
                let entry = message_details(
                    message,
                    reader,
                    use_ntfs,
                    options,
                    folder_path,
                    &results.name,
                )?;
                if entry.is_none() {
                    continue;
                }
                entries.push(entry.unwrap());
            }

            output_messages(&entries, options, output).await?;
            chunks = Vec::new();
        }

        // Get any leftover messages
        if !chunks.is_empty() {
            results.messages_table.rows = chunks;

            // Get our messages
            let messages = reader.read_message(use_ntfs, &results.messages_table, None)?;
            let mut entries = Vec::new();

            // Now process messages
            for message in messages {
                let entry = message_details(
                    message,
                    reader,
                    use_ntfs,
                    options,
                    folder_path,
                    &results.name,
                )?;

                if entry.is_none() {
                    continue;
                }
                entries.push(entry.unwrap());
            }
            output_messages(&entries, options, output).await?;
        }

        // Now check for subfolders
        for folder in results.subfolders {
            let new_folder_path = format!("{folder_path}/{}", results.name);
            Box::pin(stream_outlook(
                reader,
                use_ntfs,
                options,
                output,
                folder.node,
                &new_folder_path,
            ))
            .await?;
        }

        return Ok(());
    }

    // We have branch messages. This is a bit more complex
    if let Some(branches) = &results.messages_table.has_branch {
        let mut all_rows = 0;
        // Loop through branches containing the messages
        for branch in branches {
            let mut chunks = Vec::new();
            // Each branch contains a collection of messages. Count depends messages size
            for message in all_rows..branch.rows_info.count + all_rows {
                chunks.push(message);
                if chunks.len() != message_limit {
                    continue;
                }
                // If we are at the limit get messages
                results.messages_table.rows = chunks.clone();

                // Get our messages
                let messages =
                    reader.read_message(use_ntfs, &results.messages_table, Some(branch))?;
                let mut entries = Vec::new();

                // Now process messages
                for message in messages {
                    let entry = message_details(
                        message,
                        reader,
                        use_ntfs,
                        options,
                        folder_path,
                        &results.name,
                    )?;

                    if entry.is_none() {
                        continue;
                    }

                    entries.push(entry.unwrap());
                }

                output_messages(&entries, options, output).await?;
                chunks = Vec::new();
            }

            // Get any leftover messages
            if !chunks.is_empty() {
                results.messages_table.rows = chunks;

                // Get our messages
                let messages =
                    reader.read_message(use_ntfs, &results.messages_table, Some(branch))?;
                let mut entries = Vec::new();

                // Now process messages
                for message in messages {
                    let entry = message_details(
                        message,
                        reader,
                        use_ntfs,
                        options,
                        folder_path,
                        &results.name,
                    )?;
                    if entry.is_none() {
                        continue;
                    }
                    entries.push(entry.unwrap());
                }

                output_messages(&entries, options, output).await?;
            }

            all_rows += branch.rows_info.count;
        }
    }

    // Now check for subfolders
    for folder in &results.subfolders {
        let new_folder_path = format!("{folder_path}/{}", results.name);
        Box::pin(stream_outlook(
            reader,
            use_ntfs,
            options,
            output,
            folder.node,
            &new_folder_path,
        ))
        .await?;
    }

    Ok(())
}

/// Read and extract message details. We only get attachments if explicitly enabled
fn message_details<T: std::io::Seek + std::io::Read>(
    message: MessageDetails,
    reader: &mut OutlookReader<T>,
    use_ntfs: Option<&NtfsFile<'_>>,
    options: &OutlookRunner,
    folder_path: &str,
    folder: &str,
) -> Result<Option<OutlookMessage>, OutlookError> {
    let mut message_result = OutlookMessage {
        body: message.body,
        subject: message.subject,
        from: message.from,
        recipient: message.recipient,
        delivered: message.delivered,
        recipients: message.recipients,
        attachments: Vec::new(),
        properties: message.props,
        folder_path: format!("{folder_path}/{folder}"),
        source_file: options.source.clone(),
        yara_hits: Vec::new(),
    };

    // Check if message body matches Yara rule
    if let Some(rule) = &options.yara_rule_message {
        let result = scan_bytes(message_result.body.as_bytes(), rule).unwrap_or_default();
        if result.is_empty() {
            return Ok(None);
        }

        message_result.yara_hits = result;
    }

    // Check if message occurs after our start data
    if let Some(start) = &options.start_date {
        let compare_result = compare_timestamps(&message_result.delivered, start);
        if compare_result.is_ok_and(|x| !x) {
            return Ok(None);
        }
    }

    // Check if message occurs before our end date
    if let Some(end) = &options.end_date {
        let compare_result = compare_timestamps(&message_result.delivered, end);
        if compare_result.is_ok_and(|x| x) {
            return Ok(None);
        }
    }

    let mut attachments = Vec::new();

    if options.include_attachments {
        for attach in &message.attachments {
            let attach_info =
                reader.read_attachment(use_ntfs, attach.block_id, attach.descriptor_id)?;

            let message_attach = OutlookAttachment {
                name: attach_info.name,
                size: attach_info.size,
                method: String::new(),
                mime: attach_info.mime,
                extension: attach_info.extension,
                data: attach_info.data,
                properties: attach_info.props,
            };

            // Check if attachment matches Yara rule
            if let Some(rule) = &options.yara_rule_attachment {
                let result = scan_base64_bytes(&message_attach.data, rule).unwrap_or_default();
                if result.is_empty() {
                    continue;
                }

                attachments.push(message_attach);
                continue;
            }
            attachments.push(message_attach);
        }

        message_result.attachments = attachments;
    }

    Ok(Some(message_result))
}

/// Output the extract messages
async fn output_messages(
    messages: &[OutlookMessage],
    options: &OutlookRunner,
    output: &mut Output,
) -> Result<(), OutlookError> {
    if messages.is_empty() {
        return Ok(());
    }
    let serde_data_result = serde_json::to_value(messages);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[outlook] Failed to serialize Outlook messages: {err:?}");
            return Err(OutlookError::Serialize);
        }
    };
    let result = output_data(
        &mut serde_data,
        "outlook",
        output,
        options.start_time,
        options.filter,
    )
    .await;
    match result {
        Ok(_result) => {}
        Err(err) => {
            error!("[outlook] Could not output Outlook messages: {err:?}");
            return Err(OutlookError::OutputData);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::grab_outlook;
    use crate::structs::{artifacts::os::windows::OutlookOptions, toml::Output};
    use std::path::PathBuf;

    #[tokio::test]
    #[cfg(target_family = "unix")]
    async fn test_grab_outlook() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/outlook/windows11/test@outlook.com.ost");

        let options = OutlookOptions {
            alt_file: Some(test_location.to_str().unwrap().to_string()),
            include_attachments: true,
            start_date: None,
            end_date: None,
            yara_rule_message: None,
            yara_rule_attachment: None,
        };

        let mut out = Output {
            name: "outlook_temp".to_string(),
            directory: "./tmp".to_string(),
            format: String::from("jsonl"),
            compress: false,
            timeline: false,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: "local".to_string(),
            filter_name: None,
            filter_script: None,
            logging: None,
        };

        grab_outlook(&options, &mut out, false).await.unwrap()
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_grab_outlook_windows_alt() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\outlook\\windows11\\test@outlook.com.ost");

        let options = OutlookOptions {
            alt_file: Some(test_location.to_str().unwrap().to_string()),
            include_attachments: true,
            start_date: None,
            end_date: None,
            yara_rule_message: None,
            yara_rule_attachment: None,
        };

        let mut out = Output {
            name: "outlook_temp".to_string(),
            directory: "./tmp".to_string(),
            format: String::from("jsonl"),
            compress: false,
            timeline: false,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: "local".to_string(),
            filter_name: None,
            filter_script: None,
            logging: None,
        };

        grab_outlook(&options, &mut out, false).unwrap()
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_grab_outlook_windows() {
        let options = OutlookOptions {
            alt_file: None,
            include_attachments: true,
            start_date: None,
            end_date: None,
            yara_rule_message: None,
            yara_rule_attachment: None,
        };

        let mut out = Output {
            name: "outlook_temp".to_string(),
            directory: "./tmp".to_string(),
            format: String::from("jsonl"),
            compress: false,
            timeline: false,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: "local".to_string(),
            filter_name: None,
            filter_script: None,
            logging: None,
        };

        grab_outlook(&options, &mut out, false).unwrap()
    }
}
