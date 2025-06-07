use crate::{
    artifacts::os::windows::outlook::{
        header::FormatType,
        helper::{OutlookReader, OutlookReaderAction},
        reader::{setup_outlook_reader, setup_outlook_reader_windows},
        tables::context::TableInfo,
    },
    filesystem::ntfs::setup::setup_ntfs_parser,
    runtime::helper::{bigint_arg, boolean_arg, string_arg, value_arg},
};
use boa_engine::{Context, JsError, JsResult, JsValue, js_string};
use log::error;
use std::io::BufReader;

pub(crate) fn js_root_folder(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, &0)?;
    let use_ntfs = boolean_arg(args, &1, context)?;

    let root_result = if use_ntfs {
        let mut ntfs_parser = match setup_ntfs_parser(&path.chars().next().unwrap_or('C')) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Failed to setup NTFS reader: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };
        let ntfs_file =
            match setup_outlook_reader_windows(&ntfs_parser.ntfs, &mut ntfs_parser.fs, &path) {
                Ok(result) => result,
                Err(err) => {
                    let issue = format!("Failed to setup NTFS outlook reader: {err:?}");
                    return Err(JsError::from_opaque(js_string!(issue).into()));
                }
            };

        let mut reader = OutlookReader {
            fs: ntfs_parser.fs,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unknown,
            // This will get updated when parsing starts
            size: 4096,
        };

        let err = reader.setup(Some(&ntfs_file));
        if err.is_err() {
            let issue = format!("Failed to setup outlook reader: {:?}", err.unwrap_err());
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
        reader.root_folder(Some(&ntfs_file))
    } else {
        let reader = match setup_outlook_reader(&path) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Failed to setup reader: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };
        let buf_reader = BufReader::new(reader);

        let mut reader = OutlookReader {
            fs: buf_reader,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unknown,
            // This will get updated when parsing starts
            size: 4096,
        };

        let err = reader.setup(None);
        if err.is_err() {
            let issue = format!("Failed to setup outlook reader: {:?}", err.unwrap_err());
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
        reader.root_folder(None)
    };

    let root = match root_result {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to read root folder: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&root).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

pub(crate) fn js_message_store(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, &0)?;
    let use_ntfs = boolean_arg(args, &1, context)?;

    let root_result = if use_ntfs {
        let mut ntfs_parser = match setup_ntfs_parser(&path.chars().next().unwrap_or('C')) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Failed to setup NTFS reader: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };
        let ntfs_file =
            match setup_outlook_reader_windows(&ntfs_parser.ntfs, &mut ntfs_parser.fs, &path) {
                Ok(result) => result,
                Err(err) => {
                    let issue = format!("Failed to setup NTFS outlook reader: {err:?}");
                    return Err(JsError::from_opaque(js_string!(issue).into()));
                }
            };

        let mut reader = OutlookReader {
            fs: ntfs_parser.fs,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unknown,
            // This will get updated when parsing starts
            size: 4096,
        };

        let err = reader.setup(Some(&ntfs_file));
        if err.is_err() {
            let issue = format!("Failed to setup outlook reader: {:?}", err.unwrap_err());
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
        reader.message_store(Some(&ntfs_file))
    } else {
        let reader = match setup_outlook_reader(&path) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Failed to setup reader: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };
        let buf_reader = BufReader::new(reader);

        let mut reader = OutlookReader {
            fs: buf_reader,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unknown,
            // This will get updated when parsing starts
            size: 4096,
        };

        let err = reader.setup(None);
        if err.is_err() {
            let issue = format!("Failed to setup outlook reader: {:?}", err.unwrap_err());
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
        reader.message_store(None)
    };

    let root = match root_result {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to read root folder: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&root).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

