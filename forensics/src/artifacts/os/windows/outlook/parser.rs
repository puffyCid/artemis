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
    artifacts::os::systeminfo::info::get_platform,
    filesystem::{metadata::glob_paths, ntfs::setup::setup_ntfs_parser},
    output::{manager::OutputManager, record::serialize_records_to_stream},
    structs::artifacts::os::windows::OutlookOptions,
    utils::{environment::get_systemdrive, time::compare_timestamps},
};
use common::windows::{OutlookAttachment, OutlookMessage};
use ntfs::NtfsFile;
use std::io::BufReader;
use tracing::error;

#[cfg(feature = "yarax")]
use crate::utils::yara::{scan_base64_bytes, scan_bytes};

/// Parse and grab Outlook messages based on options provided
pub(crate) fn grab_outlook(
    options: &OutlookOptions,
    manager: &mut OutputManager,
) -> Result<(), OutlookError> {
    if let Some(file) = &options.alt_file {
        return grab_outlook_file(file, options, manager);
    }
    let systemdrive_result = get_systemdrive();
    let drive = match systemdrive_result {
        Ok(result) => result,
        Err(err) => {
            error!("Could not get systemdrive: {err:?}");
            return Err(OutlookError::Systemdrive);
        }
    };

    // Only OST files supported right now. Outlook 2013+
    let glob_path = format!("{drive}:\\Users\\*\\AppData\\Local\\Microsoft\\Outlook\\*.ost");
    let paths_result = glob_paths(&glob_path);
    let paths = match paths_result {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to glob: {glob_path}: {err:?}");
            return Err(OutlookError::GlobPath);
        }
    };

    for path in paths {
        let status = grab_outlook_file(&path.full_path, options, manager);
        if let Err(result) = status {
            error!(
                "Could not extract messages from {}: {result:?}",
                path.full_path
            );
        }
    }

    Ok(())
}

/// Parse the provided OST file and grab messages
fn grab_outlook_file(
    path: &str,
    options: &OutlookOptions,
    manager: &mut OutputManager,
) -> Result<(), OutlookError> {
    let runner = OutlookRunner {
        start_date: options.start_date.clone(),
        end_date: options.end_date.clone(),
        include_attachments: options.include_attachments,
        yara_rule_attachment: options.yara_rule_attachment.clone(),
        yara_rule_message: options.yara_rule_message.clone(),
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
        return read_outlook(&mut outlook_reader, None, &runner, manager, options);
    }

    // Windows we default to parsing the NTFS in order to bypass locked OST
    let ntfs_parser_result = setup_ntfs_parser(path.chars().next().unwrap_or('C'));
    let mut ntfs_parser = match ntfs_parser_result {
        Ok(result) => result,
        Err(err) => {
            error!("Could not setup NTFS parser: {err:?}");
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

    read_outlook(
        &mut outlook_reader,
        Some(&ntfs_file),
        &runner,
        manager,
        options,
    )
}

struct OutlookRunner {
    start_date: Option<String>,
    end_date: Option<String>,
    include_attachments: bool,
    yara_rule_attachment: Option<String>,
    yara_rule_message: Option<String>,
    source: String,
}

/// Start reading the OST file
fn read_outlook<T: std::io::Seek + std::io::Read>(
    reader: &mut OutlookReader<T>,
    use_ntfs: Option<&NtfsFile<'_>>,
    options: &OutlookRunner,
    manager: &mut OutputManager,
    params: &OutlookOptions,
) -> Result<(), OutlookError> {
    // Parse the Outlook header and extract the initial BTrees, format type, and page size
    reader.setup(use_ntfs)?;

    // Get the root folder
    let root = reader.root_folder(use_ntfs)?;

    for folders in root.subfolders {
        stream_outlook(
            reader,
            use_ntfs,
            options,
            manager,
            params,
            folders.node,
            &root.name,
        )?;
    }

    Ok(())
}

/// Loop and stream all folders and messages in OST
fn stream_outlook<T: std::io::Seek + std::io::Read>(
    reader: &mut OutlookReader<T>,
    use_ntfs: Option<&NtfsFile<'_>>,
    options: &OutlookRunner,
    manager: &mut OutputManager,
    params: &OutlookOptions,
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

            output_messages(entries, manager, params)?;
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
            output_messages(entries, manager, params)?;
        }

        // Now check for subfolders
        for folder in results.subfolders {
            let new_folder_path = format!("{folder_path}/{}", results.name);
            stream_outlook(
                reader,
                use_ntfs,
                options,
                manager,
                params,
                folder.node,
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

                    if entry.is_none() {
                        continue;
                    }

                    entries.push(entry.unwrap());
                }

                output_messages(entries, manager, params)?;
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

                output_messages(entries, manager, params)?;
            }

            all_rows += branch.rows_info.count;
        }
    }

    // Now check for subfolders
    for folder in &results.subfolders {
        let new_folder_path = format!("{folder_path}/{}", results.name);
        stream_outlook(
            reader,
            use_ntfs,
            options,
            manager,
            params,
            folder.node,
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
        evidence: options.source.clone(),
        yara_hits: Vec::new(),
    };

    #[cfg(feature = "yarax")]
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

            #[cfg(feature = "yarax")]
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
fn output_messages(
    messages: Vec<OutlookMessage>,
    manager: &mut OutputManager,
    options: &OutlookOptions,
) -> Result<(), OutlookError> {
    if messages.is_empty() {
        return Ok(());
    }
    let mut records = match serialize_records_to_stream(messages) {
        Ok(results) => results,
        Err(err) => {
            error!("Failed to serialize Outlook messages: {err:?}");
            return Err(OutlookError::Serialize);
        }
    };
    let artifact_name = "outlook";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("Could not output Outlook messages: {err:?}");
        return Err(OutlookError::OutputData);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::grab_outlook;
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use crate::{output::manager::OutputManager, structs::artifacts::os::windows::OutlookOptions};
    use std::path::PathBuf;

    fn output_options(name: &str, directory: &str, compress: bool) -> OutputManager {
        let config = OutputConfig {
            name: name.to_string(),
            directory: PathBuf::from(directory),
            format: OutputFormat::Jsonl,
            compress,
            endpoint_id: String::from("abcd"),
            destination: OutputDestination::Local,
            ..Default::default()
        };
        OutputManager::new(config).unwrap()
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn test_grab_outlook() {
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

        let mut out = output_options("outlook_temp", "./tmp", false);

        grab_outlook(&options, &mut out).unwrap()
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

        let mut out = output_options("outlook_temp", "./tmp", false);

        grab_outlook(&options, &mut out).unwrap()
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

        let mut out = output_options("outlook_temp", "./tmp", false);

        grab_outlook(&options, &mut out).unwrap()
    }
}
