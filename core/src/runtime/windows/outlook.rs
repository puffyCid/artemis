use crate::{
    artifacts::os::windows::outlook::{
        header::FormatType,
        helper::{OutlookReader, OutlookReaderAction},
        reader::{setup_outlook_reader, setup_outlook_reader_windows},
        tables::context::TableInfo,
    },
    filesystem::ntfs::setup::setup_ntfs_parser,
};
use deno_core::{error::AnyError, op2};
use std::io::BufReader;

#[op2]
#[string]
pub(crate) fn get_root_folder(#[string] path: String, use_ntfs: bool) -> Result<String, AnyError> {
    let root = if use_ntfs {
        let mut ntfs_parser = setup_ntfs_parser(&path.chars().next().unwrap_or('C'))?;
        let ntfs_file =
            setup_outlook_reader_windows(&ntfs_parser.ntfs, &mut ntfs_parser.fs, &path)?;

        let mut reader = OutlookReader {
            fs: ntfs_parser.fs,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unknown,
            // This will get updated when parsing starts
            size: 4096,
        };

        reader.setup(Some(&ntfs_file))?;
        reader.root_folder(Some(&ntfs_file))?
    } else {
        let reader = setup_outlook_reader(&path)?;
        let buf_reader = BufReader::new(reader);

        let mut reader = OutlookReader {
            fs: buf_reader,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unknown,
            // This will get updated when parsing starts
            size: 4096,
        };

        reader.setup(None)?;
        reader.root_folder(None)?
    };

    let results = serde_json::to_string(&root)?;
    Ok(results)
}

#[op2]
#[string]
pub(crate) fn read_folder(
    #[string] path: String,
    use_ntfs: bool,
    #[bigint] folder: u64,
) -> Result<String, AnyError> {
    let folder = if use_ntfs {
        let mut ntfs_parser = setup_ntfs_parser(&path.chars().next().unwrap_or('C'))?;
        let ntfs_file =
            setup_outlook_reader_windows(&ntfs_parser.ntfs, &mut ntfs_parser.fs, &path)?;

        let mut reader = OutlookReader {
            fs: ntfs_parser.fs,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unknown,
            // This will get updated when parsing starts
            size: 4096,
        };

        reader.setup(Some(&ntfs_file))?;
        reader.read_folder(Some(&ntfs_file), folder)?
    } else {
        let reader = setup_outlook_reader(&path)?;
        let buf_reader = BufReader::new(reader);

        let mut reader = OutlookReader {
            fs: buf_reader,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unknown,
            // This will get updated when parsing starts
            size: 4096,
        };

        reader.setup(None)?;
        reader.read_folder(None, folder)?
    };

    let results = serde_json::to_string(&folder)?;
    Ok(results)
}

#[op2]
#[string]
pub(crate) fn read_messages(
    #[string] path: String,
    use_ntfs: bool,
    #[string] table: String,
    #[bigint] offset: u64,
) -> Result<String, AnyError> {
    let message_table: TableInfo = serde_json::from_str(&table)?;

    let messages = if use_ntfs {
        let mut ntfs_parser = setup_ntfs_parser(&path.chars().next().unwrap_or('C'))?;
        let ntfs_file =
            setup_outlook_reader_windows(&ntfs_parser.ntfs, &mut ntfs_parser.fs, &path)?;

        let mut reader = OutlookReader {
            fs: ntfs_parser.fs,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unknown,
            // This will get updated when parsing starts
            size: 4096,
        };

        reader.setup(Some(&ntfs_file))?;
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

                let mut emails = reader.read_message(Some(&ntfs_file), &message_table, None)?;
                chunks.append(&mut emails);
                if chunks.len() < message_table.rows.len() {
                    continue;
                }
            }
            chunks
        } else {
            reader.read_message(Some(&ntfs_file), &message_table, None)?
        }
    } else {
        let reader = setup_outlook_reader(&path)?;
        let buf_reader = BufReader::new(reader);

        let mut reader = OutlookReader {
            fs: buf_reader,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unknown,
            // This will get updated when parsing starts
            size: 4096,
        };

        reader.setup(None)?;
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

                let mut emails = reader.read_message(None, &message_table, None)?;
                chunks.append(&mut emails);
                if chunks.len() < message_table.rows.len() {
                    continue;
                }
                break;
            }
            chunks
        } else {
            reader.read_message(None, &message_table, None)?
        }
    };

    let results = serde_json::to_string(&messages)?;
    Ok(results)
}