pub(crate) fn js_name_map(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, &0)?;
    let use_ntfs = boolean_arg(args, &1, context)?;

    let root_result = if use_ntfs {
        let mut ntfs_parser = match setup_ntfs_parser(&path.chars().next().unwrap_or('C')) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Failed to setup NTFS reader: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };
        let ntfs_file =
            match setup_outlook_reader_windows(&ntfs_parser.ntfs, &mut ntfs_parser.fs, &path) {
                Ok(result) => result,
                Err(err) => {
                    let issue = format!("Failed to setup NTFS outlook reader: {err:?}");
                    return Err(JsError::from_opaque(js_string!(issue).into()));
                }
            };

        let mut reader = OutlookReader {
            fs: ntfs_parser.fs,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unknown,
            // This will get updated when parsing starts
            size: 4096,
        };

        let err = reader.setup(Some(&ntfs_file));
        if err.is_err() {
            let issue = format!("Failed to setup outlook reader: {:?}", err.unwrap_err());
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
        reader.name_id_map(Some(&ntfs_file))
    } else {
        let reader = match setup_outlook_reader(&path) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Failed to setup reader: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };
        let buf_reader = BufReader::new(reader);

        let mut reader = OutlookReader {
            fs: buf_reader,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unknown,
            // This will get updated when parsing starts
            size: 4096,
        };

        let err = reader.setup(None);
        if err.is_err() {
            let issue = format!("Failed to setup outlook reader: {:?}", err.unwrap_err());
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
        reader.name_id_map(None)
    };

    let root = match root_result {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to read root folder: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&root).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

pub(crate) fn js_read_folder(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, &0)?;
    let use_ntfs: bool = boolean_arg(args, &1, context)?;
    let folder_id = bigint_arg(args, &2)? as u64;

    let folder_result = if use_ntfs {
        let mut ntfs_parser = match setup_ntfs_parser(&path.chars().next().unwrap_or('C')) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Failed to setup NTFS reader: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };
        let ntfs_file =
            match setup_outlook_reader_windows(&ntfs_parser.ntfs, &mut ntfs_parser.fs, &path) {
                Ok(result) => result,
                Err(err) => {
                    let issue = format!("Failed to setup NTFS outlook reader: {err:?}");
                    return Err(JsError::from_opaque(js_string!(issue).into()));
                }
            };

        let mut reader = OutlookReader {
            fs: ntfs_parser.fs,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unknown,
            // This will get updated when parsing starts
            size: 4096,
        };

        let err = reader.setup(Some(&ntfs_file));
        if err.is_err() {
            let issue = format!("Failed to setup outlook reader: {:?}", err.unwrap_err());
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
        reader.read_folder(Some(&ntfs_file), folder_id)
    } else {
        let reader = match setup_outlook_reader(&path) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Failed to setup reader: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };
        let buf_reader = BufReader::new(reader);

        let mut reader = OutlookReader {
            fs: buf_reader,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unknown,
            // This will get updated when parsing starts
            size: 4096,
        };

        let err = reader.setup(None);
        if err.is_err() {
            let issue = format!("Failed to setup outlook reader: {:?}", err.unwrap_err());
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
        reader.read_folder(None, folder_id)
    };

    let folder = match folder_result {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to read folder: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&folder).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

