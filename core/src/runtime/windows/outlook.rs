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
        reader.root_folder(None)
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

pub(crate) fn js_read_folder(
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
                if chunks.len() < message_table.rows.len() {
                    continue;
                }
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
        let test = "Ly8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3V0aWxzL2Vycm9yLnRzCnZhciBFcnJvckJhc2UgPSBjbGFzcyBleHRlbmRzIEVycm9yIHsKICBjb25zdHJ1Y3RvcihuYW1lLCBtZXNzYWdlKSB7CiAgICBzdXBlcigpOwogICAgdGhpcy5uYW1lID0gbmFtZTsKICAgIHRoaXMubWVzc2FnZSA9IG1lc3NhZ2U7CiAgfQp9OwoKLy8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3dpbmRvd3MvZXJyb3JzLnRzCnZhciBXaW5kb3dzRXJyb3IgPSBjbGFzcyBleHRlbmRzIEVycm9yQmFzZSB7Cn07CgovLyAuLi8uLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvd2luZG93cy9vdXRsb29rLnRzCnZhciBPdXRsb29rID0gY2xhc3MgewogIGNvbnN0cnVjdG9yKHBhdGgsIG50ZnMgPSBmYWxzZSkgewogICAgdGhpcy5wYXRoID0gcGF0aDsKICAgIHRoaXMudXNlX250ZnMgPSBudGZzOwogIH0KICByb290Rm9sZGVyKCkgewogICAgdHJ5IHsKICAgICAgY29uc3QgZGF0YSA9IGpzX3Jvb3RfZm9sZGVyKAogICAgICAgIHRoaXMucGF0aCwKICAgICAgICB0aGlzLnVzZV9udGZzCiAgICAgICk7CiAgICAgIHJldHVybiBkYXRhOwogICAgfSBjYXRjaCAoZXJyKSB7CiAgICAgIHJldHVybiBuZXcgV2luZG93c0Vycm9yKAogICAgICAgICJPVVRMT09LIiwKICAgICAgICBgZmFpbGVkIHRvIGRldGVybWluZSByb290IGZvbGRlciBmb3IgJHt0aGlzLnBhdGh9OiAke2Vycn1gCiAgICAgICk7CiAgICB9CiAgfQogIHJlYWRGb2xkZXIoZm9sZGVyKSB7CiAgICB0cnkgewogICAgICBjb25zdCBkYXRhID0ganNfcmVhZF9mb2xkZXIoCiAgICAgICAgdGhpcy5wYXRoLAogICAgICAgIHRoaXMudXNlX250ZnMsCiAgICAgICAgZm9sZGVyCiAgICAgICk7CiAgICAgIHJldHVybiBkYXRhOwogICAgfSBjYXRjaCAoZXJyKSB7CiAgICAgIHJldHVybiBuZXcgV2luZG93c0Vycm9yKAogICAgICAgICJPVVRMT09LIiwKICAgICAgICBgZmFpbGVkIHRvIHJlYWQgZm9sZGVyIGZvciAke3RoaXMucGF0aH06ICR7ZXJyfWAKICAgICAgKTsKICAgIH0KICB9CiAgcmVhZE1lc3NhZ2VzKHRhYmxlLCBvZmZzZXQsIGxpbWl0ID0gNTApIHsKICAgIGNvbnN0IHJvd3MgPSBbXTsKICAgIGZvciAobGV0IGkgPSBvZmZzZXQ7IGkgPCBsaW1pdCArIG9mZnNldDsgaSsrKSB7CiAgICAgIHJvd3MucHVzaChpKTsKICAgIH0KICAgIHRhYmxlLnJvd3MgPSByb3dzOwogICAgdHJ5IHsKICAgICAgY29uc3QgZGF0YSA9IGpzX3JlYWRfbWVzc2FnZXMoCiAgICAgICAgdGhpcy5wYXRoLAogICAgICAgIHRoaXMudXNlX250ZnMsCiAgICAgICAgdGFibGUsCiAgICAgICAgb2Zmc2V0CiAgICAgICk7CiAgICAgIHJldHVybiBkYXRhOwogICAgfSBjYXRjaCAoZXJyKSB7CiAgICAgIHJldHVybiBuZXcgV2luZG93c0Vycm9yKAogICAgICAgICJPVVRMT09LIiwKICAgICAgICBgZmFpbGVkIHRvIHJlYWQgZW1haWwgbWVzc2FnZSBmb3IgJHt0aGlzLnBhdGh9OiAke2Vycn1gCiAgICAgICk7CiAgICB9CiAgfQogIHJlYWRBdHRhY2htZW50KGJsb2NrX2lkLCBkZXNjcmlwdG9yX2lkKSB7CiAgICB0cnkgewogICAgICBjb25zdCBkYXRhID0ganNfcmVhZF9hdHRhY2htZW50KAogICAgICAgIHRoaXMucGF0aCwKICAgICAgICB0aGlzLnVzZV9udGZzLAogICAgICAgIGJsb2NrX2lkLAogICAgICAgIGRlc2NyaXB0b3JfaWQKICAgICAgKTsKICAgICAgcmV0dXJuIGRhdGE7CiAgICB9IGNhdGNoIChlcnIpIHsKICAgICAgcmV0dXJuIG5ldyBXaW5kb3dzRXJyb3IoCiAgICAgICAgIk9VVExPT0siLAogICAgICAgIGBmYWlsZWQgdG8gcmVhZCBlbWFpbCBhdHRhY2htZW50IGZvciAke3RoaXMucGF0aH06ICR7ZXJyfWAKICAgICAgKTsKICAgIH0KICB9Cn07CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgcGF0aCA9ICIuL3Rlc3RzL3Rlc3RfZGF0YS93aW5kb3dzL291dGxvb2svd2luZG93czExL3Rlc3RAb3V0bG9vay5jb20ub3N0IjsKICBjb25zdCByZWFkZXIgPSBuZXcgT3V0bG9vayhwYXRoKTsKICBjb25zdCByZXN1bHQgPSByZWFkZXIucm9vdEZvbGRlcigpOwogIGlmIChyZXN1bHQgaW5zdGFuY2VvZiBXaW5kb3dzRXJyb3IpIHsKICAgIGNvbnNvbGUubG9nKHJlc3VsdCk7CiAgICByZXR1cm47CiAgfQogIGZvciAoY29uc3Qgc3ViIG9mIHJlc3VsdC5zdWJmb2xkZXJzKSB7CiAgICBjb25zb2xlLmxvZyhgTmFtZTogJHtzdWIubmFtZX0gLSBOb2RlOiAke3N1Yi5ub2RlfWApOwogICAgd2Fsa0ZvbGRlcnMoc3ViLCByZWFkZXIsIGAvJHtzdWIubmFtZX1gKTsKICB9Cn0KZnVuY3Rpb24gd2Fsa0ZvbGRlcnMoZm9sZGVyLCByZWFkZXIsIGZ1bGxfcGF0aCkgewogIGNvbnN0IHJlc3VsdCA9IHJlYWRlci5yZWFkRm9sZGVyKGZvbGRlci5ub2RlKTsKICBpZiAocmVzdWx0IGluc3RhbmNlb2YgV2luZG93c0Vycm9yKSB7CiAgICBjb25zb2xlLmxvZyhyZXN1bHQpOwogICAgcmV0dXJuOwogIH0KICBpZiAocmVzdWx0Lm1lc3NhZ2VfY291bnQgIT0gMCkgewogICAgY29uc29sZS5sb2coYFRvdGFsIG1lc3NhZ2VzOiAke3Jlc3VsdC5tZXNzYWdlX2NvdW50fWApOwogICAgbGV0IGxpbWl0ID0gMjAwOwogICAgaWYgKGxpbWl0ID4gcmVzdWx0Lm1lc3NhZ2VfY291bnQpIHsKICAgICAgbGltaXQgPSByZXN1bHQubWVzc2FnZV9jb3VudDsKICAgIH0KICAgIGxldCBvZmZzZXQgPSAwOwogICAgbGV0IGNvdW50ID0gcmVzdWx0Lm1lc3NhZ2VfY291bnQ7CiAgICB3aGlsZSAoY291bnQgIT0gMCkgewogICAgICBjb25zdCBlbWFpbHMgPSByZWFkZXIucmVhZE1lc3NhZ2VzKHJlc3VsdC5tZXNzYWdlc190YWJsZSwgb2Zmc2V0LCBsaW1pdCk7CiAgICAgIGlmIChlbWFpbHMgaW5zdGFuY2VvZiBXaW5kb3dzRXJyb3IpIHsKICAgICAgICBjb25zb2xlLmxvZyhlbWFpbHMpOwogICAgICAgIGJyZWFrOwogICAgICB9CiAgICAgIGNvbnNvbGUubG9nKGBFbWFpbCBtZXNzYWdlczogJHtlbWFpbHMubGVuZ3RofWApOwogICAgICBmb3IgKGNvbnN0IGVtYWlsIG9mIGVtYWlscykgewogICAgICAgIGlmIChlbWFpbC5zdWJqZWN0ID09PSAiSGkiKSB7CiAgICAgICAgICBjb25zb2xlLmxvZyhlbWFpbC5ib2R5KTsKICAgICAgICB9CiAgICAgICAgZm9yIChjb25zdCBhdHRhY2ggb2YgZW1haWwuYXR0YWNobWVudHMpIHsKICAgICAgICAgIGNvbnNvbGUubG9nKGBBdHRhY2htZW50OiAke2F0dGFjaC5uYW1lfWApOwogICAgICAgICAgY29uc3QgZGV0YWlscyA9IHJlYWRlci5yZWFkQXR0YWNobWVudChhdHRhY2guYmxvY2tfaWQsIGF0dGFjaC5kZXNjcmlwdG9yX2lkKTsKICAgICAgICAgIGlmIChkZXRhaWxzIGluc3RhbmNlb2YgV2luZG93c0Vycm9yKSB7CiAgICAgICAgICAgIGNvbnNvbGUuZXJyb3IoZGV0YWlscyk7CiAgICAgICAgICAgIGNvbnRpbnVlOwogICAgICAgICAgfQogICAgICAgICAgY29uc29sZS5sb2coZGV0YWlscyk7CiAgICAgICAgICBicmVhazsKICAgICAgICB9CiAgICAgIH0KICAgICAgaWYgKGVtYWlscy5sZW5ndGggPCBsaW1pdCkgewogICAgICAgIGJyZWFrOwogICAgICB9CiAgICAgIGNvdW50ID0gZW1haWxzLmxlbmd0aDsKICAgICAgb2Zmc2V0ICs9IGxpbWl0OwogICAgfQogIH0KICBmb3IgKGNvbnN0IHN1YiBvZiByZXN1bHQuc3ViZm9sZGVycykgewogICAgY29uc3QgcGF0aCA9IGAke2Z1bGxfcGF0aH0vJHtzdWIubmFtZX1gOwogICAgY29uc29sZS5sb2coYE5hbWU6ICR7c3ViLm5hbWV9IC0gTm9kZTogJHtzdWIubm9kZX0gLSBGb2xkZXIgcGF0aDogJHtwYXRofWApOwogICAgd2Fsa0ZvbGRlcnMoc3ViLCByZWFkZXIsIHBhdGgpOwogIH0KfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("outlook_js"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
