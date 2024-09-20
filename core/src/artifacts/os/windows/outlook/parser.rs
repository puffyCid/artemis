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
    utils::{environment::get_systemdrive, time::time_now},
};
use common::windows::{OutlookAttachment, OutlookMessage};
use log::error;
use ntfs::NtfsFile;
use std::io::BufReader;

/// Parse and grab Outlook messages based on options provided
pub(crate) fn grab_outlook(
    options: &OutlookOptions,
    output: &mut Output,
    filter: &bool,
) -> Result<(), OutlookError> {
    if let Some(file) = &options.alt_file {
        return grab_outlook_file(file, options, filter, output);
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
        println!("parsing: {}", path.full_path);
        let status = grab_outlook_file(&path.full_path, options, filter, output);
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
fn grab_outlook_file(
    path: &str,
    options: &OutlookOptions,
    filter: &bool,
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
        filter: *filter,
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
        return read_outlook(&mut outlook_reader, None, &runner, output);
    }

    // Windows we default to parsing the NTFS in order to bypass locked OST
    let ntfs_parser_result = setup_ntfs_parser(&path.chars().next().unwrap_or('C'));
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

    return read_outlook(&mut outlook_reader, Some(&ntfs_file), &runner, output);
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
fn read_outlook<T: std::io::Seek + std::io::Read>(
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
        stream_outlook(reader, use_ntfs, options, output, &folders.node, &root.name)?;
    }

    Ok(())
}

/// Loop and stream all folders and messages in OST
fn stream_outlook<T: std::io::Seek + std::io::Read>(
    reader: &mut OutlookReader<T>,
    use_ntfs: Option<&NtfsFile<'_>>,
    options: &OutlookRunner,
    output: &mut Output,
    folder: &u64,
    folder_path: &str,
) -> Result<(), OutlookError> {
    println!("reading: {folder_path}");
    // Read the provided folder
    let mut results = reader.read_folder(use_ntfs, *folder)?;
    println!("got folder: {:?}", results);

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
                entries.push(entry);
            }

            output_messages(&entries, options, output)?;
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
                println!("email message: {entry:?}");
                entries.push(entry);
            }
            output_messages(&entries, options, output)?;
        }

        // Now check for subfolders
        for folder in results.subfolders {
            let new_folder_path = format!("{folder_path}/{}", results.name);
            stream_outlook(
                reader,
                use_ntfs,
                options,
                output,
                &folder.node,
                &new_folder_path,
            )?;
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

                    entries.push(entry);
                }

                output_messages(&entries, options, output)?;
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
                    entries.push(entry);
                }

                output_messages(&entries, options, output)?;
            }

            all_rows += branch.rows_info.count;
        }
    }

    // Now check for subfolders
    for folder in &results.subfolders {
        println!("sub folder: {} - {}", folder.name, folder.node);

        let new_folder_path = format!("{folder_path}/{}", results.name);
        stream_outlook(
            reader,
            use_ntfs,
            options,
            output,
            &folder.node,
            &new_folder_path,
        )?;
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
) -> Result<OutlookMessage, OutlookError> {
    let mut message_result = OutlookMessage {
        body: message.body,
        subject: message.subject,
        from: message.from,
        recipient: message.recipient,
        delivered: message.delivered,
        recipients: Vec::new(),
        attachments: Vec::new(),
        properties: Vec::new(),
        folder_path: format!("{folder_path}/{folder}"),
        source_file: options.source.clone(),
    };
    if let Some(start) = &options.start_date {
        println!("filter by start date");
        return Ok(message_result);
    }
    if let Some(end) = &options.end_date {
        println!("filter by end date");
        return Ok(message_result);
    }
    let mut attachments = Vec::new();

    if options.include_attachments {
        for attach in &message.attachments {
            let attach_info =
                reader.read_attachment(use_ntfs, &attach.block_id, &attach.descriptor_id)?;

            let message_attach = OutlookAttachment {
                name: attach_info.name,
                size: attach_info.size,
                method: String::new(),
                mime: attach_info.mime,
                extension: attach_info.extension,
                data: attach_info.data,
                properties: Vec::new(),
            };
            attachments.push(message_attach);
        }
        if let Some(rule) = &options.yara_rule_attachment {
            println!("scan with yara!");
        }

        if let Some(rule) = &options.yara_rule_attachment {
            println!("scan with yara!");
        }

        message_result.attachments = attachments;
    }

    if let Some(rule) = &options.yara_rule_message {
        println!("scan message with yara!");
        return Ok(message_result);
    }
    return Ok(message_result);
}

/// Output the extract messages
fn output_messages(
    messages: &[OutlookMessage],
    options: &OutlookRunner,
    output: &mut Output,
) -> Result<(), OutlookError> {
    if messages.is_empty() {
        return Ok(());
    }
    let serde_data_result = serde_json::to_value(&messages);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[outlook] Failed to serialize Outlook messages: {err:?}");
            return Err(OutlookError::Serialize);
        }
    };
    let result = output_data(
        &serde_data,
        "outlook",
        output,
        &options.start_time,
        &options.filter,
    );
    match result {
        Ok(_result) => {}
        Err(err) => {
            error!("[outlook] Could not output Outlook messages: {err:?}");
            return Err(OutlookError::OutputData);
        }
    }

    Ok(())
}