pub(crate) fn js_read_messages(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, &0)?;
    let use_ntfs = boolean_arg(args, &1, context)?;
    let table = value_arg(args, &2, context)?;
    let offset = bigint_arg(args, &3)? as u64;
    let message_table: TableInfo = match serde_json::from_value(table) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to deserialize TableInfo: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let messages = if use_ntfs {
        let mut ntfs_parser = match setup_ntfs_parser(&path.chars().next().unwrap_or('C')) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Failed to setup NTFS reader: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };
        let ntfs_file =
            match setup_outlook_reader_windows(&ntfs_parser.ntfs, &mut ntfs_parser.fs, &path) {
                Ok(result) => result,
                Err(err) => {
                    let issue = format!("Failed to setup NTFS outlook reader: {err:?}");
                    return Err(JsError::from_opaque(js_string!(issue).into()));
                }
            };

        let mut reader = OutlookReader {
            fs: ntfs_parser.fs,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unknown,
            // This will get updated when parsing starts
            size: 4096,
        };

        let err = reader.setup(Some(&ntfs_file));
        if err.is_err() {
            let issue = format!("Failed to setup outlook reader: {:?}", err.unwrap_err());
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
        // This is difficult
        if message_table.has_branch.is_some() {
            let mut main_count = 0;
            let mut chunks = Vec::new();
            // Each branch has a collection of messages. Ex: Messages 0-20
            for branch in message_table.has_branch.as_ref().unwrap() {
                // If the offset is greater than the current branch message count.
                // Go to next branch. Ex: Branch 1 has messages 0-20. Branch 2 has messages 21-40, etc
                if offset > branch.rows_info.count + main_count {
                    main_count += branch.rows_info.count;
                    continue;
                }

                let mut emails = match reader.read_message(Some(&ntfs_file), &message_table, None) {
                    Ok(result) => result,
                    Err(err) => {
                        error!("[runtime] Failed to read message {err:?}");
                        continue;
                    }
                };
                chunks.append(&mut emails);
            }
            chunks
        } else {
            match reader.read_message(Some(&ntfs_file), &message_table, None) {
                Ok(result) => result,
                Err(err) => {
                    let issue = format!("Failed to read messages: {err:?}");
                    return Err(JsError::from_opaque(js_string!(issue).into()));
                }
            }
        }
    } else {
        let reader = match setup_outlook_reader(&path) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Failed to setup reader: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };
        let buf_reader = BufReader::new(reader);

        let mut reader = OutlookReader {
            fs: buf_reader,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unknown,
            // This will get updated when parsing starts
            size: 4096,
        };

        let err = reader.setup(None);
        if err.is_err() {
            let issue = format!("Failed to setup outlook reader: {:?}", err.unwrap_err());
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
        if message_table.has_branch.is_some() {
            let mut main_count = 0;
            let mut chunks = Vec::new();
            // Each branch has a collection of messages. Ex: Messages 0-20
            for branch in message_table.has_branch.as_ref().unwrap() {
                // If the offset is greater than the current branch message count.
                // Go to next branch. Ex: Branch 1 has messages 0-20. Branch 2 has messages 21-40, etc
                if offset > branch.rows_info.count + main_count {
                    main_count += branch.rows_info.count;
                    continue;
                }

                let mut emails = match reader.read_message(None, &message_table, None) {
                    Ok(result) => result,
                    Err(err) => {
                        error!("[runtime] Failed to read message {err:?}");
                        continue;
                    }
                };
                chunks.append(&mut emails);
                if chunks.len() < message_table.rows.len() {
                    continue;
                }
                break;
            }
            chunks
        } else {
            match reader.read_message(None, &message_table, None) {
                Ok(result) => result,
                Err(err) => {
                    let issue = format!("Failed to read messages: {err:?}");
                    return Err(JsError::from_opaque(js_string!(issue).into()));
                }
            }
        }
    };

    let results = serde_json::to_value(&messages).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

pub(crate) fn js_read_attachment(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, &0)?;
    let use_ntfs = boolean_arg(args, &1, context)?;
    let block_id = bigint_arg(args, &2)? as u64;
    let descriptor_id = bigint_arg(args, &3)? as u64;

    let attachment_result = if use_ntfs {
        let mut ntfs_parser = match setup_ntfs_parser(&path.chars().next().unwrap_or('C')) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Failed to setup NTFS reader: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };
        let ntfs_file =
            match setup_outlook_reader_windows(&ntfs_parser.ntfs, &mut ntfs_parser.fs, &path) {
                Ok(result) => result,
                Err(err) => {
                    let issue = format!("Failed to setup NTFS outlook reader: {err:?}");
                    return Err(JsError::from_opaque(js_string!(issue).into()));
                }
            };

        let mut reader = OutlookReader {
            fs: ntfs_parser.fs,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unknown,
            // This will get updated when parsing starts
            size: 4096,
        };

        let err = reader.setup(Some(&ntfs_file));
        if err.is_err() {
            let issue = format!("Failed to setup outlook reader: {:?}", err.unwrap_err());
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
        reader.read_attachment(Some(&ntfs_file), &block_id, &descriptor_id)
    } else {
        let reader = match setup_outlook_reader(&path) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Failed to setup reader: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };
        let buf_reader = BufReader::new(reader);

        let mut reader = OutlookReader {
            fs: buf_reader,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unknown,
            // This will get updated when parsing starts
            size: 4096,
        };

        let err = reader.setup(None);
        if err.is_err() {
            let issue = format!("Failed to setup outlook reader: {:?}", err.unwrap_err());
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
        reader.read_attachment(None, &block_id, &descriptor_id)
    };

    let attachment = match attachment_result {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to read attachment: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&attachment).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

