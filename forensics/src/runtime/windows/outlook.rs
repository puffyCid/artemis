use crate::{
    accessor::access::Accessor,
    artifacts::os::windows::outlook::{
        header::FormatType,
        helper::{OutlookReader, OutlookReaderAction},
        tables::context::TableInfo,
    },
    runtime::helper::{bigint_arg, string_arg, value_arg},
};
use boa_engine::{Context, JsError, JsResult, JsValue, js_string};
use tracing::error;

pub(crate) fn js_root_folder(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, 0)?;
    let fs = match Accessor::with_defaults().open_reader(&path) {
        Ok(results) => results,
        Err(err) => {
            let issue = format!("Failed to setup outlook reader: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let mut reader = OutlookReader {
        fs,
        block_btree: Vec::new(),
        node_btree: Vec::new(),
        format: FormatType::Unknown,
        // This will get updated when parsing starts
        size: 4096,
    };

    if let Err(result) = reader.setup() {
        let issue = format!("Failed to setup outlook reader: {result:?}");
        return Err(JsError::from_opaque(js_string!(issue).into()));
    }

    let root = match reader.root_folder() {
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
    let path = string_arg(args, 0)?;

    let fs = match Accessor::with_defaults().open_reader(&path) {
        Ok(results) => results,
        Err(err) => {
            let issue = format!("Failed to setup outlook reader: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let mut reader = OutlookReader {
        fs,
        block_btree: Vec::new(),
        node_btree: Vec::new(),
        format: FormatType::Unknown,
        // This will get updated when parsing starts
        size: 4096,
    };

    if let Err(result) = reader.setup() {
        let issue = format!("Failed to setup outlook reader: {result:?}");
        return Err(JsError::from_opaque(js_string!(issue).into()));
    }

    let store = match reader.message_store() {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to read message store: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&store).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

pub(crate) fn js_name_map(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, 0)?;

    let fs = match Accessor::with_defaults().open_reader(&path) {
        Ok(results) => results,
        Err(err) => {
            let issue = format!("Failed to setup outlook reader: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let mut reader = OutlookReader {
        fs,
        block_btree: Vec::new(),
        node_btree: Vec::new(),
        format: FormatType::Unknown,
        // This will get updated when parsing starts
        size: 4096,
    };

    if let Err(result) = reader.setup() {
        let issue = format!("Failed to setup outlook reader: {result:?}");
        return Err(JsError::from_opaque(js_string!(issue).into()));
    }

    let map = match reader.name_id_map() {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to read name id map: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&map).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

pub(crate) fn js_read_folder(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, 0)?;
    let folder_id = bigint_arg(args, 1)? as u64;

    let fs = match Accessor::with_defaults().open_reader(&path) {
        Ok(results) => results,
        Err(err) => {
            let issue = format!("Failed to setup outlook reader: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let mut reader = OutlookReader {
        fs,
        block_btree: Vec::new(),
        node_btree: Vec::new(),
        format: FormatType::Unknown,
        // This will get updated when parsing starts
        size: 4096,
    };

    if let Err(result) = reader.setup() {
        let issue = format!("Failed to setup outlook reader: {result:?}");
        return Err(JsError::from_opaque(js_string!(issue).into()));
    }

    let folder = match reader.read_folder(folder_id) {
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
    let path = string_arg(args, 0)?;
    let table = value_arg(args, 1, context)?;
    let offset = bigint_arg(args, 2)? as u64;
    let message_table: TableInfo = match serde_json::from_value(table) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to deserialize TableInfo: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let fs = match Accessor::with_defaults().open_reader(&path) {
        Ok(results) => results,
        Err(err) => {
            let issue = format!("Failed to setup outlook reader: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let mut reader = OutlookReader {
        fs,
        block_btree: Vec::new(),
        node_btree: Vec::new(),
        format: FormatType::Unknown,
        // This will get updated when parsing starts
        size: 4096,
    };

    if let Err(result) = reader.setup() {
        let issue = format!("Failed to setup outlook reader: {result:?}");
        return Err(JsError::from_opaque(js_string!(issue).into()));
    }

    // This is difficult
    let message = if let Some(has_branch) = &message_table.has_branch {
        let mut main_count = 0;
        let mut chunks = Vec::new();
        // Each branch has a collection of messages. Ex: Messages 0-20
        for branch in has_branch {
            // If the offset is greater than the current branch message count.
            // Go to next branch. Ex: Branch 1 has messages 0-20. Branch 2 has messages 21-40, etc
            if offset > branch.rows_info.count + main_count {
                main_count += branch.rows_info.count;
                continue;
            }

            let mut emails = match reader.read_message(&message_table, None) {
                Ok(result) => result,
                Err(err) => {
                    error!("Failed to read message {err:?}");
                    continue;
                }
            };
            chunks.append(&mut emails);
        }
        chunks
    } else {
        match reader.read_message(&message_table, None) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Failed to read messages: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        }
    };

    let results = serde_json::to_value(&message).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

pub(crate) fn js_read_attachment(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, 0)?;
    let block_id = bigint_arg(args, 1)? as u64;
    let descriptor_id = bigint_arg(args, 2)? as u64;

    let fs = match Accessor::with_defaults().open_reader(&path) {
        Ok(results) => results,
        Err(err) => {
            let issue = format!("Failed to setup outlook reader: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let mut reader = OutlookReader {
        fs,
        block_btree: Vec::new(),
        node_btree: Vec::new(),
        format: FormatType::Unknown,
        // This will get updated when parsing starts
        size: 4096,
    };

    if let Err(result) = reader.setup() {
        let issue = format!("Failed to setup outlook reader: {result:?}");
        return Err(JsError::from_opaque(js_string!(issue).into()));
    }

    let attachment = match reader.read_attachment(block_id, descriptor_id) {
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
    let path = string_arg(args, 0)?;
    let folder_id = bigint_arg(args, 1)? as u64;

    let fs = match Accessor::with_defaults().open_reader(&path) {
        Ok(results) => results,
        Err(err) => {
            let issue = format!("Failed to setup outlook reader: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let mut reader = OutlookReader {
        fs,
        block_btree: Vec::new(),
        node_btree: Vec::new(),
        format: FormatType::Unknown,
        // This will get updated when parsing starts
        size: 4096,
    };

    if let Err(result) = reader.setup() {
        let issue = format!("Failed to setup outlook reader: {result:?}");
        return Err(JsError::from_opaque(js_string!(issue).into()));
    }

    let folder = match reader.folder_metadata(folder_id) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to read attachment: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&folder).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

#[cfg(test)]
mod tests {
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use crate::{
        output::manager::OutputManager, runtime::run::execute_script,
        structs::artifacts::runtime::script::JSScript,
    };
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
    fn test_get_outlook() {
        let test = "dmFyIEVycm9yQmFzZT1jbGFzcyBleHRlbmRzIEVycm9ye2NvbnN0cnVjdG9yKGUsbyl7c3VwZXIoKSx0aGlzLm5hbWU9ZSx0aGlzLm1lc3NhZ2U9b319LFdpbmRvd3NFcnJvcj1jbGFzcyBleHRlbmRzIEVycm9yQmFzZXt9LE91dGxvb2s9Y2xhc3N7Y29uc3RydWN0b3IoZSxvPSExKXt0aGlzLnBhdGg9ZX1yb290Rm9sZGVyKCl7dHJ5e3JldHVybiBqc19yb290X2ZvbGRlcih0aGlzLnBhdGgpfWNhdGNoKGUpe3JldHVybiBuZXcgV2luZG93c0Vycm9yKCJPVVRMT09LIixgZmFpbGVkIHRvIGRldGVybWluZSByb290IGZvbGRlciBmb3IgJHt0aGlzLnBhdGh9OiAke2V9YCl9fXJlYWRGb2xkZXIoZSl7dHJ5e3JldHVybiBqc19yZWFkX2ZvbGRlcih0aGlzLnBhdGgsZSl9Y2F0Y2goZSl7cmV0dXJuIG5ldyBXaW5kb3dzRXJyb3IoIk9VVExPT0siLGBmYWlsZWQgdG8gcmVhZCBmb2xkZXIgZm9yICR7dGhpcy5wYXRofTogJHtlfWApfX1yZWFkTWVzc2FnZXMoZSxvLHQ9NTApe2NvbnN0IHI9W107Zm9yKGxldCBlPW87ZTx0K287ZSsrKXIucHVzaChlKTtlLnJvd3M9cjt0cnl7cmV0dXJuIGpzX3JlYWRfbWVzc2FnZXModGhpcy5wYXRoLGUsbyl9Y2F0Y2goZSl7cmV0dXJuIG5ldyBXaW5kb3dzRXJyb3IoIk9VVExPT0siLGBmYWlsZWQgdG8gcmVhZCBlbWFpbCBtZXNzYWdlIGZvciAke3RoaXMucGF0aH06ICR7ZX1gKX19cmVhZEF0dGFjaG1lbnQoZSxvKXt0cnl7cmV0dXJuIGpzX3JlYWRfYXR0YWNobWVudCh0aGlzLnBhdGgsZSxvKX1jYXRjaChlKXtyZXR1cm4gbmV3IFdpbmRvd3NFcnJvcigiT1VUTE9PSyIsYGZhaWxlZCB0byByZWFkIGVtYWlsIGF0dGFjaG1lbnQgZm9yICR7dGhpcy5wYXRofTogJHtlfWApfX1mb2xkZXJNZXRhZGF0YShlKXt0cnl7cmV0dXJuIGpzX2ZvbGRlcl9tZXRhKHRoaXMucGF0aCxlKX1jYXRjaChlKXtyZXR1cm4gbmV3IFdpbmRvd3NFcnJvcigiT1VUTE9PSyIsYGZhaWxlZCB0byByZWFkIGZvbGRlciBtZXRhZGF0YSBmb3IgJHt0aGlzLnBhdGh9OiAke2V9YCl9fW1lc3NhZ2VTdG9yZSgpe3RyeXtyZXR1cm4ganNfbWVzc2FnZV9zdG9yZSh0aGlzLnBhdGgpfWNhdGNoKGUpe3JldHVybiBuZXcgV2luZG93c0Vycm9yKCJPVVRMT09LIixgZmFpbGVkIHRvIGV4cG9ydCBtZXNzYWdlIHN0b3JlIGZvciAke3RoaXMucGF0aH06ICR7ZX1gKX19bmFtZU1hcHMoKXt0cnl7cmV0dXJuIGpzX25hbWVfbWFwKHRoaXMucGF0aCl9Y2F0Y2goZSl7cmV0dXJuIG5ldyBXaW5kb3dzRXJyb3IoIk9VVExPT0siLGBmYWlsZWQgdG8gZ2V0IG5hbWUgbWFwcyBmb3IgJHt0aGlzLnBhdGh9OiAke2V9YCl9fX07ZnVuY3Rpb24gbWFpbigpe2NvbnN0IGU9bmV3IE91dGxvb2soIi4vdGVzdHMvdGVzdF9kYXRhL3dpbmRvd3Mvb3V0bG9vay93aW5kb3dzMTEvdGVzdEBvdXRsb29rLmNvbS5vc3QiKSxvPWUucm9vdEZvbGRlcigpO2lmKG8gaW5zdGFuY2VvZiBXaW5kb3dzRXJyb3IpcmV0dXJuIHZvaWQgY29uc29sZS5sb2cobyk7Y29uc3QgdD1lLm1lc3NhZ2VTdG9yZSgpO3QgaW5zdGFuY2VvZiBXaW5kb3dzRXJyb3J8fGNvbnNvbGUubG9nKGBNZXNzYWdlIHN0b3JlIGNvbnRhaW5zOiAke3QubGVuZ3RofSBlbnRyaWVzYCk7Y29uc3Qgcj1lLm5hbWVNYXBzKCk7ciBpbnN0YW5jZW9mIFdpbmRvd3NFcnJvcnx8Y29uc29sZS5sb2coYE5hbWUgbWFwIGNvbnRhaW5zOiAke09iamVjdC5rZXlzKHIpLmxlbmd0aH0gZW50cmllc2ApO2Zvcihjb25zdCB0IG9mIG8uc3ViZm9sZGVycyljb25zb2xlLmxvZyhgTmFtZTogJHt0Lm5hbWV9IC0gTm9kZTogJHt0Lm5vZGV9YCksd2Fsa0ZvbGRlcnModCxlLGAvJHt0Lm5hbWV9YCl9ZnVuY3Rpb24gd2Fsa0ZvbGRlcnMoZSxvLHQpe2NvbnN0IHI9by5yZWFkRm9sZGVyKGUubm9kZSk7aWYociBpbnN0YW5jZW9mIFdpbmRvd3NFcnJvciljb25zb2xlLmxvZyhyKTtlbHNle2lmKDAhPXIubWVzc2FnZV9jb3VudCl7Y29uc29sZS5sb2coYFRvdGFsIG1lc3NhZ2VzOiAke3IubWVzc2FnZV9jb3VudH1gKTtsZXQgZT0yMDA7ZT5yLm1lc3NhZ2VfY291bnQmJihlPXIubWVzc2FnZV9jb3VudCk7bGV0IHQ9MCxzPXIubWVzc2FnZV9jb3VudDtmb3IoOzAhPXM7KXtjb25zdCBhPW8ucmVhZE1lc3NhZ2VzKHIubWVzc2FnZXNfdGFibGUsdCxlKTtpZihhIGluc3RhbmNlb2YgV2luZG93c0Vycm9yKXtjb25zb2xlLmxvZyhhKTticmVha31jb25zb2xlLmxvZyhgRW1haWwgbWVzc2FnZXM6ICR7YS5sZW5ndGh9YCk7Zm9yKGNvbnN0IGUgb2YgYSl7IkhpIj09PWUuc3ViamVjdCYmY29uc29sZS5sb2coZS5ib2R5KTtmb3IoY29uc3QgdCBvZiBlLmF0dGFjaG1lbnRzKXtjb25zb2xlLmxvZyhgQXR0YWNobWVudDogJHt0Lm5hbWV9YCk7Y29uc3QgZT1vLnJlYWRBdHRhY2htZW50KHQuYmxvY2tfaWQsdC5kZXNjcmlwdG9yX2lkKTtpZighKGUgaW5zdGFuY2VvZiBXaW5kb3dzRXJyb3IpKXtjb25zb2xlLmxvZyhKU09OLnN0cmluZ2lmeShlKSk7YnJlYWt9Y29uc29sZS5lcnJvcihlKX19aWYoYS5sZW5ndGg8ZSlicmVhaztzPWEubGVuZ3RoLHQrPWV9fWZvcihjb25zdCBlIG9mIHIuc3ViZm9sZGVycyl7Y29uc3Qgcj1gJHt0fS8ke2UubmFtZX1gO2NvbnNvbGUubG9nKGBOYW1lOiAke2UubmFtZX0gLSBOb2RlOiAke2Uubm9kZX0gLSBGb2xkZXIgcGF0aDogJHtyfWApLHdhbGtGb2xkZXJzKGUsbyxyKX19fW1haW4oKTs=";
        let mut output = output_options("runtime_test", "./tmp", false);
        let script = JSScript {
            name: String::from("outlook_js"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