#[op2]
#[string]
pub(crate) fn read_attachment(
    #[string] path: String,
    use_ntfs: bool,
    #[bigint] block_id: u64,
    #[bigint] descriptor_id: u64,
) -> Result<String, AnyError> {
    let attachment = if use_ntfs {
        let mut ntfs_parser = setup_ntfs_parser(&path.chars().next().unwrap_or('C'))?;
        let ntfs_file =
            setup_outlook_reader_windows(&ntfs_parser.ntfs, &mut ntfs_parser.fs, &path)?;

        let mut reader = OutlookReader {
            fs: ntfs_parser.fs,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unknown,
            // This will get updated when parsing starts
            size: 4096,
        };

        reader.setup(Some(&ntfs_file))?;
        reader.read_attachment(Some(&ntfs_file), &block_id, &descriptor_id)?
    } else {
        let reader = setup_outlook_reader(&path)?;
        let buf_reader = BufReader::new(reader);

        let mut reader = OutlookReader {
            fs: buf_reader,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unknown,
            // This will get updated when parsing starts
            size: 4096,
        };

        reader.setup(None)?;
        reader.read_attachment(None, &block_id, &descriptor_id)?
    };

    let results = serde_json::to_string(&attachment)?;
    Ok(results)
}

#[cfg(test)]
mod tests {
    use crate::{
        runtime::deno::execute_script, structs::artifacts::runtime::script::JSScript,
        structs::toml::Output,
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
        let test = "Ly8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3V0aWxzL2Vycm9yLnRzDQp2YXIgRXJyb3JCYXNlID0gY2xhc3MgZXh0ZW5kcyBFcnJvciB7DQogIGNvbnN0cnVjdG9yKG5hbWUsIG1lc3NhZ2UpIHsNCiAgICBzdXBlcigpOw0KICAgIHRoaXMubmFtZSA9IG5hbWU7DQogICAgdGhpcy5tZXNzYWdlID0gbWVzc2FnZTsNCiAgfQ0KfTsNCg0KLy8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3dpbmRvd3MvZXJyb3JzLnRzDQp2YXIgV2luZG93c0Vycm9yID0gY2xhc3MgZXh0ZW5kcyBFcnJvckJhc2Ugew0KfTsNCg0KLy8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3dpbmRvd3Mvb3V0bG9vay50cw0KdmFyIE91dGxvb2sgPSBjbGFzcyB7DQogIGNvbnN0cnVjdG9yKHBhdGgsIG50ZnMgPSBmYWxzZSkgew0KICAgIHRoaXMucGF0aCA9IHBhdGg7DQogICAgdGhpcy51c2VfbnRmcyA9IG50ZnM7DQogIH0NCiAgcm9vdEZvbGRlcigpIHsNCiAgICB0cnkgew0KICAgICAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X3Jvb3RfZm9sZGVyKA0KICAgICAgICB0aGlzLnBhdGgsDQogICAgICAgIHRoaXMudXNlX250ZnMNCiAgICAgICk7DQogICAgICBjb25zdCByZXN1bHRzID0gSlNPTi5wYXJzZShkYXRhKTsNCiAgICAgIHJldHVybiByZXN1bHRzOw0KICAgIH0gY2F0Y2ggKGVycikgew0KICAgICAgcmV0dXJuIG5ldyBXaW5kb3dzRXJyb3IoDQogICAgICAgICJPVVRMT09LIiwNCiAgICAgICAgYGZhaWxlZCB0byBkZXRlcm1pbmUgcm9vdCBmb2xkZXIgZm9yICR7dGhpcy5wYXRofTogJHtlcnJ9YA0KICAgICAgKTsNCiAgICB9DQogIH0NCiAgcmVhZEZvbGRlcihmb2xkZXIpIHsNCiAgICB0cnkgew0KICAgICAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMucmVhZF9mb2xkZXIoDQogICAgICAgIHRoaXMucGF0aCwNCiAgICAgICAgdGhpcy51c2VfbnRmcywNCiAgICAgICAgZm9sZGVyDQogICAgICApOw0KICAgICAgY29uc3QgcmVzdWx0cyA9IEpTT04ucGFyc2UoZGF0YSk7DQogICAgICByZXR1cm4gcmVzdWx0czsNCiAgICB9IGNhdGNoIChlcnIpIHsNCiAgICAgIHJldHVybiBuZXcgV2luZG93c0Vycm9yKA0KICAgICAgICAiT1VUTE9PSyIsDQogICAgICAgIGBmYWlsZWQgdG8gcmVhZCBmb2xkZXIgZm9yICR7dGhpcy5wYXRofTogJHtlcnJ9YA0KICAgICAgKTsNCiAgICB9DQogIH0NCiAgcmVhZE1lc3NhZ2VzKHRhYmxlLCBvZmZzZXQsIGxpbWl0ID0gNTApIHsNCiAgICBjb25zdCByb3dzID0gW107DQogICAgZm9yIChsZXQgaSA9IG9mZnNldDsgaSA8IGxpbWl0ICsgb2Zmc2V0OyBpKyspIHsNCiAgICAgIHJvd3MucHVzaChpKTsNCiAgICB9DQogICAgdGFibGUucm93cyA9IHJvd3M7DQogICAgdHJ5IHsNCiAgICAgIGNvbnN0IGRhdGEgPSBEZW5vLmNvcmUub3BzLnJlYWRfbWVzc2FnZXMoDQogICAgICAgIHRoaXMucGF0aCwNCiAgICAgICAgdGhpcy51c2VfbnRmcywNCiAgICAgICAgSlNPTi5zdHJpbmdpZnkodGFibGUpLA0KICAgICAgICBvZmZzZXQNCiAgICAgICk7DQogICAgICBjb25zdCByZXN1bHRzID0gSlNPTi5wYXJzZShkYXRhKTsNCiAgICAgIHJldHVybiByZXN1bHRzOw0KICAgIH0gY2F0Y2ggKGVycikgew0KICAgICAgcmV0dXJuIG5ldyBXaW5kb3dzRXJyb3IoDQogICAgICAgICJPVVRMT09LIiwNCiAgICAgICAgYGZhaWxlZCB0byByZWFkIGVtYWlsIG1lc3NhZ2UgZm9yICR7dGhpcy5wYXRofTogJHtlcnJ9YA0KICAgICAgKTsNCiAgICB9DQogIH0NCiAgcmVhZEF0dGFjaG1lbnQoYmxvY2tfaWQsIGRlc2NyaXB0b3JfaWQpIHsNCiAgICB0cnkgew0KICAgICAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMucmVhZF9hdHRhY2htZW50KA0KICAgICAgICB0aGlzLnBhdGgsDQogICAgICAgIHRoaXMudXNlX250ZnMsDQogICAgICAgIGJsb2NrX2lkLA0KICAgICAgICBkZXNjcmlwdG9yX2lkDQogICAgICApOw0KICAgICAgY29uc3QgcmVzdWx0cyA9IEpTT04ucGFyc2UoZGF0YSk7DQogICAgICByZXR1cm4gcmVzdWx0czsNCiAgICB9IGNhdGNoIChlcnIpIHsNCiAgICAgIHJldHVybiBuZXcgV2luZG93c0Vycm9yKA0KICAgICAgICAiT1VUTE9PSyIsDQogICAgICAgIGBmYWlsZWQgdG8gcmVhZCBlbWFpbCBhdHRhY2htZW50IGZvciAke3RoaXMucGF0aH06ICR7ZXJyfWANCiAgICAgICk7DQogICAgfQ0KICB9DQp9Ow0KDQovLyBtYWluLnRzDQpmdW5jdGlvbiBtYWluKCkgew0KICBjb25zdCBwYXRoID0gIi4vdGVzdHMvdGVzdF9kYXRhL3dpbmRvd3Mvb3V0bG9vay93aW5kb3dzMTEvdGVzdEBvdXRsb29rLmNvbS5vc3QiOw0KICBjb25zdCByZWFkZXIgPSBuZXcgT3V0bG9vayhwYXRoKTsNCiAgY29uc3QgcmVzdWx0ID0gcmVhZGVyLnJvb3RGb2xkZXIoKTsNCiAgaWYgKHJlc3VsdCBpbnN0YW5jZW9mIFdpbmRvd3NFcnJvcikgew0KICAgIGNvbnNvbGUubG9nKHJlc3VsdCk7DQogICAgcmV0dXJuOw0KICB9DQogIGZvciAoY29uc3Qgc3ViIG9mIHJlc3VsdC5zdWJmb2xkZXJzKSB7DQogICAgY29uc29sZS5sb2coYE5hbWU6ICR7c3ViLm5hbWV9IC0gTm9kZTogJHtzdWIubm9kZX1gKTsNCiAgICB3YWxrRm9sZGVycyhzdWIsIHJlYWRlciwgYC8ke3N1Yi5uYW1lfWApOw0KICB9DQp9DQpmdW5jdGlvbiB3YWxrRm9sZGVycyhmb2xkZXIsIHJlYWRlciwgZnVsbF9wYXRoKSB7DQogIGNvbnN0IHJlc3VsdCA9IHJlYWRlci5yZWFkRm9sZGVyKGZvbGRlci5ub2RlKTsNCiAgaWYgKHJlc3VsdCBpbnN0YW5jZW9mIFdpbmRvd3NFcnJvcikgew0KICAgIGNvbnNvbGUubG9nKHJlc3VsdCk7DQogICAgcmV0dXJuOw0KICB9DQogIGlmIChyZXN1bHQubWVzc2FnZV9jb3VudCAhPSAwKSB7DQogICAgY29uc29sZS5sb2coYFRvdGFsIG1lc3NhZ2VzOiAke3Jlc3VsdC5tZXNzYWdlX2NvdW50fWApOw0KICAgIGxldCBsaW1pdCA9IDIwMDsNCiAgICBpZiAobGltaXQgPiByZXN1bHQubWVzc2FnZV9jb3VudCkgew0KICAgICAgbGltaXQgPSByZXN1bHQubWVzc2FnZV9jb3VudDsNCiAgICB9DQogICAgbGV0IG9mZnNldCA9IDA7DQogICAgbGV0IGNvdW50ID0gcmVzdWx0Lm1lc3NhZ2VfY291bnQ7DQogICAgd2hpbGUgKGNvdW50ICE9IDApIHsNCiAgICAgIGNvbnN0IGVtYWlscyA9IHJlYWRlci5yZWFkTWVzc2FnZXMocmVzdWx0Lm1lc3NhZ2VzX3RhYmxlLCBvZmZzZXQsIGxpbWl0KTsNCiAgICAgIGlmIChlbWFpbHMgaW5zdGFuY2VvZiBXaW5kb3dzRXJyb3IpIHsNCiAgICAgICAgY29uc29sZS5sb2coZW1haWxzKTsNCiAgICAgICAgYnJlYWs7DQogICAgICB9DQogICAgICBjb25zb2xlLmxvZyhgRW1haWwgbWVzc2FnZXM6ICR7ZW1haWxzLmxlbmd0aH1gKTsNCiAgICAgIGZvciAoY29uc3QgZW1haWwgb2YgZW1haWxzKSB7DQogICAgICAgIGlmIChlbWFpbC5zdWJqZWN0ID09PSAiSGkiKSB7DQogICAgICAgICAgY29uc29sZS5sb2coZW1haWwuYm9keSk7DQogICAgICAgIH0NCiAgICAgICAgZm9yIChjb25zdCBhdHRhY2ggb2YgZW1haWwuYXR0YWNobWVudHMpIHsNCiAgICAgICAgICBjb25zb2xlLmxvZyhgQXR0YWNobWVudDogJHthdHRhY2gubmFtZX1gKTsNCiAgICAgICAgICBjb25zdCBkZXRhaWxzID0gcmVhZGVyLnJlYWRBdHRhY2htZW50KGF0dGFjaC5ibG9ja19pZCwgYXR0YWNoLmRlc2NyaXB0b3JfaWQpOw0KICAgICAgICAgIGlmIChkZXRhaWxzIGluc3RhbmNlb2YgV2luZG93c0Vycm9yKSB7DQogICAgICAgICAgICBjb25zb2xlLmVycm9yKGRldGFpbHMpOw0KICAgICAgICAgICAgY29udGludWU7DQogICAgICAgICAgfQ0KICAgICAgICAgIGNvbnNvbGUubG9nKGRldGFpbHMpOw0KICAgICAgICAgIGJyZWFrOw0KICAgICAgICB9DQogICAgICB9DQogICAgICBpZiAoZW1haWxzLmxlbmd0aCA8IGxpbWl0KSB7DQogICAgICAgIGJyZWFrOw0KICAgICAgfQ0KICAgICAgY291bnQgPSBlbWFpbHMubGVuZ3RoOw0KICAgICAgb2Zmc2V0ICs9IGxpbWl0Ow0KICAgIH0NCiAgfQ0KICBmb3IgKGNvbnN0IHN1YiBvZiByZXN1bHQuc3ViZm9sZGVycykgew0KICAgIGNvbnN0IHBhdGggPSBgJHtmdWxsX3BhdGh9LyR7c3ViLm5hbWV9YDsNCiAgICBjb25zb2xlLmxvZyhgTmFtZTogJHtzdWIubmFtZX0gLSBOb2RlOiAke3N1Yi5ub2RlfSAtIEZvbGRlciBwYXRoOiAke3BhdGh9YCk7DQogICAgd2Fsa0ZvbGRlcnMoc3ViLCByZWFkZXIsIHBhdGgpOw0KICB9DQp9DQptYWluKCk7DQo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("outlook_js"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