pub(crate) fn js_folder_meta(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, &0)?;
    let use_ntfs = boolean_arg(args, &1, context)?;
    let folder_id = bigint_arg(args, &2)? as u64;

    let folder_result = if use_ntfs {
        let mut ntfs_parser = match setup_ntfs_parser(&path.chars().next().unwrap_or('C')) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Failed to setup NTFS reader: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };
        let ntfs_file =
            match setup_outlook_reader_windows(&ntfs_parser.ntfs, &mut ntfs_parser.fs, &path) {
                Ok(result) => result,
                Err(err) => {
                    let issue = format!("Failed to setup NTFS outlook reader: {err:?}");
                    return Err(JsError::from_opaque(js_string!(issue).into()));
                }
            };

        let mut reader = OutlookReader {
            fs: ntfs_parser.fs,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unknown,
            // This will get updated when parsing starts
            size: 4096,
        };

        let err = reader.setup(Some(&ntfs_file));
        if err.is_err() {
            let issue = format!("Failed to setup outlook reader: {:?}", err.unwrap_err());
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
        reader.folder_metadata(Some(&ntfs_file), folder_id)
    } else {
        let reader = match setup_outlook_reader(&path) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Failed to setup reader: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };
        let buf_reader = BufReader::new(reader);

        let mut reader = OutlookReader {
            fs: buf_reader,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unknown,
            // This will get updated when parsing starts
            size: 4096,
        };

        let err = reader.setup(None);
        if err.is_err() {
            let issue = format!("Failed to setup outlook reader: {:?}", err.unwrap_err());
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
        reader.folder_metadata(None, folder_id)
    };

    let folder = match folder_result {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to read folder: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&folder).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

#[cfg(test)]
mod tests {
    use crate::{
        runtime::run::execute_script,
        structs::{artifacts::runtime::script::JSScript, toml::Output},
    };

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("json"),
            compress,
            timeline: false,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: None,
            filter_script: None,
            logging: None,
        }
    }

    #[test]
    fn test_get_outlook() {
        let test = "dmFyIEVycm9yQmFzZT1jbGFzcyBleHRlbmRzIEVycm9ye2NvbnN0cnVjdG9yKGUsbyl7c3VwZXIoKSx0aGlzLm5hbWU9ZSx0aGlzLm1lc3NhZ2U9b319LFdpbmRvd3NFcnJvcj1jbGFzcyBleHRlbmRzIEVycm9yQmFzZXt9LE91dGxvb2s9Y2xhc3N7Y29uc3RydWN0b3IoZSxvPSExKXt0aGlzLnBhdGg9ZSx0aGlzLnVzZV9udGZzPW99cm9vdEZvbGRlcigpe3RyeXtyZXR1cm4ganNfcm9vdF9mb2xkZXIodGhpcy5wYXRoLHRoaXMudXNlX250ZnMpfWNhdGNoKGUpe3JldHVybiBuZXcgV2luZG93c0Vycm9yKCJPVVRMT09LIixgZmFpbGVkIHRvIGRldGVybWluZSByb290IGZvbGRlciBmb3IgJHt0aGlzLnBhdGh9OiAke2V9YCl9fXJlYWRGb2xkZXIoZSl7dHJ5e3JldHVybiBqc19yZWFkX2ZvbGRlcih0aGlzLnBhdGgsdGhpcy51c2VfbnRmcyxlKX1jYXRjaChlKXtyZXR1cm4gbmV3IFdpbmRvd3NFcnJvcigiT1VUTE9PSyIsYGZhaWxlZCB0byByZWFkIGZvbGRlciBmb3IgJHt0aGlzLnBhdGh9OiAke2V9YCl9fXJlYWRNZXNzYWdlcyhlLG8sdD01MCl7Y29uc3Qgcz1bXTtmb3IobGV0IGU9bztlPHQrbztlKyspcy5wdXNoKGUpO2Uucm93cz1zO3RyeXtyZXR1cm4ganNfcmVhZF9tZXNzYWdlcyh0aGlzLnBhdGgsdGhpcy51c2VfbnRmcyxlLG8pfWNhdGNoKGUpe3JldHVybiBuZXcgV2luZG93c0Vycm9yKCJPVVRMT09LIixgZmFpbGVkIHRvIHJlYWQgZW1haWwgbWVzc2FnZSBmb3IgJHt0aGlzLnBhdGh9OiAke2V9YCl9fXJlYWRBdHRhY2htZW50KGUsbyl7dHJ5e3JldHVybiBqc19yZWFkX2F0dGFjaG1lbnQodGhpcy5wYXRoLHRoaXMudXNlX250ZnMsZSxvKX1jYXRjaChlKXtyZXR1cm4gbmV3IFdpbmRvd3NFcnJvcigiT1VUTE9PSyIsYGZhaWxlZCB0byByZWFkIGVtYWlsIGF0dGFjaG1lbnQgZm9yICR7dGhpcy5wYXRofTogJHtlfWApfX1mb2xkZXJNZXRhZGF0YShlKXt0cnl7cmV0dXJuIGpzX2ZvbGRlcl9tZXRhKHRoaXMucGF0aCx0aGlzLnVzZV9udGZzLGUpfWNhdGNoKGUpe3JldHVybiBuZXcgV2luZG93c0Vycm9yKCJPVVRMT09LIixgZmFpbGVkIHRvIHJlYWQgZm9sZGVyIG1ldGFkYXRhIGZvciAke3RoaXMucGF0aH06ICR7ZX1gKX19bWVzc2FnZVN0b3JlKCl7dHJ5e3JldHVybiBqc19tZXNzYWdlX3N0b3JlKHRoaXMucGF0aCx0aGlzLnVzZV9udGZzKX1jYXRjaChlKXtyZXR1cm4gbmV3IFdpbmRvd3NFcnJvcigiT1VUTE9PSyIsYGZhaWxlZCB0byBleHBvcnQgbWVzc2FnZSBzdG9yZSBmb3IgJHt0aGlzLnBhdGh9OiAke2V9YCl9fW5hbWVNYXBzKCl7dHJ5e3JldHVybiBqc19uYW1lX21hcCh0aGlzLnBhdGgsdGhpcy51c2VfbnRmcyl9Y2F0Y2goZSl7cmV0dXJuIG5ldyBXaW5kb3dzRXJyb3IoIk9VVExPT0siLGBmYWlsZWQgdG8gZ2V0IG5hbWUgbWFwcyBmb3IgJHt0aGlzLnBhdGh9OiAke2V9YCl9fX07ZnVuY3Rpb24gbWFpbigpe2NvbnN0IGU9bmV3IE91dGxvb2soIi4vdGVzdHMvdGVzdF9kYXRhL3dpbmRvd3Mvb3V0bG9vay93aW5kb3dzMTEvdGVzdEBvdXRsb29rLmNvbS5vc3QiKSxvPWUucm9vdEZvbGRlcigpO2lmKG8gaW5zdGFuY2VvZiBXaW5kb3dzRXJyb3IpcmV0dXJuIHZvaWQgY29uc29sZS5sb2cobyk7Y29uc3QgdD1lLm1lc3NhZ2VTdG9yZSgpO3QgaW5zdGFuY2VvZiBXaW5kb3dzRXJyb3J8fGNvbnNvbGUubG9nKGBNZXNzYWdlIHN0b3JlIGNvbnRhaW5zOiAke3QubGVuZ3RofSBlbnRyaWVzYCk7Y29uc3Qgcz1lLm5hbWVNYXBzKCk7cyBpbnN0YW5jZW9mIFdpbmRvd3NFcnJvcnx8Y29uc29sZS5sb2coYE5hbWUgbWFwIGNvbnRhaW5zOiAke09iamVjdC5rZXlzKHMpLmxlbmd0aH0gZW50cmllc2ApO2Zvcihjb25zdCB0IG9mIG8uc3ViZm9sZGVycyljb25zb2xlLmxvZyhgTmFtZTogJHt0Lm5hbWV9IC0gTm9kZTogJHt0Lm5vZGV9YCksd2Fsa0ZvbGRlcnModCxlLGAvJHt0Lm5hbWV9YCl9ZnVuY3Rpb24gd2Fsa0ZvbGRlcnMoZSxvLHQpe2NvbnN0IHM9by5yZWFkRm9sZGVyKGUubm9kZSk7aWYocyBpbnN0YW5jZW9mIFdpbmRvd3NFcnJvciljb25zb2xlLmxvZyhzKTtlbHNle2lmKDAhPXMubWVzc2FnZV9jb3VudCl7Y29uc29sZS5sb2coYFRvdGFsIG1lc3NhZ2VzOiAke3MubWVzc2FnZV9jb3VudH1gKTtsZXQgZT0yMDA7ZT5zLm1lc3NhZ2VfY291bnQmJihlPXMubWVzc2FnZV9jb3VudCk7bGV0IHQ9MCxyPXMubWVzc2FnZV9jb3VudDtmb3IoOzAhPXI7KXtjb25zdCBuPW8ucmVhZE1lc3NhZ2VzKHMubWVzc2FnZXNfdGFibGUsdCxlKTtpZihuIGluc3RhbmNlb2YgV2luZG93c0Vycm9yKXtjb25zb2xlLmxvZyhuKTticmVha31jb25zb2xlLmxvZyhgRW1haWwgbWVzc2FnZXM6ICR7bi5sZW5ndGh9YCk7Zm9yKGNvbnN0IGUgb2Ygbil7IkhpIj09PWUuc3ViamVjdCYmY29uc29sZS5sb2coZS5ib2R5KTtmb3IoY29uc3QgdCBvZiBlLmF0dGFjaG1lbnRzKXtjb25zb2xlLmxvZyhgQXR0YWNobWVudDogJHt0Lm5hbWV9YCk7Y29uc3QgZT1vLnJlYWRBdHRhY2htZW50KHQuYmxvY2tfaWQsdC5kZXNjcmlwdG9yX2lkKTtpZighKGUgaW5zdGFuY2VvZiBXaW5kb3dzRXJyb3IpKXtjb25zb2xlLmxvZyhKU09OLnN0cmluZ2lmeShlKSk7YnJlYWt9Y29uc29sZS5lcnJvcihlKX19aWYobi5sZW5ndGg8ZSlicmVhaztyPW4ubGVuZ3RoLHQrPWV9fWZvcihjb25zdCBlIG9mIHMuc3ViZm9sZGVycyl7Y29uc3Qgcz1gJHt0fS8ke2UubmFtZX1gO2NvbnNvbGUubG9nKGBOYW1lOiAke2UubmFtZX0gLSBOb2RlOiAke2Uubm9kZX0gLSBGb2xkZXIgcGF0aDogJHtzfWApLHdhbGtGb2xkZXJzKGUsbyxzKX19fW1haW4oKTs=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("outlook_js"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
